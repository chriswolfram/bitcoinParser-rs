use chrono::prelude::*;
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs::{self, DirEntry};
use std::io::BufReader;
use std::io::{Read, Result};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Mutex;

use std::time::Instant;

struct BitcoinTransactionInput {
    prev_transaction: [u8; 32],
    prev_transaction_output: u32,
    script: Vec<u8>,
}

struct BitcoinTransactionOutput {
    value: u64,
    script: Vec<u8>,
}

struct BitcoinTransaction {
    inputs: Vec<BitcoinTransactionInput>,
    outputs: Vec<BitcoinTransactionOutput>,
    lock_time: DateTime<Utc>,
}

struct BitcoinBlock {
    version: u32,
    prev_hash: [u8; 32],
    merkle_root: [u8; 32],
    timestamp: DateTime<Utc>,
    nonce: u32,
    transactions: Vec<BitcoinTransaction>,
}

struct BlockFileIterator<T: Read> {
    reader: T,
}

impl<T: Read> BlockFileIterator<T> {
    fn new(reader: T) -> BlockFileIterator<T> {
        BlockFileIterator { reader }
    }
}

impl<T: Read> Iterator for BlockFileIterator<T> {
    type Item = BitcoinBlock;

    fn next(self: &mut BlockFileIterator<T>) -> Option<BitcoinBlock> {
        read_block(&mut self.reader).ok()
    }
}

struct BlockCollection {
    base_dir: std::path::PathBuf,
}

impl BlockCollection {
    fn new(base_dir: std::path::PathBuf) -> BlockCollection {
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

    fn iter(self: &BlockCollection) -> impl Iterator<Item = BitcoinBlock> {
        self.blk_paths().flat_map(|e| {
            BlockFileIterator::new(BufReader::new(
                fs::File::open(e).expect("Failed to open file."),
            ))
        })
    }

    fn par_iter(self: &BlockCollection) -> impl ParallelIterator<Item = BitcoinBlock> {
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
}

fn read_le_u8<T: Read>(reader: &mut T) -> Result<u8> {
    let mut buffer = [0u8; 1];
    reader.read_exact(&mut buffer)?;
    Ok(u8::from_le_bytes(buffer))
}

fn read_le_u16<T: Read>(reader: &mut T) -> Result<u16> {
    let mut buffer = [0u8; 2];
    reader.read_exact(&mut buffer)?;
    Ok(u16::from_le_bytes(buffer))
}

fn read_le_u32<T: Read>(reader: &mut T) -> Result<u32> {
    let mut buffer = [0u8; 4];
    reader.read_exact(&mut buffer)?;
    Ok(u32::from_le_bytes(buffer))
}

fn read_le_u64<T: Read>(reader: &mut T) -> Result<u64> {
    let mut buffer = [0u8; 8];
    reader.read_exact(&mut buffer)?;
    Ok(u64::from_le_bytes(buffer))
}

fn read_varint<T: Read>(reader: &mut T) -> Result<u64> {
    let prefix = read_le_u8(reader)?;
    read_varint_with_prefix(prefix, reader)
}

fn read_varint_with_prefix<T: Read>(prefix: u8, reader: &mut T) -> Result<u64> {
    match prefix {
        0xff => read_le_u64(reader),
        0xfe => Ok(read_le_u32(reader)?.into()),
        0xfd => Ok(read_le_u16(reader)?.into()),
        _ => Ok(prefix.into()),
    }
}

fn read_transaction_input<T: Read>(reader: &mut T) -> Result<BitcoinTransactionInput> {
    let mut prev_transaction = [0u8; 32];
    reader.read_exact(&mut prev_transaction)?;

    let prev_transaction_output = read_le_u32(reader)?;
    let script_size = read_varint(reader)?;

    let mut script: Vec<u8> = std::iter::repeat(0u8).take(script_size as usize).collect();
    reader.read_exact(&mut script)?;

    let _sequence = read_le_u32(reader)?;

    Ok(BitcoinTransactionInput {
        prev_transaction,
        prev_transaction_output,
        script,
    })
}

fn read_transaction_output<T: Read>(reader: &mut T) -> Result<BitcoinTransactionOutput> {
    let value = read_le_u64(reader)?;
    let script_size = read_varint(reader)?;

    let mut script: Vec<u8> = std::iter::repeat(0u8).take(script_size as usize).collect();
    reader.read_exact(&mut script)?;

    Ok(BitcoinTransactionOutput { value, script })
}

// Based on https://github.com/bitcoin/bips/blob/master/bip-0144.mediawiki
// and https://bitcoincore.org/en/segwit_wallet_dev/
// and https://github.com/bitcoin/bitcoin/blob/master/src/primitives/transaction.h
fn read_witness<T: Read>(reader: &mut T) -> Result<()> {
    let length = read_varint(reader)?;
    if length != 0 {
        for _ in 0..length {
            let inner_length = read_varint(reader)?;
            let mut buffer: Vec<u8> = std::iter::repeat(0u8).take(inner_length as usize).collect();
            reader.read_exact(&mut buffer)?;
        }
    }

    Ok(())
}

fn read_transaction<T: Read>(reader: &mut T) -> Result<BitcoinTransaction> {
    let _version = read_le_u32(reader)?;
    let dummy = read_le_u8(reader)?;
    let input_count;
    let flags;
    let extended_format = dummy == 0x00;

    if extended_format {
        flags = read_le_u8(reader)?;
        input_count = read_varint(reader)?;
    } else {
        flags = 0;
        input_count = read_varint_with_prefix(dummy, reader)?;
    }

    let mut inputs = Vec::with_capacity(input_count as usize);
    for _ in 0..input_count {
        inputs.push(read_transaction_input(reader)?);
    }

    let output_count = read_varint(reader)?;

    let mut outputs = Vec::with_capacity(output_count as usize);
    for _ in 0..output_count {
        outputs.push(read_transaction_output(reader)?);
    }

    if extended_format && flags == 0x01 {
        for _ in 0..input_count {
            read_witness(reader)?;
        }
    }

    let lock_time = DateTime::from_utc(
        NaiveDateTime::from_timestamp(read_le_u32(reader)? as i64, 0),
        Utc,
    );

    Ok(BitcoinTransaction {
        inputs,
        outputs,
        lock_time,
    })
}

fn read_block<T: Read>(reader: &mut T) -> Result<BitcoinBlock> {
    let magic_number = read_le_u32(reader)?;
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
    for _ in 0..transaction_count {
        transactions.push(read_transaction(reader)?);
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

fn main() {
    let blocks = BlockCollection::new(std::path::PathBuf::from("/Users/christopher/Documents/bitcoin-core//blocks/"));

    // Sequential
    let start = Instant::now();
    let mut block_count = 0;
    for _ in blocks.iter() {
        block_count += 1;
        if block_count % 10000 == 0 {
            println!("Block: {:?}\tTime: {:?}", block_count, start.elapsed());
        }
    }
    println!("-- Sequential --\nBlock count: {:?}\nTime: {:?}", block_count, start.elapsed());

    // Parallel
    let start = Instant::now();
    let block_count = std::sync::atomic::AtomicUsize::from(0);
    blocks.par_iter().for_each(|_| {
        block_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let static_block_count = block_count.load(std::sync::atomic::Ordering::Relaxed);
        if static_block_count % 10000 == 0 {
            println!("Block: {:?}\tTime: {:?}", static_block_count, start.elapsed());
        }
    });
    println!("-- Parallel --\nBlock count: {:?}\nTime: {:?}", block_count, start.elapsed());
}
