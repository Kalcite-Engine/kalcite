#[derive(Clone,Copy,Debug,PartialEq,Eq)]pub struct Header{pub version:u32,pub schema:u32}
pub const MAGIC:[u8;4]=*b"KSAV";
pub trait Migratable:Sized{const VERSION:u32;fn migrate(from:u32,data:&[u8])->Result<Self,String>;}
pub fn schema_id(name:&str)->u32{let mut h=2166136261u32;for b in name.bytes(){h^=b as u32;h=h.wrapping_mul(16777619);}h}
pub fn encode(version:u32,schema:u32,payload:&[u8])->Vec<u8>{let mut o=Vec::with_capacity(12+payload.len());o.extend(MAGIC);o.extend(version.to_le_bytes());o.extend(schema.to_le_bytes());o.extend(payload);o}
pub fn decode(data:&[u8])->Result<(Header,&[u8]),String>{if data.len()<12||data[..4]!=MAGIC{return Err("invalid KSAV header".into())}let version=u32::from_le_bytes(data[4..8].try_into().unwrap());let schema=u32::from_le_bytes(data[8..12].try_into().unwrap());Ok((Header{version,schema},&data[12..]))}
pub fn load<T:Migratable>(expected_schema:u32,data:&[u8])->Result<T,String>{let(h,p)=decode(data)?;if h.schema!=expected_schema{return Err("save schema mismatch".into())}if h.version>T::VERSION{return Err("save was created by a newer schema version".into())}T::migrate(h.version,p)}
#[cfg(test)]mod tests{use super::*;#[test]fn roundtrip(){let b=encode(2,schema_id("Game.State"),b"abc");let(h,p)=decode(&b).unwrap();assert_eq!(h.version,2);assert_eq!(p,b"abc");}}
