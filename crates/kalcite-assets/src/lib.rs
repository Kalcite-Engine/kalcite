use std::{collections::BTreeMap, fs::File, io::BufReader, path::Path};

pub const KIND_SPRITE: u8 = 1;
pub const KIND_TILEMAP: u8 = 2;
pub const KIND_SPRITESHEET: u8 = 3;

pub struct Sprite {
    pub w: u16,
    pub h: u16,
    pub rle: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpriteSheet {
    pub image: String,
    pub frame_w: u16,
    pub frame_h: u16,
}

pub fn rgb565(r: u8, g: u8, b: u8) -> u16 {
    ((r as u16 >> 3) << 11) | ((g as u16 >> 2) << 5) | (b as u16 >> 3)
}

pub fn rle565(pixels: &[u16]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < pixels.len() {
        let value = pixels[i];
        let mut count = 1usize;
        while i + count < pixels.len() && pixels[i + count] == value && count < 255 {
            count += 1;
        }
        out.push(count as u8);
        out.extend(value.to_le_bytes());
        i += count;
    }
    out
}

/// Encode row-bounded RGB565 runs as `(count, transparent, lo, hi)`.
pub fn rle565_rows(pixels: &[Option<u16>], width: usize) -> Vec<u8> {
    let mut out = Vec::new();
    if width == 0 {
        return out;
    }
    for row in pixels.chunks(width) {
        let mut at = 0;
        while at < row.len() {
            let value = row[at];
            let mut count = 1usize;
            while at + count < row.len() && row[at + count] == value && count < 255 {
                count += 1;
            }
            out.push(count as u8);
            out.push(u8::from(value.is_none()));
            out.extend(value.unwrap_or(0).to_le_bytes());
            at += count;
        }
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
    if info.width > u16::MAX as u32 || info.height > u16::MAX as u32 {
        return Err("PNG dimensions exceed KSP limits".into());
    }
    let data = &buf[..info.buffer_size()];
    let mut pixels = Vec::with_capacity((info.width * info.height) as usize);
    match info.color_type {
        png::ColorType::Rgb => {
            for px in data.chunks_exact(3) {
                pixels.push(Some(rgb565(px[0], px[1], px[2])));
            }
        }
        png::ColorType::Rgba => {
            for px in data.chunks_exact(4) {
                pixels.push(if px[3] < 128 {
                    None
                } else {
                    Some(rgb565(px[0], px[1], px[2]))
                });
            }
        }
        png::ColorType::Grayscale => {
            for &v in data {
                pixels.push(Some(rgb565(v, v, v)));
            }
        }
        png::ColorType::GrayscaleAlpha => {
            for px in data.chunks_exact(2) {
                pixels.push(if px[1] < 128 {
                    None
                } else {
                    Some(rgb565(px[0], px[0], px[0]))
                });
            }
        }
        png::ColorType::Indexed => return Err("indexed PNG was not expanded by decoder".into()),
    }
    Ok(Sprite {
        w: info.width as u16,
        h: info.height as u16,
        rle: rle565_rows(&pixels, info.width as usize),
    })
}

pub fn spritesheet(text: &str) -> Result<SpriteSheet, String> {
    let mut image = None;
    let mut frame_w = None;
    let mut frame_h = None;
    for (line_number, raw) in text.lines().enumerate() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| format!("spritesheet line {}: expected key=value", line_number + 1))?;
        match key.trim() {
            "image" => image = Some(value.trim().trim_matches('"').to_string()),
            "frame_w" => {
                frame_w = Some(value.trim().parse::<u16>().map_err(|_| {
                    format!("spritesheet line {}: invalid frame_w", line_number + 1)
                })?)
            }
            "frame_h" => {
                frame_h = Some(value.trim().parse::<u16>().map_err(|_| {
                    format!("spritesheet line {}: invalid frame_h", line_number + 1)
                })?)
            }
            other => {
                return Err(format!(
                    "spritesheet line {}: unknown key `{other}`",
                    line_number + 1
                ));
            }
        }
    }
    let sheet = SpriteSheet {
        image: image.ok_or("spritesheet image missing")?,
        frame_w: frame_w.ok_or("spritesheet frame_w missing")?,
        frame_h: frame_h.ok_or("spritesheet frame_h missing")?,
    };
    if sheet.frame_w == 0 || sheet.frame_h == 0 {
        return Err("spritesheet frame dimensions must be non-zero".into());
    }
    Ok(sheet)
}

pub fn tilemap_csv(s: &str) -> Result<Vec<u16>, String> {
    s.replace('\n', ",")
        .split(',')
        .filter(|x| !x.trim().is_empty())
        .map(|x| x.trim().parse().map_err(|_| "bad tile".into()))
        .collect()
}

fn tilemap_size(source: &str) -> Result<(u16, u16), String> {
    let rows = source
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| {
            line.split(',')
                .filter(|cell| !cell.trim().is_empty())
                .count()
        })
        .collect::<Vec<_>>();
    let Some(&width) = rows.first() else {
        return Err("tilemap is empty".into());
    };
    if width == 0 || rows.iter().any(|row| *row != width) {
        return Err("tilemap rows must have equal non-zero width".into());
    }
    Ok((
        u16::try_from(width).map_err(|_| "tilemap width exceeds u16")?,
        u16::try_from(rows.len()).map_err(|_| "tilemap height exceeds u16")?,
    ))
}

#[derive(Clone, Debug)]
pub struct AssetEntry {
    pub name: String,
    pub id: u64,
    pub kind: u8,
    pub data: Vec<u8>,
}
pub fn asset_id(name: &str) -> u64 {
    let mut h = 14695981039346656037u64;
    for b in name.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(1099511628211);
    }
    h
}
pub fn pack_dir(root: &Path) -> Result<Vec<AssetEntry>, String> {
    let mut paths = Vec::new();
    collect(root, root, &mut paths).map_err(|e| e.to_string())?;
    paths.sort();
    let mut out = Vec::new();
    for (path, name) in paths {
        match path
            .extension()
            .and_then(|x| x.to_str())
            .unwrap_or("")
            .to_ascii_lowercase()
            .as_str()
        {
            "png" => {
                let s = png(&path)?;
                let mut d = Vec::with_capacity(4 + s.rle.len());
                d.extend(s.w.to_le_bytes());
                d.extend(s.h.to_le_bytes());
                d.extend(s.rle);
                out.push(AssetEntry {
                    name: name.clone(),
                    id: asset_id(&name),
                    kind: KIND_SPRITE,
                    data: d,
                });
            }
            "csv" => {
                let src = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
                let (width, height) = tilemap_size(&src)?;
                let tiles = tilemap_csv(&src)?;
                let mut d = Vec::with_capacity(4 + tiles.len() * 2);
                d.extend(width.to_le_bytes());
                d.extend(height.to_le_bytes());
                for t in tiles {
                    d.extend(t.to_le_bytes())
                }
                out.push(AssetEntry {
                    name: name.clone(),
                    id: asset_id(&name),
                    kind: KIND_TILEMAP,
                    data: d,
                });
            }
            "ksheet" => {
                let source = std::fs::read_to_string(&path).map_err(|error| error.to_string())?;
                let sheet = spritesheet(&source)?;
                let mut data = Vec::with_capacity(12);
                data.extend(asset_id(&sheet.image).to_le_bytes());
                data.extend(sheet.frame_w.to_le_bytes());
                data.extend(sheet.frame_h.to_le_bytes());
                out.push(AssetEntry {
                    name: name.clone(),
                    id: asset_id(&name),
                    kind: KIND_SPRITESHEET,
                    data,
                });
            }
            _ => {}
        }
    }
    Ok(out)
}
pub fn encode_pack(entries: &[AssetEntry]) -> Vec<u8> {
    let mut payload_indexes = BTreeMap::<Vec<u8>, u16>::new();
    let mut payloads = Vec::<&[u8]>::new();
    let mut indexes = Vec::with_capacity(entries.len());
    for entry in entries {
        let index = if let Some(index) = payload_indexes.get(&entry.data) {
            *index
        } else {
            let index = payloads.len() as u16;
            payload_indexes.insert(entry.data.clone(), index);
            payloads.push(&entry.data);
            index
        };
        indexes.push(index);
    }
    let mut out = Vec::new();
    out.extend(*b"KAP1");
    out.extend((entries.len() as u16).to_le_bytes());
    out.extend((payloads.len() as u16).to_le_bytes());
    for (entry, payload) in entries.iter().zip(indexes) {
        out.extend(entry.id.to_le_bytes());
        out.push(entry.kind);
        out.extend((entry.name.len() as u16).to_le_bytes());
        out.extend(payload.to_le_bytes());
        out.extend(entry.name.as_bytes());
    }
    for payload in payloads {
        out.extend((payload.len() as u32).to_le_bytes());
        out.extend(payload);
    }
    out
}
fn collect(
    base: &Path,
    dir: &Path,
    out: &mut Vec<(std::path::PathBuf, String)>,
) -> std::io::Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    for e in std::fs::read_dir(dir)? {
        let p = e?.path();
        if p.is_dir() {
            collect(base, &p, out)?
        } else {
            let n = p
                .strip_prefix(base)
                .unwrap_or(&p)
                .to_string_lossy()
                .replace('\\', "/");
            out.push((p, n));
        }
    }
    Ok(())
}
#[cfg(test)]
mod pack_tests {
    use super::*;
    #[test]
    fn ids_stable() {
        assert_eq!(asset_id("player.png"), asset_id("player.png"));
        assert_ne!(asset_id("a"), asset_id("b"));
    }

    #[test]
    fn alpha_runs_are_explicit_and_row_bounded() {
        let runs = rle565_rows(&[Some(0), Some(0), Some(0), Some(0), None, None], 3);
        assert_eq!(runs, [3, 0, 0, 0, 1, 0, 0, 0, 2, 1, 0, 0]);
    }

    #[test]
    fn pack_deduplicates_equal_payloads() {
        let entries = vec![
            AssetEntry {
                name: "a".into(),
                id: 1,
                kind: 1,
                data: vec![7, 8],
            },
            AssetEntry {
                name: "b".into(),
                id: 2,
                kind: 1,
                data: vec![7, 8],
            },
        ];
        let pack = encode_pack(&entries);
        assert_eq!(&pack[..4], b"KAP1");
        assert_eq!(u16::from_le_bytes([pack[6], pack[7]]), 1);
    }

    #[test]
    fn parses_spritesheet_metadata() {
        assert_eq!(
            spritesheet("image=hero.png\nframe_w=8\nframe_h=12\n").unwrap(),
            SpriteSheet {
                image: "hero.png".into(),
                frame_w: 8,
                frame_h: 12
            }
        );
    }
}
