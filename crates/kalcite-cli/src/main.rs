use std::{env, fs, path::{Path, PathBuf}, process::ExitCode};
use kalcite_linter::{has_errors, lint, Severity};
use kalcite_object::Target;
use kalcite_project::{discover, find_root, init_project, load_manifest, validate};

fn usage() {
    eprintln!("usage:\n  kalcite init [DIR] [--name NAME]\n  kalcite project-check [DIR]\n  kalcite project-build [DIR] [--target portable|numworks|desktop|web]\n  kalcite build-app FILE.klc --target numworks [-o GAME.nwa] [--name NAME] [--no-build]\n  kalcite build-nwa FILE.klc [-o GAME.nwa] [--name NAME] [--no-build] [--install]\n  kalcite doctor numworks\n  kalcite libs\n  kalcite scene-check FILE.kscn\n  kalcite asset-png FILE.png [-o FILE.ksp]\n  kalcite package-lock [DIR]\n  kalcite test [DIR]\n  kalcite run FILE.klc [--name NAME] [--scale N] [--fps N] [--screenshot FILE.ppm]\n  kalcite check FILE.klc\n  kalcite lint FILE.klc\n  kalcite emit-mir FILE.klc\n  kalcite emit-rust FILE.klc\n  kalcite build FILE.klc [-o FILE.kco] [--target portable|numworks|desktop|web]");
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let Some(command) = args.get(1).map(String::as_str) else { usage(); return ExitCode::FAILURE; };
    match command {
        "init" => init_command(&args[2..]),
        "project-check" => project_command(&args[2..], false),
        "project-build" => project_command(&args[2..], true),
        "build-nwa" => build_nwa_command(&args[2..]),
        "build-app" => build_app_command(&args[2..]),
        "doctor" => doctor_command(&args[2..]),
        "libs" => libs_command(),
        "scene-check" => scene_check_command(&args[2..]),
        "asset-png" => asset_png_command(&args[2..]),
        "package-lock" => package_lock_command(&args[2..]),
        "test" => test_command(&args[2..]),
        "run" => run_command(&args[2..]),
        _ => file_command(command, &args[2..]),
    }
}


fn build_app_command(args: &[String]) -> ExitCode {
    let target = args.windows(2).find(|w| w[0] == "--target").map(|w| w[1].as_str()).unwrap_or("numworks");
    let filtered: Vec<String> = args.iter().enumerate().filter_map(|(i, x)| {
        if x == "--target" || (i > 0 && args[i-1] == "--target") { None } else { Some(x.clone()) }
    }).collect();
    match target {
        "numworks" => build_nwa_command(&filtered),
        "desktop" => build_desktop_command(&filtered, false, &[]),
        other => { eprintln!("native backend `{other}` is not implemented yet; available: numworks, desktop"); ExitCode::FAILURE }
    }
}

fn run_command(args: &[String]) -> ExitCode {
    let mut build_args = Vec::new();
    let mut runner_args = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--scale" | "--fps" | "--screenshot" if i + 1 < args.len() => {
                runner_args.push(args[i].clone());
                runner_args.push(args[i + 1].clone());
                i += 2;
            }
            "--headless" => { runner_args.push(args[i].clone()); i += 1; }
            "--frames" if i + 1 < args.len() => {
                runner_args.push(args[i].clone());
                runner_args.push(args[i + 1].clone());
                i += 2;
            }
            _ => { build_args.push(args[i].clone()); i += 1; }
        }
    }
    build_desktop_command(&build_args, true, &runner_args)
}

fn build_desktop_command(args: &[String], run: bool, runner_args: &[String]) -> ExitCode {
    let Some(input_arg) = args.first() else { usage(); return ExitCode::FAILURE; };
    let input = PathBuf::from(input_arg);
    let source = match fs::read_to_string(&input) { Ok(v) => v, Err(e) => { eprintln!("{}: {e}", input.display()); return ExitCode::FAILURE; } };
    let mut output = input.with_extension(if cfg!(windows) { "exe" } else { "desktop" });
    let mut app_name = input.file_stem().and_then(|x| x.to_str()).unwrap_or("Kalcite").to_string();
    let mut no_build = false; let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-o" if i+1 < args.len() => { output = PathBuf::from(&args[i+1]); i += 2; }
            "--name" if i+1 < args.len() => { app_name = args[i+1].clone(); i += 2; }
            "--no-build" => { no_build = true; i += 1; }
            other => { eprintln!("unknown option `{other}`"); return ExitCode::FAILURE; }
        }
    }
    let generated_root = PathBuf::from(".kalcite/desktop").join(input.file_stem().unwrap_or_default());
    if let Err(e) = kalcite_compiler::emit_desktop_project(&source, &app_name, &generated_root) {
        eprintln!("desktop project generation failed: {e}"); return ExitCode::FAILURE;
    }
    println!("generated desktop project in {}", generated_root.display());
    if no_build { return ExitCode::SUCCESS; }
    let status = std::process::Command::new("cargo").current_dir(&generated_root).args(["build","--release"]).status();
    if !matches!(status, Ok(s) if s.success()) { eprintln!("desktop build failed"); return ExitCode::FAILURE; }
    let exe_name = if cfg!(windows) { "kalcite-game-desktop.exe" } else { "kalcite-game-desktop" };
    let built = generated_root.join("target/release").join(exe_name);
    if !built.exists() { eprintln!("expected desktop executable not found: {}", built.display()); return ExitCode::FAILURE; }
    if let Some(parent)=output.parent(){let _=fs::create_dir_all(parent);} if let Err(e)=fs::copy(&built,&output){eprintln!("{}: {e}",output.display());return ExitCode::FAILURE;}
    println!("built {}", output.display());
    if run {
        let status=std::process::Command::new(&output).args(runner_args).status();
        if !matches!(status,Ok(s) if s.success()){eprintln!("desktop game exited with an error");return ExitCode::FAILURE;}
    }
    ExitCode::SUCCESS
}

fn build_nwa_command(args: &[String]) -> ExitCode {
    let Some(input_arg) = args.first() else { usage(); return ExitCode::FAILURE; };
    let input = PathBuf::from(input_arg);
    let source = match fs::read_to_string(&input) {
        Ok(source) => source,
        Err(error) => { eprintln!("{}: {error}", input.display()); return ExitCode::FAILURE; }
    };

    let mut output = input.with_extension("nwa");
    let mut app_name = input.file_stem().and_then(|x| x.to_str()).unwrap_or("Kalcite").to_string();
    let mut no_build = false;
    let mut install = false;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-o" if i + 1 < args.len() => { output = PathBuf::from(&args[i + 1]); i += 2; }
            "--name" if i + 1 < args.len() => { app_name = args[i + 1].clone(); i += 2; }
            "--no-build" => { no_build = true; i += 1; }
            "--install" => { install = true; i += 1; }
            other => { eprintln!("unknown option `{other}`"); return ExitCode::FAILURE; }
        }
    }

    let generated_root = PathBuf::from(".kalcite/numworks").join(input.file_stem().unwrap_or_default());
    if let Err(error) = kalcite_compiler::emit_numworks_project(&source, &app_name, &generated_root) {
        eprintln!("NumWorks project generation failed: {error}");
        return ExitCode::FAILURE;
    }
    println!("generated official-style EADK project in {}", generated_root.display());
    if no_build {
        println!("build it with: cd {} && cargo build --release", generated_root.display());
        return ExitCode::SUCCESS;
    }

    if let Err(error) = ensure_numworks_toolchain() {
        eprintln!("NumWorks Rust toolchain error: {error}");
        eprintln!("hint: run `kalcite doctor numworks`");
        return ExitCode::FAILURE;
    }

    if let Err(error) = build_numworks_project(&generated_root) {
        eprintln!("native NumWorks build failed: {error}");
        eprintln!("generated project kept at {}", generated_root.display());
        eprintln!("hint: run `kalcite doctor numworks`");
        return ExitCode::FAILURE;
    }

    let elf = generated_root.join("target/thumbv7em-none-eabihf/release/kalcite-game");
    if !elf.exists() {
        eprintln!("expected NumWorks ELF not found: {}", elf.display());
        return ExitCode::FAILURE;
    }
    if let Err(error) = validate_nwa_elf(&elf) {
        eprintln!("generated NumWorks image is invalid: {error}");
        return ExitCode::FAILURE;
    }
    if let Some(parent) = output.parent() { let _ = fs::create_dir_all(parent); }
    if let Err(error) = fs::copy(&elf, &output) {
        eprintln!("{}: {error}", output.display());
        return ExitCode::FAILURE;
    }
    let size = fs::metadata(&output).map(|m| m.len()).unwrap_or(0);
    println!("built {} ({} bytes)", output.display(), size);

    if install {
        println!("installing {} with nwlink...", output.display());
        match install_nwa(&output) {
            Ok(()) => println!("installed on NumWorks"),
            Err(error) => { eprintln!("installation failed: {error}"); return ExitCode::FAILURE; }
        }
    }
    ExitCode::SUCCESS
}

fn scene_check_command(args: &[String]) -> ExitCode {
    let Some(path) = args.first() else { eprintln!("usage: kalcite scene-check FILE.kscn"); return ExitCode::FAILURE; };
    match kalcite_scene::load(Path::new(path)) {
        Ok(scene) => { println!("ok: scene `{}`: {} nodes, {} static signals", scene.name, scene.nodes.len(), scene.signals.len()); ExitCode::SUCCESS }
        Err(e) => { eprintln!("{path}: {e}"); ExitCode::FAILURE }
    }
}

fn asset_png_command(args: &[String]) -> ExitCode {
    let Some(path) = args.first() else { eprintln!("usage: kalcite asset-png FILE.png [-o FILE.ksp]"); return ExitCode::FAILURE; };
    let mut output = PathBuf::from(path).with_extension("ksp");
    let mut i=1; while i<args.len(){ match args[i].as_str(){ "-o" if i+1<args.len()=>{output=PathBuf::from(&args[i+1]);i+=2}, x=>{eprintln!("unknown option `{x}`");return ExitCode::FAILURE;} } }
    let sprite=match kalcite_assets::png(Path::new(path)){Ok(v)=>v,Err(e)=>{eprintln!("{path}: {e}");return ExitCode::FAILURE;}};
    let mut bytes=Vec::with_capacity(12+sprite.rle.len()); bytes.extend_from_slice(b"KSP1"); bytes.extend_from_slice(&sprite.w.to_le_bytes()); bytes.extend_from_slice(&sprite.h.to_le_bytes()); bytes.extend_from_slice(&(sprite.rle.len() as u32).to_le_bytes()); bytes.extend_from_slice(&sprite.rle);
    if let Err(e)=fs::write(&output,bytes){eprintln!("{}: {e}",output.display());return ExitCode::FAILURE;} println!("compiled {}x{} RGB565/RLE sprite -> {}",sprite.w,sprite.h,output.display()); ExitCode::SUCCESS
}

fn package_lock_command(args: &[String]) -> ExitCode {
    let root=args.first().map(PathBuf::from).unwrap_or_else(||PathBuf::from(".")); let path=root.join("kalcite.lock");
    let lock=match kalcite_package::load(&path){Ok(v)=>v,Err(e)=>{eprintln!("{}: {e}",path.display());return ExitCode::FAILURE;}};
    if let Err(e)=kalcite_package::save(&path,&lock){eprintln!("{}: {e}",path.display());return ExitCode::FAILURE;} println!("locked {} packages in {}",lock.packages.len(),path.display()); ExitCode::SUCCESS
}

fn test_command(args: &[String]) -> ExitCode {
    let dir=args.first().map(PathBuf::from).unwrap_or_else(||PathBuf::from("tests/klc"));
    let cases=match kalcite_test_runner::discover(&dir){Ok(v)=>v,Err(e)=>{eprintln!("{}: {e}",dir.display());return ExitCode::FAILURE;}}; let mut failed=0;
    for case in &cases { let src=match fs::read_to_string(case){Ok(v)=>v,Err(e)=>{eprintln!("FAIL {case}: {e}");failed+=1;continue}}; match kalcite_compiler::check(&src){Ok(_)=>println!("PASS {case}"),Err(e)=>{eprintln!("FAIL {case}: {e}");failed+=1}} }
    println!("{} tests, {} failed",cases.len(),failed); if failed==0{ExitCode::SUCCESS}else{ExitCode::FAILURE}
}

fn libs_command() -> ExitCode {
    println!("Kalcite bundled libraries:");
    for lib in kalcite_stdlib::LIBRARIES {
        let kind=match lib.kind { kalcite_stdlib::LibraryKind::Rust=>"rust/no_std", kalcite_stdlib::LibraryKind::Klc=>"klc" };
        println!("  use {};  [{}]",lib.name,kind);
    }
    ExitCode::SUCCESS
}

fn doctor_command(args: &[String]) -> ExitCode {
    if args.first().map(String::as_str) != Some("numworks") {
        eprintln!("usage: kalcite doctor numworks");
        return ExitCode::FAILURE;
    }
    println!("Kalcite NumWorks doctor");
    if let Err(error) = ensure_numworks_toolchain() {
        eprintln!("[FAIL] Rust ARM toolchain: {error}");
        return ExitCode::FAILURE;
    }
    println!("[ OK ] Rust stable + thumbv7em-none-eabihf");

    let root = PathBuf::from(".kalcite/doctor/numworks");
    if let Err(error) = fs::create_dir_all(root.join("src")) { eprintln!("[FAIL] doctor directory: {error}"); return ExitCode::FAILURE; }
    if let Err(error) = fs::write(root.join("Cargo.toml"), "[package]\nname=\"kalcite-numworks-doctor\"\nversion=\"0.0.0\"\nedition=\"2021\"\n\n[workspace]\n") { eprintln!("[FAIL] doctor manifest: {error}"); return ExitCode::FAILURE; }
    if let Err(error) = fs::write(root.join("src/lib.rs"), "#![no_std]\npub fn probe(x:u32)->u32{x.wrapping_add(1)}\n") { eprintln!("[FAIL] doctor source: {error}"); return ExitCode::FAILURE; }
    match cargo_for_numworks(&root, &["check", "--target", "thumbv7em-none-eabihf"]) {
        Ok(()) => println!("[ OK ] no_std/core probe compiled for ARM"),
        Err(error) => { eprintln!("[FAIL] ARM core probe: {error}"); return ExitCode::FAILURE; }
    }

    match std::process::Command::new("node").arg("--version").output() {
        Ok(output) if output.status.success() => println!("[ OK ] Node.js {}", String::from_utf8_lossy(&output.stdout).trim()),
        _ => println!("[WARN] Node.js not found; nwlink will require Node/npm"),
    }
    match std::process::Command::new("npx").arg("--version").output() {
        Ok(output) if output.status.success() => println!("[ OK ] npx {}", String::from_utf8_lossy(&output.stdout).trim()),
        _ => println!("[WARN] npx not found; icon conversion/install will not work"),
    }
    println!("doctor completed");
    ExitCode::SUCCESS
}

fn ensure_numworks_toolchain() -> Result<(), String> {
    let status = std::process::Command::new("rustup")
        .args(["toolchain", "install", "stable", "--profile", "minimal", "--target", "thumbv7em-none-eabihf"])
        .status().map_err(|e| e.to_string())?;
    if !status.success() { return Err("rustup failed to install stable + thumbv7em-none-eabihf".into()); }

    let rustc = rustup_which("stable", "rustc")?;
    let target_libdir = rustc_print(&rustc, &["--target", "thumbv7em-none-eabihf", "--print", "target-libdir"])?;
    let path = PathBuf::from(&target_libdir);
    let has_core = path.is_dir() && fs::read_dir(&path).map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .any(|entry| entry.file_name().to_string_lossy().starts_with("libcore-"));
    if !has_core {
        let status = std::process::Command::new("rustup")
            .args(["target", "add", "--toolchain", "stable", "thumbv7em-none-eabihf"])
            .status().map_err(|e| e.to_string())?;
        if !status.success() { return Err("thumbv7em-none-eabihf standard library is missing".into()); }
    }
    Ok(())
}

fn build_numworks_project(root: &Path) -> Result<(), String> {
    cargo_for_numworks(root, &["build", "--release"])
}

fn cargo_for_numworks(root: &Path, args: &[&str]) -> Result<(), String> {
    let cargo = rustup_which("stable", "cargo")?;
    let rustc = rustup_which("stable", "rustc")?;
    let toolchain_bin = rustc.parent().ok_or("invalid rustc path")?;
    let mut paths = vec![toolchain_bin.to_path_buf()];
    if let Some(existing) = env::var_os("PATH") { paths.extend(env::split_paths(&existing)); }
    let path = env::join_paths(paths).map_err(|e| e.to_string())?;

    let status = std::process::Command::new(&cargo)
        .current_dir(root)
        .args(args)
        .env("RUSTC", &rustc)
        .env("RUSTUP_TOOLCHAIN", "stable")
        .env("PATH", path)
        .env_remove("RUSTFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .env_remove("RUSTC_WRAPPER")
        .env_remove("CARGO_BUILD_RUSTC")
        .env_remove("CARGO_BUILD_RUSTC_WRAPPER")
        .env_remove("CARGO_TARGET_THUMBV7EM_NONE_EABIHF_RUSTFLAGS")
        .status().map_err(|e| e.to_string())?;
    if status.success() { Ok(()) } else { Err(format!("Cargo exited with {status}")) }
}

fn validate_nwa_elf(path: &Path) -> Result<(), String> {
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    if !bytes.starts_with(b"\x7fELF") { return Err("output is not an ELF file".into()); }
    for section in [b".rodata.eadk_app_name".as_slice(), b".rodata.eadk_api_level".as_slice(), b".rodata.eadk_app_icon".as_slice()] {
        if !bytes.windows(section.len()).any(|window| window == section) {
            return Err(format!("required EADK section `{}` is missing", String::from_utf8_lossy(section)));
        }
    }
    Ok(())
}

fn install_nwa(path: &Path) -> Result<(), String> {
    let status = std::process::Command::new("npx")
        .args(["--yes", "--package=node@18.20.8", "--package=nwlink@0.0.19", "--", "nwlink", "install-nwa"])
        .arg(path)
        .status().map_err(|e| e.to_string())?;
    if status.success() { Ok(()) } else { Err(format!("nwlink exited with {status}")) }
}

fn rustc_print(rustc: &Path, args: &[&str]) -> Result<String, String> {
    let output = std::process::Command::new(rustc).args(args).output().map_err(|e| e.to_string())?;
    if !output.status.success() { return Err(String::from_utf8_lossy(&output.stderr).trim().to_string()); }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() { Err("rustc returned an empty path".into()) } else { Ok(value) }
}

fn rustup_which(toolchain: &str, binary: &str) -> Result<PathBuf, String> {
    let output = std::process::Command::new("rustup")
        .args(["which", "--toolchain", toolchain, binary])
        .output().map_err(|error| error.to_string())?;
    if !output.status.success() { return Err(String::from_utf8_lossy(&output.stderr).trim().to_string()); }
    let path = String::from_utf8(output.stdout).map_err(|error| error.to_string())?.trim().to_string();
    if path.is_empty() { Err(format!("rustup returned an empty path for {binary}")) } else { Ok(PathBuf::from(path)) }
}

fn init_command(args: &[String]) -> ExitCode {
    let mut root = PathBuf::from(".");
    let mut name = "MyGame".to_string();
    let mut i = 0;
    if args.first().is_some_and(|x| !x.starts_with('-')) { root = PathBuf::from(&args[0]); i = 1; }
    while i < args.len() {
        match args[i].as_str() {
            "--name" if i + 1 < args.len() => { name = args[i + 1].clone(); i += 2; }
            other => { eprintln!("unknown option `{other}`"); return ExitCode::FAILURE; }
        }
    }
    match init_project(&root, &name) {
        Ok(()) => { println!("created Kalcite project `{name}` in {}", root.display()); println!("next: cd {} && kalcite project-check", root.display()); ExitCode::SUCCESS }
        Err(error) => { eprintln!("project creation failed: {error:?}"); ExitCode::FAILURE }
    }
}

fn project_command(args: &[String], build: bool) -> ExitCode {
    let start = args.first().filter(|x| !x.starts_with('-')).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    let Some(root) = find_root(&start) else { eprintln!("no kalcite.toml found from {}", start.display()); return ExitCode::FAILURE; };
    let manifest = match load_manifest(&root) { Ok(v) => v, Err(e) => { eprintln!("manifest error: {e:?}"); return ExitCode::FAILURE; } };
    let index = match discover(&root, &manifest) { Ok(v) => v, Err(e) => { eprintln!("project scan failed: {e:?}"); return ExitCode::FAILURE; } };
    let diagnostics = validate(&index);
    for d in &diagnostics {
        let level = match d.severity { Severity::Warning => "warning", Severity::Error => "error" };
        eprintln!("{}: {level}[{}]: {}", relative(&root, &d.path).display(), d.code, d.message);
    }
    if diagnostics.iter().any(|d| d.severity == Severity::Error) { return ExitCode::FAILURE; }
    let scene_path = root.join(&manifest.entry_scene);
    let scene = match kalcite_scene::load(&scene_path) { Ok(v) => v, Err(e) => { eprintln!("{}: scene error: {e}", relative(&root,&scene_path).display()); return ExitCode::FAILURE; } };
    let assets = match kalcite_assets::pack_dir(&root.join(&manifest.assets_dir)) { Ok(v)=>v, Err(e)=>{eprintln!("asset pipeline: {e}");return ExitCode::FAILURE;} };
    let input_path=root.join(&manifest.input_map); if !input_path.is_file(){eprintln!("warning: input map missing: {}",relative(&root,&input_path).display());}
    let save_path=root.join(&manifest.save_schema); if !save_path.is_file(){eprintln!("warning: save schema missing: {}",relative(&root,&save_path).display());}
    println!("ok: {} scripts, {} global classes, {} scene nodes, {} assets", index.scripts.len(), index.symbols.len(), scene.nodes.len(), assets.len());
    if !build { return ExitCode::SUCCESS; }
    let pack_path=root.join(".kalcite/assets.kap"); if let Some(parent)=pack_path.parent(){let _=fs::create_dir_all(parent);} if let Err(e)=fs::write(&pack_path,kalcite_assets::encode_pack(&assets)){eprintln!("{}: {e}",pack_path.display());return ExitCode::FAILURE;}

    let target = parse_target_option(args).unwrap_or_else(|| target_from_name(&manifest.target));
    let out_dir = root.join(".kalcite/objects");
    if let Err(e) = fs::create_dir_all(&out_dir) { eprintln!("{}: {e}", out_dir.display()); return ExitCode::FAILURE; }
    for script in &index.scripts {
        if script.module.items.is_empty() { continue; }
        let Some(stem) = script.path.file_stem() else { continue; };
        let output = out_dir.join(stem).with_extension("kco");
        match kalcite_compiler::emit_kco(&script.module, target) {
            Ok(bytes) => if let Err(e) = fs::write(&output, bytes) { eprintln!("{}: {e}", output.display()); return ExitCode::FAILURE; },
            Err(e) => { eprintln!("{}: object emission failed: {e:?}", script.path.display()); return ExitCode::FAILURE; }
        }
    }
    println!("built project objects in {}", out_dir.display());
    ExitCode::SUCCESS
}

fn file_command(command: &str, args: &[String]) -> ExitCode {
    let Some(input_arg) = args.first() else { usage(); return ExitCode::FAILURE; };
    let input = PathBuf::from(input_arg);
    let source = match fs::read_to_string(&input) { Ok(s) => s, Err(e) => { eprintln!("{}: {e}", input.display()); return ExitCode::FAILURE; } };
    if command == "lint" {
        let diagnostics = lint(&source);
        for d in &diagnostics { let level=match d.severity{Severity::Warning=>"warning",Severity::Error=>"error"}; eprintln!("{level}[{}]: {}",d.code,d.message); }
        return if has_errors(&diagnostics) { ExitCode::FAILURE } else { ExitCode::SUCCESS };
    }
    let (module, report) = match kalcite_compiler::check(&source) { Ok(v)=>v, Err(e)=>{eprintln!("{e}");return ExitCode::FAILURE;} };
    match command {
        "check" => println!("ok: {} classes, {} structs, {} functions, ~{} bytes static (rough)",report.classes,report.structs,report.functions,report.estimated_static_bytes),
        "emit-mir" => match kalcite_compiler::emit_mir(&source) { Ok(mir) => print!("{mir}"), Err(e) => { eprintln!("{e}"); return ExitCode::FAILURE; } },
        "emit-rust" => match kalcite_compiler::emit_rust(&source) { Ok(rust) => print!("{rust}"), Err(e) => { eprintln!("{e}"); return ExitCode::FAILURE; } },
        "build" => {
            let mut output=input.with_extension("kco"); let mut target=Target::Portable; let mut i=1;
            while i<args.len(){match args[i].as_str(){"-o" if i+1<args.len()=>{output=PathBuf::from(&args[i+1]);i+=2},"--target" if i+1<args.len()=>{target=target_from_name(&args[i+1]);i+=2},other=>{eprintln!("unknown option `{other}`");return ExitCode::FAILURE;}}}
            match kalcite_compiler::emit_kco(&module,target){Ok(bytes)=>if let Err(e)=fs::write(&output,bytes){eprintln!("{}: {e}",output.display());return ExitCode::FAILURE},Err(e)=>{eprintln!("object emission failed: {e:?}");return ExitCode::FAILURE;}}
            println!("built {}",output.display());
        }
        _=>{usage();return ExitCode::FAILURE;}
    }
    ExitCode::SUCCESS
}

fn target_from_name(name:&str)->Target{match name{"numworks"=>Target::NumWorks,"desktop"=>Target::Desktop,"web"=>Target::Web,_=>Target::Portable}}
fn parse_target_option(args:&[String])->Option<Target>{args.windows(2).find(|w|w[0]=="--target").map(|w|target_from_name(&w[1]))}
fn relative<'a>(root:&'a Path,path:&'a Path)->&'a Path{path.strip_prefix(root).unwrap_or(path)}
