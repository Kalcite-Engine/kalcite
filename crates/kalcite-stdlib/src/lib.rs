#![no_std]

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LibraryKind {
    Rust,
    Klc,
}

#[derive(Clone, Copy, Debug)]
pub struct Library {
    pub name: &'static str,
    pub kind: LibraryKind,
    pub source: Option<&'static str>,
}

pub const LIBRARIES: &[Library] = &[
    Library {
        name: "std.msgpack",
        kind: LibraryKind::Rust,
        source: None,
    },
    Library {
        name: "std.save",
        kind: LibraryKind::Rust,
        source: None,
    },
    Library {
        name: "std.math",
        kind: LibraryKind::Rust,
        source: None,
    },
    Library {
        name: "std.checksum",
        kind: LibraryKind::Rust,
        source: None,
    },
    Library {
        name: "std.bits",
        kind: LibraryKind::Rust,
        source: None,
    },
    Library {
        name: "std.fixed",
        kind: LibraryKind::Rust,
        source: None,
    },
    Library {
        name: "std.color",
        kind: LibraryKind::Rust,
        source: None,
    },
    Library {
        name: "std.easing",
        kind: LibraryKind::Klc,
        source: Some(include_str!("../klc/easing.klc")),
    },
];

pub fn find(name: &str) -> Option<&'static Library> {
    LIBRARIES.iter().find(|lib| lib.name == name)
}

pub const RUST_SOURCE: &str = include_str!("portable.rs");
