use std::fs::File;
use std::io::BufReader;
use std::io::Read;

struct BitcoinTransaction {

}

struct BitcoinBlock {

}

fn read_le_u8<T: Read>(reader: &mut T) -> u8 {
    let mut buffer = [0u8; 1];
    reader.read_exact(&mut buffer).expect("Could not read from file.");
    u8::from_le_bytes(buffer)
}

fn read_le_u16<T: Read>(reader: &mut T) -> u16 {
    let mut buffer = [0u8; 2];
    reader.read_exact(&mut buffer).expect("Could not read from file.");
    u16::from_le_bytes(buffer)
}

fn read_le_u32<T: Read>(reader: &mut T) -> u32 {
    let mut buffer = [0u8; 4];
    reader.read_exact(&mut buffer).expect("Could not read from file.");
    u32::from_le_bytes(buffer)
}

fn read_le_u64<T: Read>(reader: &mut T) -> u64 {
    let mut buffer = [0u8; 8];
    reader.read_exact(&mut buffer).expect("Could not read from file.");
    u64::from_le_bytes(buffer)
}

fn read_varint<T: Read>(reader: &mut T) -> u64 {
    let prefix = read_le_u8(reader);
    read_varint_with_prefix(prefix, reader)
}

fn read_varint_with_prefix<T: Read>(prefix: u8, reader: &mut T) -> u64 {
    match prefix {
        0xff => read_le_u64(reader),
        0xfe => read_le_u32(reader).into(),
        0xfd => read_le_u16(reader).into(),
        _ => prefix.into()
    }
}

fn read_transaction_input<T: Read>(reader: &mut T) {
    let mut prev_transaction_hash = [0u8; 32];
    reader.read_exact(&mut prev_transaction_hash).expect("Could not read from file.");

    let prev_transaction_output = read_le_u32(reader);
    let script_size = read_varint(reader);

    let mut script_buffer: Vec<u8> = std::iter::repeat(0u8).take(script_size as usize).collect();
    reader.read_exact(&mut script_buffer).expect("Could not read from file.");

    let sequence = read_le_u32(reader);
}

fn read_transaction_output<T: Read>(reader: &mut T) {
    let value = read_le_u64(reader);
    let script_size = read_varint(reader);

    let mut script_buffer: Vec<u8> = std::iter::repeat(0u8).take(script_size as usize).collect();
    reader.read_exact(&mut script_buffer).expect("Could not read from file.");
}

fn read_witness<T: Read>(reader: &mut T) {
    let length = read_varint(reader);
    if length != 0 {
        for _ in 0..length {
            let inner_length = read_varint(reader);
            let mut buffer: Vec<u8> = std::iter::repeat(0u8).take(inner_length as usize).collect();
            reader.read_exact(&mut buffer).expect("Could not read from file.");
        }
    }
}

// Based on https://github.com/bitcoin/bitcoin/blob/master/src/primitives/transaction.h
fn read_transaction<T: Read>(reader: &mut T) -> BitcoinTransaction {

    // let mut test_buffer = [0u8; 1000];
    // reader.read_exact(&mut test_buffer).expect("Fail");
    // println!("buffer: {:?}", test_buffer);
    // panic!("bam");

    let _version = read_le_u32(reader);
    let dummy = read_le_u8(reader);
    let input_count;
    let flags;
    let extended_format = dummy == 0x00;

    if extended_format {
        flags = read_le_u8(reader);
        input_count = read_varint(reader);
    } else {
        flags = 0;
        input_count = read_varint_with_prefix(dummy, reader);
    }

    for _ in 0..input_count {
        read_transaction_input(reader);
    }

    let output_count = read_varint(reader);

    for _ in 0..output_count {
        read_transaction_output(reader);
    }

    if extended_format && flags == 0x01 {
        for _ in 0..input_count {
            read_witness(reader);
        }
    }

    let lock_time = read_le_u32(reader);

    BitcoinTransaction{}
}

fn read_block<T: Read>(reader: &mut T) -> BitcoinBlock {
    let magic_number = read_le_u32(reader);
    assert_eq!(magic_number, 0xd9b4bef9, "Magic number violation.");

    let block_size = read_le_u32(reader);
    let version = read_le_u32(reader);

    let mut prev_hash_buffer = [0u8; 32];
    reader.read_exact(&mut prev_hash_buffer).expect("Could not read from file.");

    let mut prev_hash_merkle_root_buffer = [0u8; 32];
    reader.read_exact(&mut prev_hash_merkle_root_buffer).expect("Could not read from file.");

    let timestamp = read_le_u32(reader);
    let bits_target = read_le_u32(reader);
    let nonce = read_le_u32(reader);
    let transaction_count = read_varint(reader);

    for _ in 0..transaction_count {
        read_transaction(reader);
    }

    BitcoinBlock{}
}

// https://learnmeabitcoin.com/explorer/block/00000000000000000076cae7f4df5fb991bd3b9ba471baf9e9a4c63367d924ad
fn main() {
    let file = File::open("/Users/christopher/Documents/bitcoin-core//blocks//blk01000.dat").expect("Could not open file.");
    let mut reader = BufReader::new(file);

    read_block(&mut reader);
}
