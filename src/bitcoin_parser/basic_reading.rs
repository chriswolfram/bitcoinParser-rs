use std::io;
use sha2::Digest;

pub fn read_le_u8<T: io::Read>(reader: &mut T) -> io::Result<u8> {
    let mut buffer = [0u8; 1];
    reader.read_exact(&mut buffer)?;
    Ok(u8::from_le_bytes(buffer))
}

pub fn read_le_u16<T: io::Read>(reader: &mut T) -> io::Result<u16> {
    let mut buffer = [0u8; 2];
    reader.read_exact(&mut buffer)?;
    Ok(u16::from_le_bytes(buffer))
}

pub fn read_le_u32<T: io::Read>(reader: &mut T) -> io::Result<u32> {
    let mut buffer = [0u8; 4];
    reader.read_exact(&mut buffer)?;
    Ok(u32::from_le_bytes(buffer))
}

pub fn read_le_u64<T: io::Read>(reader: &mut T) -> io::Result<u64> {
    let mut buffer = [0u8; 8];
    reader.read_exact(&mut buffer)?;
    Ok(u64::from_le_bytes(buffer))
}

pub fn read_varint<T: io::Read>(reader: &mut T) -> io::Result<u64> {
    let prefix = read_le_u8(reader)?;
    read_varint_with_prefix(prefix, reader)
}

pub fn read_varint_with_prefix<T: io::Read>(prefix: u8, reader: &mut T) -> io::Result<u64> {
    match prefix {
        0xff => read_le_u64(reader),
        0xfe => Ok(read_le_u32(reader)?.into()),
        0xfd => Ok(read_le_u16(reader)?.into()),
        _ => Ok(prefix.into()),
    }
}

// Hashing versions
pub fn read_le_u8_hash<T: io::Read, H: Digest>(reader: &mut T, hasher: &mut H) -> io::Result<u8> {
    let mut buffer = [0u8; 1];
    reader.read_exact(&mut buffer)?;
    hasher.update(&buffer);
    Ok(u8::from_le_bytes(buffer))
}

pub fn read_le_u16_hash<T: io::Read, H: Digest>(reader: &mut T, hasher: &mut H) -> io::Result<u16> {
    let mut buffer = [0u8; 2];
    reader.read_exact(&mut buffer)?;
    hasher.update(&buffer);
    Ok(u16::from_le_bytes(buffer))
}

pub fn read_le_u32_hash<T: io::Read, H: Digest>(reader: &mut T, hasher: &mut H) -> io::Result<u32> {
    let mut buffer = [0u8; 4];
    reader.read_exact(&mut buffer)?;
    hasher.update(&buffer);
    Ok(u32::from_le_bytes(buffer))
}

pub fn read_le_u64_hash<T: io::Read, H: Digest>(reader: &mut T, hasher: &mut H) -> io::Result<u64> {
    let mut buffer = [0u8; 8];
    reader.read_exact(&mut buffer)?;
    hasher.update(&buffer);
    Ok(u64::from_le_bytes(buffer))
}

pub fn read_varint_hash<T: io::Read, H: Digest>(reader: &mut T, hasher: &mut H) -> io::Result<u64> {
    let prefix = read_le_u8_hash(reader, hasher)?;
    read_varint_with_prefix_hash(prefix, reader, hasher)
}

pub fn read_varint_with_prefix_hash<T: io::Read, H: Digest>(prefix: u8, reader: &mut T, hasher: &mut H) -> io::Result<u64> {
    match prefix {
        0xff => read_le_u64_hash(reader, hasher),
        0xfe => Ok(read_le_u32_hash(reader, hasher)?.into()),
        0xfd => Ok(read_le_u16_hash(reader, hasher)?.into()),
        _ => Ok(prefix.into()),
    }
}