use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

// Minimal allocation-free ABI used by the KLC module generated in build.rs.
// Rust owns filesystem/Git only; policy and lockfile syntax are KLC code.
mod klc_runtime {
    #[derive(Clone, Copy)]
    pub struct BoundedString<const N: usize> {
        pub len: u16,
        pub bytes: [u8; N],
    }
    impl<const N: usize> BoundedString<N> {
        pub fn from_str(value: &str) -> Self {
            let mut result = Self {
                len: 0,
                bytes: [0; N],
            };
            let count = value.len().min(N).min(u16::MAX as usize);
            result.bytes[..count].copy_from_slice(&value.as_bytes()[..count]);
            result.len = count as u16;
            result
        }
        #[inline]
        pub fn length(&self) -> u32 {
            self.len as u32
        }
        #[inline]
        pub fn byte_at(&self, index: u32) -> u8 {
            self.bytes
                .get(index as usize)
                .copied()
                .filter(|_| index < self.len as u32)
                .unwrap_or(0)
        }
    }
    pub struct Text;
    impl Text {
        #[inline]
        pub fn length<const N: usize>(value: BoundedString<N>) -> u32 {
            value.length()
        }
        #[inline]
        pub fn byte_at<const N: usize>(value: BoundedString<N>, index: u32) -> u8 {
            value.byte_at(index)
        }
    }
}

#[allow(dead_code, unused_mut, unused_parens)]
mod klc_core {
    include!(concat!(env!("OUT_DIR"), "/kally_core.rs"));
}
#[derive(Default)]
pub struct Lock {
    pub version: u32,
    pub packages: BTreeMap<String, Package>,
}
#[derive(Clone, Default)]
pub struct Package {
    pub source: String,
    /// The mutable Git branch or tag requested by the manifest/CLI. `revision`
    /// is always the immutable commit selected from this reference.
    pub reference: String,
    pub revision: String,
    pub checksum: String,
}
pub fn valid_name(name: &str) -> bool {
    klc_core::kally_valid_name(klc_runtime::BoundedString::<65>::from_str(name))
}
pub fn checksum(data: &[u8]) -> String {
    let mut h = 14695981039346656037u64;
    for b in data {
        h ^= *b as u64;
        h = h.wrapping_mul(1099511628211);
    }
    format!("{h:016x}")
}
fn hash_bytes(mut hash: u64, data: &[u8]) -> u64 {
    for byte in data {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(1099511628211);
    }
    hash
}

/// Hash a file or directory tree using normalized relative paths and sorted
/// traversal, so lockfiles remain identical across host filesystems.
pub fn checksum_path(path: &Path) -> Result<String, String> {
    let mut files = Vec::new();
    collect_files(path, path, &mut files)?;
    files.sort_by(|a, b| a.0.cmp(&b.0));
    let mut hash = 14695981039346656037u64;
    for (relative, file) in files {
        hash = hash_bytes(hash, relative.as_bytes());
        hash = hash_bytes(hash, &[0]);
        hash = hash_bytes(hash, &fs::read(file).map_err(|error| error.to_string())?);
    }
    Ok(format!("{hash:016x}"))
}

fn collect_files(root: &Path, path: &Path, out: &mut Vec<(String, PathBuf)>) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path).map_err(|error| error.to_string())?;
    if metadata.file_type().is_symlink() {
        return Err(format!(
            "package source may not contain symlink `{}`",
            path.display()
        ));
    }
    if metadata.is_file() {
        let relative = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        out.push((
            if relative.is_empty() {
                ".".into()
            } else {
                relative
            },
            path.into(),
        ));
        return Ok(());
    }
    if !metadata.is_dir() {
        return Err(format!("unsupported package source `{}`", path.display()));
    }
    let mut entries = fs::read_dir(path)
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        collect_files(root, &entry.path(), out)?;
    }
    Ok(())
}

pub fn materialize(source: &Path, destination: &Path) -> Result<(), String> {
    let parent = destination.parent().ok_or("package cache has no parent")?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let stage = parent.join(format!(
        ".{}.stage-{}",
        destination
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("package"),
        std::process::id()
    ));
    if stage.exists() {
        if stage.is_dir() {
            fs::remove_dir_all(&stage).map_err(|error| error.to_string())?;
        } else {
            fs::remove_file(&stage).map_err(|error| error.to_string())?;
        }
    }
    copy_tree(source, &stage)?;
    if destination.exists() {
        if destination.is_dir() {
            fs::remove_dir_all(destination).map_err(|error| error.to_string())?;
        } else {
            fs::remove_file(destination).map_err(|error| error.to_string())?;
        }
    }
    fs::rename(stage, destination).map_err(|error| error.to_string())
}

fn copy_tree(source: &Path, destination: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(source).map_err(|error| error.to_string())?;
    if metadata.file_type().is_symlink() {
        return Err(format!(
            "package source may not contain symlink `{}`",
            source.display()
        ));
    }
    if metadata.is_file() {
        fs::copy(source, destination).map_err(|error| error.to_string())?;
        return Ok(());
    }
    fs::create_dir_all(destination).map_err(|error| error.to_string())?;
    let mut entries = fs::read_dir(source)
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let from = entry.path();
        let to = destination.join(entry.file_name());
        let kind = entry.file_type().map_err(|error| error.to_string())?;
        if kind.is_symlink() {
            return Err(format!(
                "package source may not contain symlink `{}`",
                from.display()
            ));
        }
        if kind.is_dir() {
            copy_tree(&from, &to)?;
        } else if kind.is_file() {
            fs::copy(from, to).map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}
pub fn load(path: &Path) -> Result<Lock, String> {
    if !path.exists() {
        return Ok(Lock {
            version: 1,
            packages: BTreeMap::new(),
        });
    }
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut lock = Lock {
        version: 1,
        packages: BTreeMap::new(),
    };
    let mut current = None;
    for raw in text.lines() {
        let line = raw.trim();
        if line.len() > 512 {
            return Err("lockfile line exceeds Kally's 512-byte limit".into());
        }
        let kind =
            klc_core::kally_lock_line_kind(klc_runtime::BoundedString::<512>::from_str(line));
        if kind == 0 {
            continue;
        }
        if kind == 1 {
            let v = line
                .strip_prefix("version=")
                .ok_or("invalid lockfile version")?;
            lock.version = v.parse().map_err(|_| "invalid lockfile version")?;
            continue;
        }
        if kind == 2 {
            let name = line[1..line.len() - 1].to_string();
            if !valid_name(&name) {
                return Err(format!("invalid package name `{name}`"));
            }
            lock.packages.entry(name.clone()).or_default();
            current = Some(name);
            continue;
        }
        if !(3..=6).contains(&kind) {
            return Err(format!("invalid lockfile line: {line}"));
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(format!("invalid lockfile line: {line}"));
        };
        let Some(name) = current.as_ref() else {
            return Err("package property outside package section".into());
        };
        let package = lock.packages.get_mut(name).unwrap();
        match kind {
            3 if key.trim() == "source" => package.source = value.trim().to_string(),
            4 if key.trim() == "reference" => package.reference = value.trim().to_string(),
            5 if key.trim() == "revision" => package.revision = value.trim().to_string(),
            6 if key.trim() == "checksum" => package.checksum = value.trim().to_string(),
            _ => return Err(format!("invalid lockfile key: {}", key.trim())),
        }
    }
    Ok(lock)
}
pub fn save(path: &Path, lock: &Lock) -> Result<(), String> {
    let mut out = format!(
        "# Kally lockfile - generated, do not edit\nversion={}\n",
        lock.version.max(1)
    );
    for (name, p) in &lock.packages {
        if !valid_name(name) {
            return Err(format!("invalid package name `{name}`"));
        }
        out.push_str(&format!(
            "\n[{name}]\nsource={}\nreference={}\nrevision={}\nchecksum={}\n",
            p.source, p.reference, p.revision, p.checksum
        ));
    }
    fs::write(path, out).map_err(|e| e.to_string())
}
pub fn verify(lock: &Lock, cache: &Path) -> Result<(), String> {
    if !klc_core::kally_lock_version_supported(lock.version) {
        return Err(format!("unsupported lockfile version {}", lock.version));
    }
    for (name, p) in &lock.packages {
        if p.revision.is_empty() {
            return Err(format!("package `{name}` is not pinned to a revision"));
        }
        let path = cache.join(name);
        if path.exists() && !p.checksum.is_empty() {
            let got = checksum_path(&path)?;
            if got != p.checksum {
                return Err(format!("checksum mismatch for `{name}`"));
            }
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hash_stable() {
        assert_eq!(checksum(b"abc"), checksum(b"abc"));
    }

    #[test]
    fn directory_hash_and_materialization_are_deterministic() {
        let root = std::env::temp_dir().join(format!("kally-{}", std::process::id()));
        let source = root.join("source");
        let cache = root.join("cache/demo");
        fs::create_dir_all(source.join("scripts")).unwrap();
        fs::write(source.join("scripts/a.klc"), b"class A {}").unwrap();
        let before = checksum_path(&source).unwrap();
        materialize(&source, &cache).unwrap();
        assert_eq!(before, checksum_path(&cache).unwrap());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn lockfile_preserves_git_reference_and_commit() {
        let root = std::env::temp_dir().join(format!("kally-lock-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let path = root.join("kally.lock");
        let mut lock = Lock::default();
        lock.packages.insert(
            "ui".into(),
            Package {
                source: "git:https://example.invalid/kalcite-packages.git#packages/ui".into(),
                reference: "v0.3.0".into(),
                revision: "0123456789abcdef".into(),
                checksum: "deadbeef".into(),
            },
        );
        save(&path, &lock).unwrap();
        let restored = load(&path).unwrap();
        let package = &restored.packages["ui"];
        assert_eq!(package.reference, "v0.3.0");
        assert_eq!(package.revision, "0123456789abcdef");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn klc_core_rejects_invalid_package_names_and_lock_keys() {
        assert!(valid_name("package-42"));
        assert!(!valid_name("package/name"));
        assert!(!valid_name(&"a".repeat(65)));

        let root = std::env::temp_dir().join(format!("kally-klc-core-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let path = root.join("kally.lock");
        fs::write(&path, "version=1\n[demo]\nunknown=value\n").unwrap();
        assert!(load(&path).is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
