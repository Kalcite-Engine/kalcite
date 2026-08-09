#![no_std]
#[derive(Debug,Clone,Copy,PartialEq,Eq)]pub enum DecodeError{Malformed,OutputTooSmall}
/// Paires `(run:u8, value:u8)`. Un run nul est invalide.
pub fn decode_rle8(input:&[u8],out:&mut[u8])->Result<usize,DecodeError>{if input.len()%2!=0{return Err(DecodeError::Malformed)}let mut at=0;for p in input.chunks_exact(2){let n=p[0]as usize;if n==0{return Err(DecodeError::Malformed)}if at+n>out.len(){return Err(DecodeError::OutputTooSmall)}out[at..at+n].fill(p[1]);at+=n}Ok(at)}
/// RGB565: `(run:u8, lo:u8, hi:u8)`.
pub fn decode_rle565(input:&[u8],out:&mut[u16])->Result<usize,DecodeError>{if input.len()%3!=0{return Err(DecodeError::Malformed)}let mut at=0;for p in input.chunks_exact(3){let n=p[0]as usize;if n==0{return Err(DecodeError::Malformed)}if at+n>out.len(){return Err(DecodeError::OutputTooSmall)}out[at..at+n].fill(u16::from_le_bytes([p[1],p[2]]));at+=n}Ok(at)}
#[cfg(test)]mod tests{use super::*;#[test]fn rle(){let mut o=[0;5];assert_eq!(decode_rle8(&[3,7,2,9],&mut o),Ok(5));assert_eq!(o,[7,7,7,9,9])}}
