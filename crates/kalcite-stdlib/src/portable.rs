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
