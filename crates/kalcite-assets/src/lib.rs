use std::{fs::File, io::BufReader, path::Path};

pub struct Sprite { pub w: u16, pub h: u16, pub rle: Vec<u8> }

pub fn rgb565(r: u8, g: u8, b: u8) -> u16 {
    ((r as u16 >> 3) << 11) | ((g as u16 >> 2) << 5) | (b as u16 >> 3)
}

pub fn rle565(pixels: &[u16]) -> Vec<u8> {
    let mut out = Vec::new(); let mut i = 0;
    while i < pixels.len() {
        let value = pixels[i]; let mut count = 1usize;
        while i + count < pixels.len() && pixels[i + count] == value && count < 255 { count += 1; }
        out.push(count as u8); out.extend(value.to_le_bytes()); i += count;
    }
    out
}

pub fn png(path: &Path) -> Result<Sprite, String> {
    let file = File::open(path).map_err(|e| e.to_string())?;
    let mut decoder = png::Decoder::new(BufReader::new(file));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    if info.width > u16::MAX as u32 || info.height > u16::MAX as u32 { return Err("PNG dimensions exceed KSP limits".into()); }
    let data = &buf[..info.buffer_size()];
    let mut pixels = Vec::with_capacity((info.width * info.height) as usize);
    match info.color_type {
        png::ColorType::Rgb => for px in data.chunks_exact(3) { pixels.push(rgb565(px[0], px[1], px[2])); },
        png::ColorType::Rgba => for px in data.chunks_exact(4) { pixels.push(if px[3] < 128 { 0 } else { rgb565(px[0], px[1], px[2]) }); },
        png::ColorType::Grayscale => for &v in data { pixels.push(rgb565(v, v, v)); },
        png::ColorType::GrayscaleAlpha => for px in data.chunks_exact(2) { pixels.push(if px[1] < 128 { 0 } else { rgb565(px[0], px[0], px[0]) }); },
        png::ColorType::Indexed => return Err("indexed PNG was not expanded by decoder".into()),
    }
    Ok(Sprite { w: info.width as u16, h: info.height as u16, rle: rle565(&pixels) })
}

pub fn tilemap_csv(s: &str) -> Result<Vec<u16>, String> {
    s.replace('\n', ",").split(',').filter(|x| !x.trim().is_empty())
        .map(|x| x.trim().parse().map_err(|_| "bad tile".into())).collect()
}

#[derive(Clone,Debug)]pub struct AssetEntry{pub name:String,pub id:u64,pub kind:u8,pub data:Vec<u8>}
pub fn asset_id(name:&str)->u64{let mut h=14695981039346656037u64;for b in name.bytes(){h^=b as u64;h=h.wrapping_mul(1099511628211);}h}
pub fn pack_dir(root:&Path)->Result<Vec<AssetEntry>,String>{let mut paths=Vec::new();collect(root,root,&mut paths).map_err(|e|e.to_string())?;paths.sort();let mut out=Vec::new();for(path,name)in paths{match path.extension().and_then(|x|x.to_str()).unwrap_or("").to_ascii_lowercase().as_str(){"png"=>{let s=png(&path)?;let mut d=Vec::with_capacity(4+s.rle.len());d.extend(s.w.to_le_bytes());d.extend(s.h.to_le_bytes());d.extend(s.rle);out.push(AssetEntry{name:name.clone(),id:asset_id(&name),kind:1,data:d});},"csv"=>{let src=std::fs::read_to_string(&path).map_err(|e|e.to_string())?;let tiles=tilemap_csv(&src)?;let mut d=Vec::with_capacity(tiles.len()*2);for t in tiles{d.extend(t.to_le_bytes())}out.push(AssetEntry{name:name.clone(),id:asset_id(&name),kind:2,data:d});},_=>{}}}Ok(out)}
pub fn encode_pack(entries:&[AssetEntry])->Vec<u8>{let mut o=Vec::new();o.extend(*b"KAP0");o.extend((entries.len()as u32).to_le_bytes());for e in entries{o.extend(e.id.to_le_bytes());o.push(e.kind);o.extend((e.name.len()as u16).to_le_bytes());o.extend((e.data.len()as u32).to_le_bytes());o.extend(e.name.as_bytes());o.extend(&e.data);}o}
fn collect(base:&Path,dir:&Path,out:&mut Vec<(std::path::PathBuf,String)>)->std::io::Result<()>{if !dir.exists(){return Ok(())}for e in std::fs::read_dir(dir)?{let p=e?.path();if p.is_dir(){collect(base,&p,out)?}else{let n=p.strip_prefix(base).unwrap_or(&p).to_string_lossy().replace('\\',"/");out.push((p,n));}}Ok(())}
#[cfg(test)]mod pack_tests{use super::*;#[test]fn ids_stable(){assert_eq!(asset_id("player.png"),asset_id("player.png"));assert_ne!(asset_id("a"),asset_id("b"));}}
