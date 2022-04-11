mod basic_reading;
mod script;

use basic_reading::*;
use chrono::prelude::*;
use rayon::prelude::*;
use script::BitcoinScript;
use sha2::{Digest, Sha256};
use std::fs;
use std::io;
use std::io::{BufReader, Read};
use std::path::PathBuf;

#[derive(Debug)]
pub struct BitcoinTransactionInput {
    pub prev_transaction: [u8; 32],
    pub prev_transaction_output: u32,
    pub script: BitcoinScript,
}

#[derive(Debug)]
pub struct BitcoinTransactionOutput {
    pub value: u64,
    pub script: BitcoinScript,
}

#[derive(Debug)]
pub struct BitcoinTransaction {
    pub inputs: Vec<BitcoinTransactionInput>,
    pub outputs: Vec<BitcoinTransactionOutput>,
    pub witnesses: Option<Vec<Vec<BitcoinScript>>>,
    pub lock_time: u32,
    pub timestamp: DateTime<Utc>,
    pub is_coinbase: bool,
    pub txid: [u8; 32],
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

impl BitcoinTransactionInput {
    pub fn new<T: Read, H: Digest>(
        reader: &mut T,
        hasher: &mut H,
    ) -> io::Result<BitcoinTransactionInput> {
        let mut prev_transaction = [0u8; 32];
        reader.read_exact(&mut prev_transaction)?;
        hasher.update(&prev_transaction);

        let prev_transaction_output = read_le_u32_hash(reader, hasher)?;
        let script_size = read_varint_hash(reader, hasher)?;

        let script = BitcoinScript::new_with_hasher(reader, script_size, hasher)?;

        let _sequence = read_le_u32_hash(reader, hasher)?;

        Ok(BitcoinTransactionInput {
            prev_transaction,
            prev_transaction_output,
            script,
        })
    }
}

impl BitcoinTransactionOutput {
    pub fn new<T: Read, H: Digest>(
        reader: &mut T,
        hasher: &mut H,
    ) -> io::Result<BitcoinTransactionOutput> {
        let value = read_le_u64_hash(reader, hasher)?;
        let script_size = read_varint_hash(reader, hasher)?;

        let script = BitcoinScript::new_with_hasher(reader, script_size, hasher)?;

        Ok(BitcoinTransactionOutput { value, script })
    }
}

impl BitcoinTransaction {
    pub fn new<T: Read>(
        reader: &mut T,
        timestamp: DateTime<Utc>,
        is_coinbase: bool,
    ) -> io::Result<BitcoinTransaction> {
        let mut hasher = Sha256::new();

        let _version = read_le_u32_hash(reader, &mut hasher)?;
        let dummy = read_le_u8(reader)?;
        let input_count;
        let flags;
        let is_extended_format = dummy == 0x00;

        if is_extended_format {
            flags = read_le_u8(reader)?;
            input_count = read_varint_hash(reader, &mut hasher)?;
        } else {
            flags = 0;
            hasher.update([dummy]);
            input_count = read_varint_with_prefix_hash(dummy, reader, &mut hasher)?;
        }

        let mut inputs = Vec::with_capacity(input_count as usize);
        for _ in 0..input_count {
            inputs.push(BitcoinTransactionInput::new(reader, &mut hasher)?);
        }

        let output_count = read_varint_hash(reader, &mut hasher)?;

        let mut outputs = Vec::with_capacity(output_count as usize);
        for _ in 0..output_count {
            outputs.push(BitcoinTransactionOutput::new(reader, &mut hasher)?);
        }

        // Based on https://github.com/bitcoin/bips/blob/master/bip-0144.mediawiki
        // and https://bitcoincore.org/en/segwit_wallet_dev/
        // and https://github.com/bitcoin/bitcoin/blob/master/src/primitives/transaction.h
        let mut witnesses = None;
        if is_extended_format && flags == 0x01 {
            witnesses = Some(Vec::with_capacity(inputs.len()));
            for _ in 0..input_count {
                let input_witness_count = read_varint(reader)?;
                let mut input_witnesses = Vec::with_capacity(input_witness_count as usize);
                for _ in 0..input_witness_count {
                    let witness_length = read_varint(reader)?;
                    let witness = BitcoinScript::new(reader, witness_length)?;
                    input_witnesses.push(witness);
                }
                if let Some(witnesses_vec) = &mut witnesses {
                    witnesses_vec.push(input_witnesses);
                }
            }
        }

        let lock_time = read_le_u32_hash(reader, &mut hasher)?;

        let hash1 = <[u8; 32]>::from(hasher.finalize());
        let mut hasher2 = Sha256::new();
        hasher2.update(hash1);
        let mut hash = <[u8; 32]>::from(hasher2.finalize());
        hash.reverse();

        Ok(BitcoinTransaction {
            inputs,
            outputs,
            lock_time,
            timestamp,
            is_coinbase,
            witnesses,
            txid: hash,
        })
    }

    pub fn value(self: &BitcoinTransaction) -> u64 {
        self.outputs.iter().map(|o| o.value).sum()
    }
}

impl BitcoinBlock {
    pub fn new<T: Read>(reader: &mut T) -> io::Result<BitcoinBlock> {
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
            transactions.push(BitcoinTransaction::new(reader, timestamp, i == 0)?);
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
        BitcoinBlock::new(&mut self.reader).ok()
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
