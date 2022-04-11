mod opcodes;
use crate::bitcoin_parser::basic_reading::*;
use opcodes::OPCode;
use sha2::Digest;
use std::io;

#[derive(Debug)]
pub enum BitcoinScript {
    OPCodes(io::Result<Vec<OPCode>>),
    Bytes(Vec<u8>),
}

impl BitcoinScript {
    pub fn new<T: std::io::Read>(reader: &mut T, length: u64) -> io::Result<BitcoinScript> {
        let mut buffer: Vec<u8> = std::iter::repeat(0u8).take(length as usize).collect();
        reader.read_exact(&mut buffer)?;

        Ok(BitcoinScript::Bytes(buffer))
    }

    pub fn new_with_hasher<T: std::io::Read, H: Digest>(
        reader: &mut T,
        length: u64,
        hasher: &mut H,
    ) -> io::Result<BitcoinScript> {
        let mut buffer: Vec<u8> = std::iter::repeat(0u8).take(length as usize).collect();
        reader.read_exact(&mut buffer)?;
        hasher.update(&buffer);

        Ok(BitcoinScript::Bytes(buffer))
    }

    // pub fn opcodes_no_cache(self: &BitcoinScript) -> io::Result<Vec<OPCode>> {
    //     match self {
    //         BitcoinScript::OPCodes(opcodes) => opcodes.as_ref().cloned(),
    //         BitcoinScript::Bytes(bytes) => bytes_to_opcodes(bytes),
    //     }
    // }

    pub fn opcodes(self: &mut BitcoinScript) -> &io::Result<Vec<OPCode>> {
        match self {
            BitcoinScript::OPCodes(opcodes) => opcodes,
            BitcoinScript::Bytes(bytes) => {
                let opcodes = bytes_to_opcodes(bytes);
                *self = BitcoinScript::OPCodes(opcodes);
                self.opcodes()
            }
        }
    }
}

fn read_buffer<T: std::io::Read>(
    reader: &mut T,
    data_size: u64,
    length_remaining: &mut u64,
) -> io::Result<Vec<u8>> {
    if *length_remaining < data_size {
        return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "More bytes requested than remaining in Script."));
    } else {
        let mut buffer: Vec<u8> = std::iter::repeat(0u8).take(data_size as usize).collect();
        reader.read_exact(&mut buffer)?;
        *length_remaining -= data_size;

        Ok(buffer)
    }
}

fn bytes_to_opcodes(bytes: &Vec<u8>) -> io::Result<Vec<OPCode>> {
    let mut opcodes = Vec::new();
    let mut length_remaining = bytes.len() as u64;
    let mut reader = io::Cursor::new(bytes);

    while length_remaining > 0 {
        let byte = read_le_u8(&mut reader)?;
        length_remaining -= 1;

        let next_token = match byte {
            1..=75 => {
                let data_size = byte as u64;
                OPCode::Data(
                    read_buffer(&mut reader, data_size, &mut length_remaining)?,
                )
            }
            76 => {
                let data_size =
                    read_le_u8(&mut reader)? as u64;
                length_remaining -= 1;
                OPCode::Data(
                    read_buffer(&mut reader, data_size, &mut length_remaining)?,
                )
            }
            77 => {
                let data_size =
                    read_le_u16(&mut reader)? as u64;
                length_remaining -= 2;
                OPCode::Data(
                    read_buffer(&mut reader, data_size, &mut length_remaining)?,
                )
            }
            78 => {
                let data_size =
                    read_le_u32(&mut reader)? as u64;
                length_remaining -= 4;
                OPCode::Data(
                    read_buffer(&mut reader, data_size, &mut length_remaining)?,
                )
            }
            _ => opcodes::byte_to_opcode(byte),
        };

        opcodes.push(next_token);
    }

    opcodes.shrink_to_fit();
    Ok(opcodes)
}
