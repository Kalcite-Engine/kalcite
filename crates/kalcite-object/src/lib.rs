#![no_std]

pub const MAGIC: [u8; 4] = *b"KCO\0";
pub const FORMAT_VERSION: u16 = 1;
pub const HEADER_SIZE: usize = 16;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Portable = 0,
    NumWorks = 1,
    Desktop = 2,
    Web = 3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Header {
    pub version: u16,
    pub target: Target,
    pub flags: u8,
    pub payload_len: u32,
    pub checksum: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObjectError {
    BufferTooSmall,
    BadMagic,
    UnsupportedVersion,
    UnknownTarget,
    Truncated,
    BadChecksum,
}

pub const fn checksum(bytes: &[u8]) -> u32 {
    let mut hash = 0x811c_9dc5u32;
    let mut i = 0;
    while i < bytes.len() {
        hash ^= bytes[i] as u32;
        hash = hash.wrapping_mul(0x0100_0193);
        i += 1;
    }
    hash
}

pub fn encode(
    target: Target,
    flags: u8,
    payload: &[u8],
    output: &mut [u8],
) -> Result<usize, ObjectError> {
    let total = HEADER_SIZE + payload.len();
    if output.len() < total {
        return Err(ObjectError::BufferTooSmall);
    }
    output[..4].copy_from_slice(&MAGIC);
    output[4..6].copy_from_slice(&FORMAT_VERSION.to_le_bytes());
    output[6] = target as u8;
    output[7] = flags;
    output[8..12].copy_from_slice(&(payload.len() as u32).to_le_bytes());
    output[12..16].copy_from_slice(&checksum(payload).to_le_bytes());
    output[HEADER_SIZE..total].copy_from_slice(payload);
    Ok(total)
}

pub fn decode(input: &[u8]) -> Result<(Header, &[u8]), ObjectError> {
    if input.len() < HEADER_SIZE {
        return Err(ObjectError::Truncated);
    }
    if input[..4] != MAGIC {
        return Err(ObjectError::BadMagic);
    }
    let version = u16::from_le_bytes([input[4], input[5]]);
    if version != FORMAT_VERSION {
        return Err(ObjectError::UnsupportedVersion);
    }
    let target = match input[6] {
        0 => Target::Portable,
        1 => Target::NumWorks,
        2 => Target::Desktop,
        3 => Target::Web,
        _ => return Err(ObjectError::UnknownTarget),
    };
    let payload_len = u32::from_le_bytes([input[8], input[9], input[10], input[11]]);
    let expected = u32::from_le_bytes([input[12], input[13], input[14], input[15]]);
    let end = HEADER_SIZE
        .checked_add(payload_len as usize)
        .ok_or(ObjectError::Truncated)?;
    if input.len() < end {
        return Err(ObjectError::Truncated);
    }
    let payload = &input[HEADER_SIZE..end];
    if checksum(payload) != expected {
        return Err(ObjectError::BadChecksum);
    }
    Ok((
        Header {
            version,
            target,
            flags: input[7],
            payload_len,
            checksum: expected,
        },
        payload,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn round_trip() {
        let mut out = [0u8; 64];
        let n = encode(Target::NumWorks, 0, b"hello", &mut out).unwrap();
        let (header, payload) = decode(&out[..n]).unwrap();
        assert_eq!(header.target, Target::NumWorks);
        assert_eq!(payload, b"hello");
    }
}
