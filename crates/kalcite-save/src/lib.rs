use std::collections::BTreeMap;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Header {
    pub version: u32,
    pub schema: u32,
}
pub const MAGIC: [u8; 4] = *b"KSAV";
pub trait Migratable: Sized {
    const VERSION: u32;
    fn migrate(from: u32, data: &[u8]) -> Result<Self, String>;
}
pub fn schema_id(name: &str) -> u32 {
    let mut h = 2166136261u32;
    for b in name.bytes() {
        h ^= b as u32;
        h = h.wrapping_mul(16777619);
    }
    h
}
pub fn encode(version: u32, schema: u32, payload: &[u8]) -> Vec<u8> {
    let mut o = Vec::with_capacity(12 + payload.len());
    o.extend(MAGIC);
    o.extend(version.to_le_bytes());
    o.extend(schema.to_le_bytes());
    o.extend(payload);
    o
}
pub fn decode(data: &[u8]) -> Result<(Header, &[u8]), String> {
    if data.len() < 12 || data[..4] != MAGIC {
        return Err("invalid KSAV header".into());
    }
    Ok((
        Header {
            version: u32::from_le_bytes(data[4..8].try_into().unwrap()),
            schema: u32::from_le_bytes(data[8..12].try_into().unwrap()),
        },
        &data[12..],
    ))
}
pub fn load<T: Migratable>(expected_schema: u32, data: &[u8]) -> Result<T, String> {
    let (h, p) = decode(data)?;
    if h.schema != expected_schema {
        return Err("save schema mismatch".into());
    }
    if h.version > T::VERSION {
        return Err("save was created by a newer schema version".into());
    }
    T::migrate(h.version, p)
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Schema {
    pub name: String,
    pub version: u32,
    pub fields: BTreeMap<String, String>,
}
pub fn parse_schema(text: &str) -> Result<Schema, String> {
    let mut name = None;
    let mut version = None;
    let mut fields = BTreeMap::new();
    for (n, raw) in text.lines().enumerate() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let (k, v) = line
            .split_once('=')
            .ok_or_else(|| format!("schema line {}: expected key=value", n + 1))?;
        match k.trim() {
            "schema" => name = Some(v.trim().into()),
            "version" => {
                version = Some(
                    v.trim()
                        .parse()
                        .map_err(|_| format!("schema line {}: invalid version", n + 1))?,
                )
            }
            field => {
                let ty = v.trim();
                if !matches!(
                    ty,
                    "bool" | "u8" | "u16" | "u32" | "i8" | "i16" | "i32" | "fx8" | "string"
                ) {
                    return Err(format!("schema line {}: unsupported type `{ty}`", n + 1));
                }
                fields.insert(field.into(), ty.into());
            }
        }
    }
    Ok(Schema {
        name: name.ok_or("save schema name missing")?,
        version: version.ok_or("save schema version missing")?,
        fields,
    })
}

/// Generate the no-heap, schema-aware save adapter linked into game projects.
/// Each generated accessor reads/writes the complete typed record, so adding
/// fields remains deterministic and older versions fail explicitly.
pub fn emit_rust(text: &str) -> Result<String, String> {
    let schema = parse_schema(text)?;
    let id = schema_id(&schema.name);
    let mut fields = String::new();
    let mut defaults = String::new();
    let mut encode = String::new();
    let mut decode = String::new();
    let mut accessors = String::new();
    let mut offset = 12usize;
    for (name, ty) in &schema.fields {
        let (rust_ty, size, zero) = match ty.as_str() {
            "bool" => ("bool", 1, "false"),
            "u8" => ("u8", 1, "0"),
            "i8" => ("i8", 1, "0"),
            "u16" | "fx8" => ("u16", 2, "0"),
            "i16" => ("i16", 2, "0"),
            "u32" => ("u32", 4, "0"),
            "i32" => ("i32", 4, "0"),
            "string" => return Err("generated saves do not yet support string fields".into()),
            _ => return Err(format!("unsupported generated save type `{ty}`")),
        };
        fields.push_str(&format!("pub {name}:{rust_ty},"));
        defaults.push_str(&format!("{name}:{zero},"));
        if ty == "bool" {
            encode.push_str(&format!("out[{offset}]=state.{name} as u8;"));
            decode.push_str(&format!("{name}:data[{offset}]!=0,"));
        } else if size == 1 {
            encode.push_str(&format!("out[{offset}]=state.{name} as u8;"));
            decode.push_str(&format!("{name}:data[{offset}] as {rust_ty},"));
        } else {
            let end = offset + size;
            encode.push_str(&format!(
                "out[{offset}..{end}].copy_from_slice(&state.{name}.to_le_bytes());"
            ));
            decode.push_str(&format!(
                "{name}:{rust_ty}::from_le_bytes(data[{offset}..{end}].try_into().unwrap()),"
            ));
        }
        let setter = format!("set_{name}");
        accessors.push_str(&format!(
            "pub fn {setter}(slot:&str,value:{rust_ty})->bool{{let mut s=Self::load(slot).unwrap_or_default();s.{name}=value;Self::save(slot,&s)}}pub fn {name}(slot:&str,fallback:{rust_ty})->{rust_ty}{{Self::load(slot).map(|s|s.{name}).unwrap_or(fallback)}}"
        ));
        offset += size;
    }
    let template = "#[derive(Clone,Copy)]pub struct SaveState{__FIELDS__}impl Default for SaveState{fn default()->Self{Self{__DEFAULTS__}}}pub struct ProjectSave;impl ProjectSave{pub const VERSION:u32=__VERSION__;pub const SCHEMA:u32=__SCHEMA__;pub const SIZE:usize=__SIZE__;pub fn version()->u32{Self::VERSION}pub fn schema()->u32{Self::SCHEMA}pub fn compatible(version:u32,schema:u32)->bool{version<=Self::VERSION&&schema==Self::SCHEMA}pub fn save(slot:&str,state:&SaveState)->bool{let mut out=[0u8;__SIZE__];out[..4].copy_from_slice(b\"KSAV\");out[4..8].copy_from_slice(&Self::VERSION.to_le_bytes());out[8..12].copy_from_slice(&Self::SCHEMA.to_le_bytes());__ENCODE__crate::platform::Storage::write_bytes(slot,&out)}pub fn load(slot:&str)->Option<SaveState>{let mut data=[0u8;__SIZE__];if crate::platform::Storage::read_into(slot,&mut data)!=Self::SIZE||&data[..4]!=b\"KSAV\"||!Self::compatible(u32::from_le_bytes(data[4..8].try_into().ok()?),u32::from_le_bytes(data[8..12].try_into().ok()?)){return None;}Some(SaveState{__DECODE__})}pub fn valid(slot:&str)->bool{Self::load(slot).is_some()}__ACCESSORS__}";
    Ok(template
        .replace("__FIELDS__", &fields)
        .replace("__DEFAULTS__", &defaults)
        .replace("__VERSION__", &schema.version.to_string())
        .replace("__SCHEMA__", &id.to_string())
        .replace("__SIZE__", &offset.to_string())
        .replace("__ENCODE__", &encode)
        .replace("__DECODE__", &decode)
        .replace("__ACCESSORS__", &accessors))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roundtrip() {
        let b = encode(2, schema_id("Game.State"), b"abc");
        let (h, p) = decode(&b).unwrap();
        assert_eq!(h.version, 2);
        assert_eq!(p, b"abc");
    }
    #[test]
    fn schema() {
        assert_eq!(
            parse_schema("schema=Game.State\nversion=1\nscore=u32")
                .unwrap()
                .fields["score"],
            "u32"
        )
    }

    #[test]
    fn emits_typed_schema_adapter() {
        let rust = emit_rust("schema=Game.State\nversion=2\nscore=u32\nalive=bool").unwrap();
        assert!(rust.contains("pub score:u32"));
        assert!(rust.contains("pub fn set_score"));
        assert!(rust.contains("pub const VERSION:u32=2"));
    }
}
