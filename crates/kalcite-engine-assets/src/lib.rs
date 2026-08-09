#![no_std]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    Malformed,
    OutputTooSmall,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Asset<'a> {
    pub id: u64,
    pub kind: u8,
    pub name: &'a str,
    pub data: &'a [u8],
}

pub struct AssetPack<'a> {
    bytes: &'a [u8],
    entries: usize,
    payloads: usize,
}

impl<'a> AssetPack<'a> {
    pub fn new(bytes: &'a [u8]) -> Result<Self, DecodeError> {
        if bytes.get(..4) != Some(b"KAP1") {
            return Err(DecodeError::Malformed);
        }
        let entries = read_u16(bytes, 4)? as usize;
        let payloads = read_u16(bytes, 6)? as usize;
        let pack = Self {
            bytes,
            entries,
            payloads,
        };
        let _ = pack.payload_start()?;
        Ok(pack)
    }

    pub fn get(&self, id: u64) -> Option<Asset<'a>> {
        self.entries().find(|asset| asset.id == id)
    }

    pub fn get_named(&self, name: &str) -> Option<Asset<'a>> {
        self.get(asset_id(name)).filter(|asset| asset.name == name)
    }

    pub fn entries(&self) -> impl Iterator<Item = Asset<'a>> + '_ {
        (0..self.entries).filter_map(|wanted| self.entry(wanted).ok())
    }

    fn entry(&self, wanted: usize) -> Result<Asset<'a>, DecodeError> {
        let mut offset = 8;
        for index in 0..self.entries {
            let id = read_u64(self.bytes, offset)?;
            let kind = *self.bytes.get(offset + 8).ok_or(DecodeError::Malformed)?;
            let name_len = read_u16(self.bytes, offset + 9)? as usize;
            let payload = read_u16(self.bytes, offset + 11)? as usize;
            let name_start = offset + 13;
            let name_end = name_start
                .checked_add(name_len)
                .ok_or(DecodeError::Malformed)?;
            let name = core::str::from_utf8(
                self.bytes
                    .get(name_start..name_end)
                    .ok_or(DecodeError::Malformed)?,
            )
            .map_err(|_| DecodeError::Malformed)?;
            if index == wanted {
                return Ok(Asset {
                    id,
                    kind,
                    name,
                    data: self.payload(payload)?,
                });
            }
            offset = name_end;
        }
        Err(DecodeError::Malformed)
    }

    fn payload_start(&self) -> Result<usize, DecodeError> {
        let mut offset = 8;
        for _ in 0..self.entries {
            let name_len = read_u16(self.bytes, offset + 9)? as usize;
            offset = offset
                .checked_add(13 + name_len)
                .ok_or(DecodeError::Malformed)?;
            if offset > self.bytes.len() {
                return Err(DecodeError::Malformed);
            }
        }
        Ok(offset)
    }

    fn payload(&self, wanted: usize) -> Result<&'a [u8], DecodeError> {
        if wanted >= self.payloads {
            return Err(DecodeError::Malformed);
        }
        let mut offset = self.payload_start()?;
        for index in 0..self.payloads {
            let len = read_u32(self.bytes, offset)? as usize;
            let start = offset + 4;
            let end = start.checked_add(len).ok_or(DecodeError::Malformed)?;
            let data = self.bytes.get(start..end).ok_or(DecodeError::Malformed)?;
            if index == wanted {
                return Ok(data);
            }
            offset = end;
        }
        Err(DecodeError::Malformed)
    }
}

pub fn asset_id(name: &str) -> u64 {
    let mut hash = 14_695_981_039_346_656_037u64;
    for byte in name.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(1_099_511_628_211);
    }
    hash
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, DecodeError> {
    Ok(u16::from_le_bytes(
        bytes
            .get(offset..offset + 2)
            .ok_or(DecodeError::Malformed)?
            .try_into()
            .map_err(|_| DecodeError::Malformed)?,
    ))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, DecodeError> {
    Ok(u32::from_le_bytes(
        bytes
            .get(offset..offset + 4)
            .ok_or(DecodeError::Malformed)?
            .try_into()
            .map_err(|_| DecodeError::Malformed)?,
    ))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, DecodeError> {
    Ok(u64::from_le_bytes(
        bytes
            .get(offset..offset + 8)
            .ok_or(DecodeError::Malformed)?
            .try_into()
            .map_err(|_| DecodeError::Malformed)?,
    ))
}
/// Paires `(run:u8, value:u8)`. Un run nul est invalide.
pub fn decode_rle8(input: &[u8], out: &mut [u8]) -> Result<usize, DecodeError> {
    if input.len() % 2 != 0 {
        return Err(DecodeError::Malformed);
    }
    let mut at = 0;
    for p in input.chunks_exact(2) {
        let n = p[0] as usize;
        if n == 0 {
            return Err(DecodeError::Malformed);
        }
        if at + n > out.len() {
            return Err(DecodeError::OutputTooSmall);
        }
        out[at..at + n].fill(p[1]);
        at += n
    }
    Ok(at)
}
/// RGB565: `(run:u8, lo:u8, hi:u8)`.
pub fn decode_rle565(input: &[u8], out: &mut [u16]) -> Result<usize, DecodeError> {
    if input.len() % 3 != 0 {
        return Err(DecodeError::Malformed);
    }
    let mut at = 0;
    for p in input.chunks_exact(3) {
        let n = p[0] as usize;
        if n == 0 {
            return Err(DecodeError::Malformed);
        }
        if at + n > out.len() {
            return Err(DecodeError::OutputTooSmall);
        }
        out[at..at + n].fill(u16::from_le_bytes([p[1], p[2]]));
        at += n
    }
    Ok(at)
}
#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    #[test]
    fn rle() {
        let mut o = [0; 5];
        assert_eq!(decode_rle8(&[3, 7, 2, 9], &mut o), Ok(5));
        assert_eq!(o, [7, 7, 7, 9, 9])
    }

    #[test]
    fn pack_lookup_resolves_deduplicated_payload() {
        let name = "hero.png";
        let mut bytes = std::vec::Vec::new();
        bytes.extend(*b"KAP1");
        bytes.extend(1u16.to_le_bytes());
        bytes.extend(1u16.to_le_bytes());
        bytes.extend(asset_id(name).to_le_bytes());
        bytes.push(1);
        bytes.extend((name.len() as u16).to_le_bytes());
        bytes.extend(0u16.to_le_bytes());
        bytes.extend(name.as_bytes());
        bytes.extend(3u32.to_le_bytes());
        bytes.extend([1, 2, 3]);
        let pack = AssetPack::new(&bytes).unwrap();
        let asset = pack.get_named(name).unwrap();
        assert_eq!(asset.kind, 1);
        assert_eq!(asset.data, [1, 2, 3]);
    }
}
