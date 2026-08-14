#![allow(dead_code)]
use crate::platform::{Color, Storage, Vec2fx};

pub struct Math;
impl Math {
    #[inline] pub fn clamp_i16(v:i16,lo:i16,hi:i16)->i16 { v.clamp(lo,hi) }
    #[inline] pub fn abs_i16(v:i16)->i16 { v.saturating_abs() }
    #[inline] pub fn min_u32(a:u32,b:u32)->u32 { core::cmp::min(a,b) }
    #[inline] pub fn max_u32(a:u32,b:u32)->u32 { core::cmp::max(a,b) }
}


pub struct Bits;
impl Bits {
    #[inline] pub fn test_u32(v:u32,bit:u8)->bool { bit<32 && (v & (1u32<<bit))!=0 }
    #[inline] pub fn set_u32(v:u32,bit:u8)->u32 { if bit<32{v|(1u32<<bit)}else{v} }
    #[inline] pub fn clear_u32(v:u32,bit:u8)->u32 { if bit<32{v&!(1u32<<bit)}else{v} }
    #[inline] pub fn toggle_u32(v:u32,bit:u8)->u32 { if bit<32{v^(1u32<<bit)}else{v} }
}

pub struct Fixed;
impl Fixed {
    #[inline] pub fn mul8(a:i16,b:i16)->i16 { (((a as i32)*(b as i32))>>8) as i16 }
    #[inline] pub fn div8(a:i16,b:i16)->i16 { if b==0{0}else{(((a as i32)<<8)/(b as i32)) as i16} }
}

pub struct ColorUtil;
impl ColorUtil {
    pub fn rgb565(r:u8,g:u8,b:u8)->Color { Color((((r as u16>>3)<<11)|((g as u16>>2)<<5)|(b as u16>>3)) as u16) }
}

pub struct Checksum;
impl Checksum {
    #[inline] pub fn fnv1a_u32(value:u32)->u32 {
        let mut h=0x811c9dc5u32;
        for b in value.to_le_bytes(){h^=b as u32;h=h.wrapping_mul(0x01000193);} h
    }
}

/// Bounded SHA-256 helpers. The full digest is deliberately exposed through
/// fixed words, which keeps the KLC ABI allocation-free.
pub struct Hash;
impl Hash {
    pub fn sha256_u32_prefix(value: u32) -> u32 { sha256(&value.to_be_bytes())[0] }
}

/// Desktop filesystem bridge. Targets without a host filesystem return safe
/// failure values; project capability checks decide whether those targets may
/// be selected by applications that need filesystem access.
pub struct Fs;
impl Fs {
    #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
    pub fn exists(path: &str) -> bool { std::path::Path::new(path).is_file() }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    pub fn exists(_: &str) -> bool { false }
    #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
    pub fn read_u32(path: &str, fallback: u32) -> u32 { std::fs::read(path).ok().and_then(|b| b.try_into().ok()).map(u32::from_le_bytes).unwrap_or(fallback) }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    pub fn read_u32(_: &str, fallback: u32) -> u32 { fallback }
    #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
    pub fn write_u32(path: &str, value: u32) -> bool { std::fs::write(path, value.to_le_bytes()).is_ok() }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    pub fn write_u32(_: &str, _: u32) -> bool { false }
}

pub struct Http;
impl Http { #[inline] pub fn available() -> bool { cfg!(any(target_os = "linux", target_os = "macos", target_os = "windows")) } }
pub struct Git;
impl Git { #[inline] pub fn available() -> bool { cfg!(any(target_os = "linux", target_os = "macos", target_os = "windows")) } }

fn sha256(data: &[u8]) -> [u32; 8] {
    const K: [u32; 64] = [0x428a2f98,0x71374491,0xb5c0fbcf,0xe9b5dba5,0x3956c25b,0x59f111f1,0x923f82a4,0xab1c5ed5,0xd807aa98,0x12835b01,0x243185be,0x550c7dc3,0x72be5d74,0x80deb1fe,0x9bdc06a7,0xc19bf174,0xe49b69c1,0xefbe4786,0x0fc19dc6,0x240ca1cc,0x2de92c6f,0x4a7484aa,0x5cb0a9dc,0x76f988da,0x983e5152,0xa831c66d,0xb00327c8,0xbf597fc7,0xc6e00bf3,0xd5a79147,0x06ca6351,0x14292967,0x27b70a85,0x2e1b2138,0x4d2c6dfc,0x53380d13,0x650a7354,0x766a0abb,0x81c2c92e,0x92722c85,0xa2bfe8a1,0xa81a664b,0xc24b8b70,0xc76c51a3,0xd192e819,0xd6990624,0xf40e3585,0x106aa070,0x19a4c116,0x1e376c08,0x2748774c,0x34b0bcb5,0x391c0cb3,0x4ed8aa4a,0x5b9cca4f,0x682e6ff3,0x748f82ee,0x78a5636f,0x84c87814,0x8cc70208,0x90befffa,0xa4506ceb,0xbef9a3f7,0xc67178f2];
    let mut block=[0u8;64]; let n=data.len().min(55); block[..n].copy_from_slice(&data[..n]); block[n]=0x80; block[56..].copy_from_slice(&((data.len() as u64)*8).to_be_bytes());
    let mut w=[0u32;64]; for i in 0..16 { w[i]=u32::from_be_bytes(block[i*4..i*4+4].try_into().unwrap()); } for i in 16..64 { let a=w[i-15].rotate_right(7)^w[i-15].rotate_right(18)^(w[i-15]>>3); let b=w[i-2].rotate_right(17)^w[i-2].rotate_right(19)^(w[i-2]>>10); w[i]=w[i-16].wrapping_add(a).wrapping_add(w[i-7]).wrapping_add(b); }
    let mut h=[0x6a09e667,0xbb67ae85,0x3c6ef372,0xa54ff53a,0x510e527f,0x9b05688c,0x1f83d9ab,0x5be0cd19]; let(mut a,mut b,mut c,mut d,mut e,mut f,mut g,mut q)=(h[0],h[1],h[2],h[3],h[4],h[5],h[6],h[7]); for i in 0..64 { let s1=e.rotate_right(6)^e.rotate_right(11)^e.rotate_right(25); let ch=(e&f)^(!e&g); let t1=q.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]); let s0=a.rotate_right(2)^a.rotate_right(13)^a.rotate_right(22); let maj=(a&b)^(a&c)^(b&c); q=g;g=f;f=e;e=d.wrapping_add(t1);d=c;c=b;b=a;a=t1.wrapping_add(s0).wrapping_add(maj); } h[0]=h[0].wrapping_add(a);h[1]=h[1].wrapping_add(b);h[2]=h[2].wrapping_add(c);h[3]=h[3].wrapping_add(d);h[4]=h[4].wrapping_add(e);h[5]=h[5].wrapping_add(f);h[6]=h[6].wrapping_add(g);h[7]=h[7].wrapping_add(q); h
}

pub struct MsgPack;
impl MsgPack {
    pub fn write_u32(name:&str,value:u32)->bool { let mut b=[0u8;5]; let n=encode_u32(value,&mut b); Storage::write_bytes(name,&b[..n]) }
    pub fn read_u32(name:&str,fallback:u32)->u32 { let mut b=[0u8;8]; let n=Storage::read_into(name,&mut b); decode_u32(&b[..n]).unwrap_or(fallback) }
    pub fn write_i32(name:&str,value:i32)->bool { let mut b=[0u8;5]; let n=encode_i32(value,&mut b); Storage::write_bytes(name,&b[..n]) }
    pub fn read_i32(name:&str,fallback:i32)->i32 { let mut b=[0u8;8]; let n=Storage::read_into(name,&mut b); decode_i32(&b[..n]).unwrap_or(fallback) }
    pub fn write_bool(name:&str,value:bool)->bool { Storage::write_bytes(name,&[if value{0xc3}else{0xc2}]) }
    pub fn read_bool(name:&str,fallback:bool)->bool { let mut b=[0u8;2]; let n=Storage::read_into(name,&mut b); if n==1&&b[0]==0xc3{true}else if n==1&&b[0]==0xc2{false}else{fallback} }
    pub fn write_vec2fx(name:&str,value:Vec2fx)->bool { let mut b=[0u8;11]; b[0]=0x92; let n1=encode_i32(value.x as i32,&mut b[1..6]); let n2=encode_i32(value.y as i32,&mut b[1+n1..]); Storage::write_bytes(name,&b[..1+n1+n2]) }
    pub fn read_vec2fx(name:&str,fallback:Vec2fx)->Vec2fx { let mut b=[0u8;16]; let n=Storage::read_into(name,&mut b); decode_vec2(&b[..n]).unwrap_or(fallback) }
}

pub struct Save;
impl Save {
    #[inline] pub fn u32(name:&str,value:u32)->bool { MsgPack::write_u32(name,value) }
    #[inline] pub fn load_u32(name:&str,fallback:u32)->u32 { MsgPack::read_u32(name,fallback) }
    #[inline] pub fn i32(name:&str,value:i32)->bool { MsgPack::write_i32(name,value) }
    #[inline] pub fn load_i32(name:&str,fallback:i32)->i32 { MsgPack::read_i32(name,fallback) }
    #[inline] pub fn boolean(name:&str,value:bool)->bool { MsgPack::write_bool(name,value) }
    #[inline] pub fn load_bool(name:&str,fallback:bool)->bool { MsgPack::read_bool(name,fallback) }
}

fn encode_u32(v:u32,o:&mut[u8])->usize { if v<=0x7f{o[0]=v as u8;1}else if v<=0xff{o[0]=0xcc;o[1]=v as u8;2}else if v<=0xffff{o[0]=0xcd;o[1..3].copy_from_slice(&(v as u16).to_be_bytes());3}else{o[0]=0xce;o[1..5].copy_from_slice(&v.to_be_bytes());5} }
fn decode_u32(b:&[u8])->Option<u32>{match b.first().copied()?{0x00..=0x7f=>Some(b[0] as u32),0xcc if b.len()>=2=>Some(b[1] as u32),0xcd if b.len()>=3=>Some(u16::from_be_bytes([b[1],b[2]]) as u32),0xce if b.len()>=5=>Some(u32::from_be_bytes([b[1],b[2],b[3],b[4]])),_=>None}}
fn encode_i32(v:i32,o:&mut[u8])->usize { if (0..=127).contains(&v){o[0]=v as u8;1}else if (-32..=-1).contains(&v){o[0]=v as i8 as u8;1}else if (-128..=127).contains(&v){o[0]=0xd0;o[1]=v as i8 as u8;2}else if (-32768..=32767).contains(&v){o[0]=0xd1;o[1..3].copy_from_slice(&(v as i16).to_be_bytes());3}else{o[0]=0xd2;o[1..5].copy_from_slice(&v.to_be_bytes());5} }
fn decode_i32(b:&[u8])->Option<i32>{let t=*b.first()?;match t{0x00..=0x7f=>Some(t as i32),0xe0..=0xff=>Some((t as i8) as i32),0xd0 if b.len()>=2=>Some((b[1] as i8) as i32),0xd1 if b.len()>=3=>Some(i16::from_be_bytes([b[1],b[2]]) as i32),0xd2 if b.len()>=5=>Some(i32::from_be_bytes([b[1],b[2],b[3],b[4]])),_=>decode_u32(b).and_then(|v|i32::try_from(v).ok())}}
fn scalar_len(b:&[u8])->Option<usize>{match *b.first()?{0x00..=0x7f|0xe0..=0xff=>Some(1),0xcc|0xd0=>Some(2),0xcd|0xd1=>Some(3),0xce|0xd2=>Some(5),_=>None}}
fn decode_vec2(b:&[u8])->Option<Vec2fx>{if b.len()<3 || b.first().copied()!=Some(0x92){return None}let l=scalar_len(&b[1..])?;let x=decode_i32(&b[1..1+l])?;let y=decode_i32(&b[1+l..])?;Some(Vec2fx::new(x as i16,y as i16))}
