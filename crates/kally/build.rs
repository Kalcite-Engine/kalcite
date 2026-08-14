use std::{env, fs, path::PathBuf};

fn main() {
    let source = PathBuf::from("src/kally_core.klc");
    println!("cargo:rerun-if-changed={}", source.display());
    let source_text = fs::read_to_string(&source).expect("read Kally KLC core");
    let syntax = kalcite_syntax::parse(&source_text).expect("parse Kally KLC core");
    let hir = kalcite_hir::lower(&syntax).expect("lower Kally KLC core");
    let mir = kalcite_mir::lower(&hir);
    let emitted = kalcite_backend_rust::emit_library(
        &mir,
        "use crate::klc_runtime::{BoundedString, Text};\n\n",
    )
    .expect("emit Kally KLC core");
    fs::write(
        PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR")).join("kally_core.rs"),
        emitted,
    )
    .expect("write generated Kally KLC core");
}
