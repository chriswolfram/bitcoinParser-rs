mod opcodes;
use crate::bitcoin_parser::basic_reading::*;
use opcodes::OPCode;
use std::io;
use bitcoin_hashes::HashEngine;

#[derive(Debug)]
pub struct BitcoinScript {
    pub opcodes: Vec<OPCode>,
}

#[derive(Debug)]
pub enum BitcoinScriptParseError {
    TooLong,
}

fn read_buffer<T: std::io::Read, H: HashEngine>(
    reader: &mut T,
    data_size: u64,
    length_remaining: &mut u64,
    hasher: &mut H
) -> io::Result<Result<Vec<u8>, BitcoinScriptParseError>> {
    if *length_remaining < data_size {
        return Ok(Err(BitcoinScriptParseError::TooLong));
    } else {
        let mut buffer: Vec<u8> = std::iter::repeat(0u8).take(data_size as usize).collect();
        reader.read_exact(&mut buffer)?;
        hasher.input(&buffer);
        *length_remaining -= data_size;

        Ok(Ok(buffer))
    }
}

impl BitcoinScript {
    pub fn new<T: std::io::Read, H: HashEngine>(
        reader: &mut T,
        length: u64,
        hasher: &mut H
    ) -> io::Result<Result<BitcoinScript, BitcoinScriptParseError>> {
        let mut opcodes = Vec::new();
        let mut length_remaining = length;

        while length_remaining > 0 {
            let byte = read_le_u8_hash(reader, hasher)?;
            length_remaining -= 1;

            let next_token = match byte {
                1..=75 => {
                    let data_size = byte as u64;
                    let buffer = read_buffer(reader, data_size, &mut length_remaining, hasher)?;
                    match buffer {
                        Ok(v) => OPCode::Data(v),
                        Err(e) => return Ok(Err(e)),
                    }
                }
                76 => {
                    let data_size = read_le_u8_hash(reader, hasher)? as u64;
                    length_remaining -= 1;
                    let buffer = read_buffer(reader, data_size, &mut length_remaining, hasher)?;
                    match buffer {
                        Ok(v) => OPCode::Data(v),
                        Err(e) => return Ok(Err(e)),
                    }
                }
                77 => {
                    let data_size = read_le_u16_hash(reader, hasher)? as u64;
                    length_remaining -= 2;
                    let buffer = read_buffer(reader, data_size, &mut length_remaining, hasher)?;
                    match buffer {
                        Ok(v) => OPCode::Data(v),
                        Err(e) => return Ok(Err(e)),
                    }
                }
                78 => {
                    let data_size = read_le_u32_hash(reader, hasher)? as u64;
                    length_remaining -= 4;
                    let buffer = read_buffer(reader, data_size, &mut length_remaining, hasher)?;
                    match buffer {
                        Ok(v) => OPCode::Data(v),
                        Err(e) => return Ok(Err(e)),
                    }
                }
                _ => opcodes::byte_to_opcode(byte),
            };

            opcodes.push(next_token);
            opcodes.shrink_to_fit();
        }

        Ok(Ok(BitcoinScript { opcodes }))
    }
}
