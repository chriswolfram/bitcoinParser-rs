mod script;
mod basic_reading;

use chrono::prelude::*;
use rayon::prelude::*;
use std::fs;
use std::io;
use std::io::{BufReader, Read};
use std::path::PathBuf;
use sha2::{Sha256, Digest};
use script::BitcoinScript;
use basic_reading::*;

#[derive(Debug)]
pub struct BitcoinTransactionInput {
    pub prev_transaction: [u8; 32],
    pub prev_transaction_output: u32,
    pub script: Result<BitcoinScript, script::BitcoinScriptParseError>,
}

#[derive(Debug)]
pub struct BitcoinTransactionOutput {
    pub value: u64,
    pub script: Result<BitcoinScript, script::BitcoinScriptParseError>,
}

#[derive(Debug)]
pub struct BitcoinTransaction {
    pub inputs: Vec<BitcoinTransactionInput>,
    pub outputs: Vec<BitcoinTransactionOutput>,
    pub lock_time: u32,
    pub timestamp: DateTime<Utc>,
    pub is_coinbase: bool,
    pub hash: [u8; 32]
}

impl BitcoinTransaction {
    pub fn value(self: &BitcoinTransaction) -> u64 {
        self.outputs.iter().map(|o| o.value).sum()
    }
}

#[derive(Debug)]
pub struct BitcoinBlock {
    pub version: u32,
    pub prev_hash: [u8; 32],
    pub merkle_root: [u8; 32],
    pub timestamp: DateTime<Utc>,
    pub nonce: u32,
    pub transactions: Vec<BitcoinTransaction>,
}

#[derive(Debug)]
pub struct BlockFileIterator<T: Read> {
    reader: T,
}

#[derive(Debug)]
pub struct BlockCollection {
    pub base_dir: std::path::PathBuf,
}

impl<T: Read> BlockFileIterator<T> {
    pub fn new(reader: T) -> BlockFileIterator<T> {
        BlockFileIterator { reader }
    }
}

impl<T: Read> Iterator for BlockFileIterator<T> {
    type Item = BitcoinBlock;

    fn next(self: &mut BlockFileIterator<T>) -> Option<BitcoinBlock> {
        read_block(&mut self.reader).ok()
    }
}

impl BlockCollection {
    pub fn new(base_dir: std::path::PathBuf) -> BlockCollection {
        BlockCollection { base_dir }
    }

    fn blk_paths(self: &BlockCollection) -> impl Iterator<Item = PathBuf> {
        fs::read_dir(&self.base_dir)
            .expect("Could not open target directory.")
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|f| {
                f.file_name()
                    .and_then(std::ffi::OsStr::to_str)
                    .map(|s| s.starts_with("blk") && s.ends_with(".dat"))
                    .unwrap_or(false)
            })
    }

    pub fn iter(self: &BlockCollection) -> impl Iterator<Item = BitcoinBlock> {
        self.blk_paths().flat_map(|e| {
            BlockFileIterator::new(BufReader::new(
                fs::File::open(e).expect("Failed to open file."),
            ))
        })
    }

    pub fn par_iter(self: &BlockCollection) -> impl ParallelIterator<Item = BitcoinBlock> {
        self.blk_paths()
            .collect::<Vec<std::path::PathBuf>>()
            .into_par_iter()
            .flat_map(|e| {
                BlockFileIterator::new(BufReader::new(
                    fs::File::open(e).expect("Failed to open file."),
                ))
                .par_bridge()
            })
    }

    pub fn transaction_iter(self: &BlockCollection) -> impl Iterator<Item = BitcoinTransaction> {
        self.iter().flat_map(|b| b.transactions.into_iter())
    }

    pub fn transaction_par_iter(
        self: &BlockCollection,
    ) -> impl ParallelIterator<Item = BitcoinTransaction> {
        self.par_iter().flat_map(|b| b.transactions.into_par_iter())
    }
}

fn read_transaction_input<T: Read, H: Digest>(reader: &mut T, hasher: &mut H) -> io::Result<BitcoinTransactionInput> {
    let mut prev_transaction = [0u8; 32];
    reader.read_exact(&mut prev_transaction)?;
    hasher.update(&prev_transaction);

    let prev_transaction_output = read_le_u32_hash(reader, hasher)?;
    let script_size = read_varint_hash(reader, hasher)?;

    let script = BitcoinScript::new(reader, script_size, hasher)?;

    let _sequence = read_le_u32_hash(reader, hasher)?;
    println!("script_size: {:x?}", script_size);

    Ok(BitcoinTransactionInput {
        prev_transaction,
        prev_transaction_output,
        script,
    })
}

fn read_transaction_output<T: Read, H: Digest>(reader: &mut T, hasher: &mut H) -> io::Result<BitcoinTransactionOutput> {
    let value = read_le_u64_hash(reader, hasher)?;
    let script_size = read_varint_hash(reader, hasher)?;

    let script = BitcoinScript::new(reader, script_size, hasher)?;

    Ok(BitcoinTransactionOutput { value, script })
}

// Based on https://github.com/bitcoin/bips/blob/master/bip-0144.mediawiki
// and https://bitcoincore.org/en/segwit_wallet_dev/
// and https://github.com/bitcoin/bitcoin/blob/master/src/primitives/transaction.h
fn read_witness<T: Read>(reader: &mut T) -> io::Result<()> {
    let length = read_varint(reader)?;
    if length != 0 {
        for _ in 0..length {
            let inner_length = read_varint(reader)?;
            let mut buffer: Vec<u8> = std::iter::repeat(0u8).take(inner_length as usize).collect();
            reader.read_exact(&mut buffer)?;
            // hasher.input(&buffer);
        }
    }

    Ok(())
}

fn read_transaction<T: Read>(
    reader: &mut T,
    timestamp: DateTime<Utc>,
    is_coinbase: bool
) -> io::Result<BitcoinTransaction> {
    let mut hasher = Sha256::new();

    //let hash_length1 = hasher.n_bytes_hashed();
    let _version = read_le_u32_hash(reader, &mut hasher)?;
    //let hash_length2 = hasher.n_bytes_hashed();
    let dummy = read_le_u8(reader)?;
    //let hash_length3 = hasher.n_bytes_hashed();
    let input_count;
    let flags;
    let is_extended_format = dummy == 0x00;

    let mut running = false;

    if is_extended_format {
        flags = read_le_u8(reader)?;
        input_count = read_varint_hash(reader, &mut hasher)?;
    } else {
        flags = 0;
        running = true;
        hasher.update([dummy]);
        input_count = read_varint_with_prefix_hash(dummy, reader, &mut hasher)?;
    }
    //let hash_length4 = hasher.n_bytes_hashed();

    let mut inputs = Vec::with_capacity(input_count as usize);
    for _ in 0..input_count {
        inputs.push(read_transaction_input(reader, &mut hasher)?);
    }
    //let hash_length5 = hasher.n_bytes_hashed();

    let output_count = read_varint_hash(reader, &mut hasher)?;
    //let hash_length6 = hasher.n_bytes_hashed();

    let mut outputs = Vec::with_capacity(output_count as usize);
    for _ in 0..output_count {
        outputs.push(read_transaction_output(reader, &mut hasher)?);
    }
    //let hash_length7 = hasher.n_bytes_hashed();

    if is_extended_format && flags == 0x01 {
        for _ in 0..input_count {
            read_witness(reader)?;
        }
    }
    //let hash_length8 = hasher.n_bytes_hashed();

    let lock_time = read_le_u32_hash(reader, &mut hasher)?;
    //let hash_length9 = hasher.n_bytes_hashed();

    // TODO TEMP
    //let hash_length = hasher.n_bytes_hashed();
    let hash1 = <[u8; 32]>::from(hasher.finalize());
    let mut hasher2 = Sha256::new();
    hasher2.update(hash1);
    let mut hash = <[u8; 32]>::from(hasher2.finalize());
    hash.reverse();
    // println!("{:x?}", &hash);
    if hash == [0xf1,0x61,0xb4,0xa9,0x22,0xf2,0x30,0x1c,0x18,0x4f,0x3d,0xbf,0x53,0xd1,0x6d,0x6c,0x81,0xf9,0xc6,0x5f,0x54,0xcd,0x6f,0x9c,0x86,0x23,0xa4,0x2d,0x36,0x92,0x4b,0xbd]/*"f161b4a922f2301c184f3dbf53d16d6c81f9c65f54cd6f9c8623a42d36924bbd"*/ /*"2cba6c75c92066f9141ef6198e1d723bad48545218dfb0ac204c7006a870e345"*/ {
        
        // println!("Hash body size 1: {:?}", hash_length1);
        // println!("Hash body size 2: {:?}", hash_length2);
        // println!("Hash body size 3: {:?}", hash_length3);
        // println!("Hash body size 4: {:?}", hash_length4);
        // println!("Hash body size 5: {:?}", hash_length5);
        // println!("Hash body size 6: {:?}", hash_length6);
        // println!("Hash body size 7: {:?}", hash_length7);
        // println!("Hash body size 8: {:?}", hash_length8);
        // println!("Hash body size 9: {:?}", hash_length9);
        println!("Running: {:?}", running);
        println!("Dummy: {:?}", dummy);

        // println!("Hash body size: {:?}", hash_length);
        println!("Extended format: {:?}", &is_extended_format);
        println!("Hash: {:x?}", &hash);
        println!("Inputs: {:x?}", &inputs);
        println!("Outputs: {:x?}", &outputs);
        println!("Lock time: {:?}", &lock_time);
    }
    
    Ok(BitcoinTransaction {
        inputs,
        outputs,
        lock_time,
        timestamp,
        is_coinbase,
        hash
    })
}

fn read_block<T: Read>(reader: &mut T) -> io::Result<BitcoinBlock> {
    let magic_number = read_le_u32(reader)?;
    // The lask blk file seems to store a large buffer of 0s at the end, making this necessary:
    if magic_number == 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Magic number 0",
        ));
    }
    assert_eq!(magic_number, 0xd9b4bef9, "Magic number violation.");

    let _block_size = read_le_u32(reader)?;
    let version = read_le_u32(reader)?;

    let mut prev_hash = [0u8; 32];
    reader.read_exact(&mut prev_hash)?;

    let mut merkle_root = [0u8; 32];
    reader.read_exact(&mut merkle_root)?;

    let timestamp = DateTime::from_utc(
        NaiveDateTime::from_timestamp(read_le_u32(reader)? as i64, 0),
        Utc,
    );
    let _bits_target = read_le_u32(reader)?;
    let nonce = read_le_u32(reader)?;
    let transaction_count = read_varint(reader)?;

    let mut transactions = Vec::with_capacity(transaction_count as usize);
    for i in 0..transaction_count {
        transactions.push(read_transaction(reader, timestamp, i == 0)?);
    }

    Ok(BitcoinBlock {
        version,
        prev_hash,
        merkle_root,
        timestamp,
        nonce,
        transactions,
    })
}
