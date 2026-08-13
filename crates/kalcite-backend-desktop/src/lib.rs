use kalcite_mir::Program;
use std::{fs, path::Path};

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    NoScene,
    Rust(kalcite_backend_rust::EmitError),
    InvalidUiSurfaceSize { width: usize, height: usize },
}
impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}
impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "{e}"),
            Self::NoScene => write!(f, "no @scene class found"),
            Self::Rust(e) => write!(f, "{e}"),
            Self::InvalidUiSurfaceSize { width, height } => write!(
                f,
                "UI surface must be at least 320x240 pixels, received {width}x{height}"
            ),
        }
    }
}

/// Emit a native desktop development runner.
///
/// The game still renders into the same 320x240 RGB565 logical framebuffer
/// used by the NumWorks backend. The host runner only presents that buffer in
/// a resizable native window using nearest-neighbour integer scaling.
pub fn emit_project(program: &Program, app_name: &str, root: &Path) -> Result<(), Error> {
    emit_project_with_resources(program, app_name, root, None, None, None, None, None)
}

pub fn emit_project_with_resources(
    program: &Program,
    app_name: &str,
    root: &Path,
    scene_data: Option<&[u8]>,
    assets: Option<&[u8]>,
    scene_runtime: Option<&str>,
    input_runtime: Option<&str>,
    save_runtime: Option<&str>,
) -> Result<(), Error> {
    let scene = program.scene().ok_or(Error::NoScene)?;
    fs::create_dir_all(root.join("src"))?;
    fs::write(root.join("Cargo.toml"), cargo_manifest(app_name))?;
    fs::write(root.join("src/platform.rs"), PLATFORM)?;
    fs::write(root.join("src/runtime.rs"), RUNTIME)?;
    fs::write(root.join("src/stdlib.rs"), kalcite_stdlib::RUST_SOURCE)?;
    write_project_data(root, scene_data, assets, input_runtime, save_runtime)?;
    fs::write(
        root.join("src/scene_runtime.rs"),
        scene_runtime
            .map(str::to_string)
            .unwrap_or_else(|| format!("pub type SceneRuntime = crate::game::{};\n", scene.name)),
    )?;
    fs::write(
        root.join("src/game.rs"),
        kalcite_backend_rust::emit_game(program).map_err(Error::Rust)?,
    )?;
    let has_update = scene_runtime.is_some()
        || scene
            .functions
            .iter()
            .any(|function| function.name == "update");
    let has_draw = scene_runtime.is_some()
        || scene
            .functions
            .iter()
            .any(|function| function.name == "draw");
    let root_type = if scene_runtime.is_some() {
        "scene_runtime::SceneRuntime".to_string()
    } else {
        format!("game::{}", scene.name)
    };
    let main = MAIN
        .replace("__SCENE__", &root_type)
        .replace("__APP_NAME__", &escape_rust_string(app_name))
        .replace(
            "__UPDATE_CALL__",
            if has_update { "game.update();" } else { "" },
        )
        .replace("__DRAW_CALL__", if has_draw { "game.draw();" } else { "" });
    fs::write(root.join("src/main.rs"), main)?;
    Ok(())
}

/// Options for the small native desktop UI runner.
///
/// This runner intentionally owns a resizable pixel surface in window
/// coordinates. It does not scale the fixed 320x240 game framebuffer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiSurfaceOptions {
    pub title: String,
    pub initial_width: usize,
    pub initial_height: usize,
}

impl Default for UiSurfaceOptions {
    fn default() -> Self {
        Self {
            title: "Kalcite Settings".into(),
            initial_width: 720,
            initial_height: 520,
        }
    }
}

/// Emit a self-contained settings application that exercises the first UI
/// surface: resizable layout, focus navigation, a button, and a bounded text
/// field. It deliberately does not depend on a game scene or framebuffer.
pub fn emit_ui_settings_project(root: &Path, options: &UiSurfaceOptions) -> Result<(), Error> {
    if options.initial_width < 320 || options.initial_height < 240 {
        return Err(Error::InvalidUiSurfaceSize {
            width: options.initial_width,
            height: options.initial_height,
        });
    }
    fs::create_dir_all(root.join("src"))?;
    fs::write(root.join("Cargo.toml"), ui_cargo_manifest(&options.title))?;
    fs::write(
        root.join("src/main.rs"),
        UI_MAIN
            .replace("__APP_NAME__", &escape_rust_string(&options.title))
            .replace("__INITIAL_WIDTH__", &options.initial_width.to_string())
            .replace("__INITIAL_HEIGHT__", &options.initial_height.to_string()),
    )?;
    Ok(())
}

fn write_project_data(
    root: &Path,
    scene: Option<&[u8]>,
    assets: Option<&[u8]>,
    input_runtime: Option<&str>,
    save_runtime: Option<&str>,
) -> Result<(), std::io::Error> {
    let scene = scene.unwrap_or_default();
    let assets = assets.unwrap_or_default();
    fs::write(root.join("src/entry.ksc2"), scene)?;
    fs::write(root.join("src/assets.kap"), assets)?;
    fs::write(
        root.join("src/project_data.rs"),
        format!(
            "#[used]\npub static ENTRY_SCENE: [u8; {}] = *include_bytes!(\"entry.ksc2\");\n#[used]\npub static ASSET_PACK: [u8; {}] = *include_bytes!(\"assets.kap\");\n{}",
            scene.len(),
            assets.len(),
            format!(
                "{}\n{}\n{}",
                ASSET_RUNTIME.trim_start_matches("#![no_std]"),
                input_runtime.unwrap_or("pub fn action_mask(_:&str)->u64{0}"),
                save_runtime.unwrap_or("pub struct ProjectSave;")
            )
        ),
    )
}

fn escape_rust_string(value: &str) -> String {
    value.chars().flat_map(char::escape_default).collect()
}

fn cargo_manifest(name: &str) -> String {
    format!(
        r#"[package]
name = "kalcite-game-desktop"
version = "0.1.0"
edition = "2021"
description = "Generated by Kalcite: {name}"

[dependencies]
minifb = "0.27"

[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
strip = "symbols"

[workspace]
"#
    )
}

fn ui_cargo_manifest(name: &str) -> String {
    format!(
        r#"[package]
name = "kalcite-ui-desktop"
version = "0.1.0"
edition = "2021"
description = "Generated Kalcite UI sample: {name}"

[dependencies]
minifb = "0.27"

[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
strip = "symbols"

[workspace]
"#
    )
}

const RUNTIME: &str = include_str!("../../kalcite-runtime-core/src/pool.rs");
const ASSET_RUNTIME: &str = include_str!("../../kalcite-engine-assets/src/lib.rs");

// Kept separate from `MAIN`: this sample owns a variable-size desktop canvas;
// the game runner below continues to use its fixed 320x240 logical viewport.
const UI_MAIN: &str = r#"use minifb::{Key, KeyRepeat, MouseButton, MouseMode, Window, WindowOptions};

const TITLE: &str = "__APP_NAME__";
const INITIAL_WIDTH: usize = __INITIAL_WIDTH__;
const INITIAL_HEIGHT: usize = __INITIAL_HEIGHT__;

#[derive(Clone, Copy)]
struct Rect { x: usize, y: usize, width: usize, height: usize }
impl Rect {
    fn contains(self, x: usize, y: usize) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.width && y < self.y + self.height
    }
}

#[derive(Default)]
struct Settings { dark_mode: bool, name: String, focus: u8 }

fn fill(buffer: &mut [u32], width: usize, rect: Rect, color: u32) {
    let height = buffer.len() / width;
    let x_end = (rect.x + rect.width).min(width);
    let y_end = (rect.y + rect.height).min(height);
    for y in rect.y.min(height)..y_end { buffer[y * width + rect.x.min(width)..y * width + x_end].fill(color); }
}

fn border(buffer: &mut [u32], width: usize, rect: Rect, color: u32) {
    fill(buffer, width, Rect { height: 1, ..rect }, color);
    fill(buffer, width, Rect { y: rect.y + rect.height.saturating_sub(1), height: 1, ..rect }, color);
    fill(buffer, width, Rect { width: 1, ..rect }, color);
    fill(buffer, width, Rect { x: rect.x + rect.width.saturating_sub(1), width: 1, ..rect }, color);
}

fn glyph(ch: char) -> [u8; 5] {
    match ch.to_ascii_uppercase() {
        'A'=>[2,5,7,5,5], 'B'=>[6,5,6,5,6], 'C'=>[3,4,4,4,3], 'D'=>[6,5,5,5,6],
        'E'=>[7,4,6,4,7], 'F'=>[7,4,6,4,4], 'G'=>[3,4,5,5,3], 'H'=>[5,5,7,5,5],
        'I'=>[7,2,2,2,7], 'J'=>[1,1,1,5,2], 'K'=>[5,5,6,5,5], 'L'=>[4,4,4,4,7],
        'M'=>[5,7,7,5,5], 'N'=>[5,7,7,7,5], 'O'=>[2,5,5,5,2], 'P'=>[6,5,6,4,4],
        'Q'=>[2,5,5,7,3], 'R'=>[6,5,6,5,5], 'S'=>[3,4,2,1,6], 'T'=>[7,2,2,2,2],
        'U'=>[5,5,5,5,7], 'V'=>[5,5,5,5,2], 'W'=>[5,5,7,7,5], 'X'=>[5,5,2,5,5],
        'Y'=>[5,5,2,2,2], 'Z'=>[7,1,2,4,7], '0'=>[7,5,5,5,7], '1'=>[2,6,2,2,7],
        '2'=>[6,1,7,4,7], '3'=>[6,1,3,1,6], '4'=>[5,5,7,1,1], '5'=>[7,4,7,1,6],
        '6'=>[3,4,7,5,7], '7'=>[7,1,2,2,2], '8'=>[7,5,7,5,7], '9'=>[7,5,7,1,6],
        _=>[0,0,0,0,0],
    }
}

fn text(buffer: &mut [u32], width: usize, x: usize, y: usize, value: &str, color: u32) {
    for (index, ch) in value.chars().enumerate() {
        for (row, bits) in glyph(ch).into_iter().enumerate() {
            for column in 0..3 { if bits & (1 << (2 - column)) != 0 {
                let px = x + index * 4 + column; let py = y + row;
                if px < width && py < buffer.len() / width { buffer[py * width + px] = color; }
            }}
        }
    }
}

fn key_char(key: Key) -> Option<char> {
    Some(match key {
        Key::A=>'A',Key::B=>'B',Key::C=>'C',Key::D=>'D',Key::E=>'E',Key::F=>'F',Key::G=>'G',Key::H=>'H',Key::I=>'I',Key::J=>'J',Key::K=>'K',Key::L=>'L',Key::M=>'M',Key::N=>'N',Key::O=>'O',Key::P=>'P',Key::Q=>'Q',Key::R=>'R',Key::S=>'S',Key::T=>'T',Key::U=>'U',Key::V=>'V',Key::W=>'W',Key::X=>'X',Key::Y=>'Y',Key::Z=>'Z',Key::Key0=>'0',Key::Key1=>'1',Key::Key2=>'2',Key::Key3=>'3',Key::Key4=>'4',Key::Key5=>'5',Key::Key6=>'6',Key::Key7=>'7',Key::Key8=>'8',Key::Key9=>'9',Key::Space=>' ', _=>return None,
    })
}

fn main() {
    let mut window = Window::new(TITLE, INITIAL_WIDTH, INITIAL_HEIGHT, WindowOptions { resize: true, ..WindowOptions::default() }).expect("unable to create UI window");
    window.set_target_fps(60);
    let mut settings = Settings { name: "ADA".into(), ..Settings::default() };
    let mut previous_mouse = false;
    while window.is_open() && !window.is_key_down(Key::Escape) {
        let (width, height) = window.get_size();
        let mut buffer = vec![if settings.dark_mode { 0x15181e } else { 0xf4f6fa }; width * height];
        let card_width = width.saturating_sub(48).min(480); let card_x = (width - card_width) / 2;
        let card = Rect { x: card_x, y: 36, width: card_width, height: height.saturating_sub(72).max(250) };
        let toggle = Rect { x: card.x + 24, y: card.y + 92, width: card.width.saturating_sub(48), height: 38 };
        let input = Rect { x: card.x + 24, y: card.y + 166, width: card.width.saturating_sub(48), height: 38 };
        let panel = if settings.dark_mode { 0x242936 } else { 0xffffff }; let ink = if settings.dark_mode { 0xf2f4f8 } else { 0x20242c }; let accent = 0x4c78ff;
        fill(&mut buffer, width, card, panel); border(&mut buffer, width, card, if settings.dark_mode { 0x3b4354 } else { 0xd5dbe7 });
        text(&mut buffer, width, card.x + 24, card.y + 24, "SETTINGS", ink); text(&mut buffer, width, card.x + 24, card.y + 48, "A RESIZABLE KALCITE UI SURFACE", ink);
        fill(&mut buffer, width, toggle, if settings.dark_mode { accent } else { 0xe7ebf3 }); border(&mut buffer, width, toggle, if settings.focus == 0 { accent } else { 0x9ca9bd });
        text(&mut buffer, width, toggle.x + 12, toggle.y + 15, if settings.dark_mode { "DARK MODE ENABLED" } else { "DARK MODE DISABLED" }, ink);
        text(&mut buffer, width, input.x, input.y.saturating_sub(12), "USER NAME", ink); fill(&mut buffer, width, input, panel); border(&mut buffer, width, input, if settings.focus == 1 { accent } else { 0x9ca9bd }); text(&mut buffer, width, input.x + 12, input.y + 15, &settings.name, ink);
        text(&mut buffer, width, card.x + 24, card.y + 228, "TAB CHANGES FOCUS. ENTER TO TOGGLE.", ink);
        let mouse = window.get_mouse_pos(MouseMode::Discard).map(|(x,y)| (x as usize,y as usize)); let mouse_down = window.get_mouse_down(MouseButton::Left);
        if mouse_down && !previous_mouse { if let Some((x,y)) = mouse { if toggle.contains(x,y) { settings.dark_mode = !settings.dark_mode; settings.focus = 0; } else if input.contains(x,y) { settings.focus = 1; } } }
        previous_mouse = mouse_down;
        if window.is_key_pressed(Key::Tab, KeyRepeat::No) { settings.focus = (settings.focus + 1) % 2; }
        if settings.focus == 0 && (window.is_key_pressed(Key::Enter, KeyRepeat::No) || window.is_key_pressed(Key::Space, KeyRepeat::No)) { settings.dark_mode = !settings.dark_mode; }
        if settings.focus == 1 { for key in window.get_keys_pressed(KeyRepeat::Yes) { if key == Key::Backspace { settings.name.pop(); } else if let Some(ch) = key_char(key) { if settings.name.len() < 24 { settings.name.push(ch); } } } }
        window.update_with_buffer(&buffer, width, height).expect("failed to present UI frame");
    }
}
"#;

const PLATFORM: &str = r#"#![allow(dead_code)]
use core::ops::{Add, AddAssign, Sub, SubAssign};
use std::{
    fs::{self, File},
    path::PathBuf,
    io::{self, Write},
    sync::{atomic::{AtomicI32, AtomicU32, AtomicU64, Ordering}, Mutex, OnceLock},
    thread,
    time::{Duration, Instant},
};

pub const WIDTH: usize = 320;
pub const HEIGHT: usize = 240;
pub const PIXELS: usize = WIDTH * HEIGHT;

static FRAMEBUFFER: OnceLock<Mutex<Box<[u16; PIXELS]>>> = OnceLock::new();
static KEYS: AtomicU64 = AtomicU64::new(0);
static PREV_KEYS: AtomicU64 = AtomicU64::new(0);
static START: OnceLock<Instant> = OnceLock::new();
static RANDOM_STATE: AtomicU32 = AtomicU32::new(0x4b1d_1234);
static CAMERA_X: AtomicI32 = AtomicI32::new(0);
static CAMERA_Y: AtomicI32 = AtomicI32::new(0);
static DRAW_CALLS: AtomicU32 = AtomicU32::new(0);
static DIRTY_PIXELS: AtomicU32 = AtomicU32::new(0);
static SPRITES: AtomicU32 = AtomicU32::new(0);
static TILES: AtomicU32 = AtomicU32::new(0);
static COLLISION_QUERIES: AtomicU32 = AtomicU32::new(0);
static PHYSICS_NS: AtomicU64 = AtomicU64::new(0);

fn framebuffer() -> &'static Mutex<Box<[u16; PIXELS]>> {
    FRAMEBUFFER.get_or_init(|| Mutex::new(Box::new([0; PIXELS])))
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct Vec2fx { pub x: i16, pub y: i16 }
impl Vec2fx { pub const fn new(x: i16, y: i16) -> Self { Self { x, y } } }
impl Add for Vec2fx { type Output=Self; fn add(self,r:Self)->Self { Self{x:self.x+r.x,y:self.y+r.y} } }
impl AddAssign for Vec2fx { fn add_assign(&mut self,r:Self){self.x+=r.x;self.y+=r.y} }
impl Sub for Vec2fx { type Output=Self; fn sub(self,r:Self)->Self { Self{x:self.x-r.x,y:self.y-r.y} } }
impl SubAssign for Vec2fx { fn sub_assign(&mut self,r:Self){self.x-=r.x;self.y-=r.y} }

#[derive(Clone, Copy)] pub struct Color(pub u16);
#[allow(non_upper_case_globals)] impl Color {
    pub const Black:Self=Self(0x0000); pub const White:Self=Self(0xffff);
    pub const Red:Self=Self(0xf800); pub const Green:Self=Self(0x07e0);
    pub const Blue:Self=Self(0x001f); pub const Orange:Self=Self(0xfd20);
    pub const Yellow:Self=Self(0xffe0); pub const Cyan:Self=Self(0x07ff); pub const Gray:Self=Self(0x8410);
}

#[derive(Clone, Copy)] pub struct Key(pub u8);
#[allow(non_upper_case_globals)] impl Key {
    pub const Left:Self=Self(0); pub const Up:Self=Self(1); pub const Down:Self=Self(2);
    pub const Right:Self=Self(3); pub const Ok:Self=Self(4); pub const Back:Self=Self(5); pub const Home:Self=Self(6);
    pub const Plus:Self=Self(7); pub const Minus:Self=Self(8);
}

pub struct Input;
impl Input {
    #[inline]
    pub fn held(key: Key) -> bool {
        let bit = 1u64.checked_shl(key.0 as u32).unwrap_or(0);
        KEYS.load(Ordering::Relaxed) & bit != 0
    }
    #[inline] pub fn pressed(key:Key)->bool{let b=1u64<<key.0;KEYS.load(Ordering::Relaxed)&b!=0&&PREV_KEYS.load(Ordering::Relaxed)&b==0}
    #[inline] pub fn released(key:Key)->bool{let b=1u64<<key.0;KEYS.load(Ordering::Relaxed)&b==0&&PREV_KEYS.load(Ordering::Relaxed)&b!=0}
    #[inline] pub fn action_held(action:&str)->bool{KEYS.load(Ordering::Relaxed)&crate::project_data::action_mask(action)!=0}
    #[inline] pub fn action_pressed(action:&str)->bool{let m=crate::project_data::action_mask(action);KEYS.load(Ordering::Relaxed)&m!=0&&PREV_KEYS.load(Ordering::Relaxed)&m==0}
    #[inline] pub fn action_released(action:&str)->bool{let m=crate::project_data::action_mask(action);KEYS.load(Ordering::Relaxed)&m==0&&PREV_KEYS.load(Ordering::Relaxed)&m!=0}
    #[inline] pub fn action_axis(negative:&str,positive:&str)->i16{Self::action_held(positive) as i16-Self::action_held(negative) as i16}
}

pub struct Physics;
impl Physics {
    #[inline] pub fn hit(ax:i16,ay:i16,aw:i16,ah:i16,bx:i16,by:i16,bw:i16,bh:i16)->bool{COLLISION_QUERIES.fetch_add(1,Ordering::Relaxed);ax<bx.saturating_add(bw)&&ax.saturating_add(aw)>bx&&ay<by.saturating_add(bh)&&ay.saturating_add(ah)>by}
    #[inline] pub fn move_x(x:i16,y:i16,w:i16,h:i16,dx:i16,sx:i16,sy:i16,sw:i16,sh:i16)->i16{let started=Instant::now();let next=x.saturating_add(dx);let out=if Self::hit(next,y,w,h,sx,sy,sw,sh){x}else{next};PHYSICS_NS.fetch_add(started.elapsed().as_nanos().min(u64::MAX as u128)as u64,Ordering::Relaxed);out}
}

static AUDIO_TONES: AtomicU32 = AtomicU32::new(0);
pub struct Audio;
impl Audio { pub fn tone(_hz:u16,_ms:u16,_volume:u8){AUDIO_TONES.fetch_add(1,Ordering::Relaxed);}pub fn stop(){}pub fn command_count()->u32{AUDIO_TONES.load(Ordering::Relaxed)} }

/// Host-only hook. Backends for calculators never expose this to game code.
pub fn host_set_key(key: Key, down: bool) {
    let bit = 1u64.checked_shl(key.0 as u32).unwrap_or(0);
    if down { KEYS.fetch_or(bit, Ordering::Relaxed); }
    else { KEYS.fetch_and(!bit, Ordering::Relaxed); }
}


pub struct System;
impl System {
    #[inline]
    pub fn millis() -> u32 {
        START.get_or_init(Instant::now).elapsed().as_millis() as u32
    }

    #[inline]
    pub fn sleep_ms(ms: u32) { thread::sleep(Duration::from_millis(ms as u64)); }
}

pub struct Hardware;
impl Hardware {
    pub fn is_numworks() -> bool { false }
    pub fn telemetry_supported() -> bool { true }
    pub fn battery_level() -> u32 { 100 }
    pub fn battery_mv() -> u32 { 5000 }
    pub fn charging() -> bool { false }
    pub fn usb_plugged() -> bool { false }
    pub fn backlight() -> u32 { 100 }
    pub fn random() -> u32 {
        let mut old=RANDOM_STATE.load(Ordering::Relaxed);
        loop {
            let mut x=old; x^=x<<13; x^=x>>17; x^=x<<5;
            match RANDOM_STATE.compare_exchange_weak(old,x,Ordering::Relaxed,Ordering::Relaxed) { Ok(_)=>return x, Err(v)=>old=v }
        }
    }
}

pub struct Storage;
impl Storage {
    pub fn supported() -> bool { true }

    fn path(name: &str) -> Option<PathBuf> {
        if name.is_empty() || name.len() > 48 { return None; }
        if !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-') { return None; }
        let root=PathBuf::from(".kalcite-saves");
        if fs::create_dir_all(&root).is_err() { return None; }
        Some(root.join(format!("{name}.kdoc")))
    }

    pub fn write_bytes(name: &str, bytes: &[u8]) -> bool {
        let Some(path)=Self::path(name) else { return false; };
        fs::write(path, bytes).is_ok()
    }

    pub fn read_into(name: &str, out: &mut [u8]) -> usize {
        let Some(path)=Self::path(name) else { return 0; };
        let Ok(bytes)=fs::read(path) else { return 0; };
        let n=core::cmp::min(bytes.len(),out.len()); out[..n].copy_from_slice(&bytes[..n]); n
    }

    pub fn write_text(name: &str, text: &str) -> bool { Self::write_bytes(name,text.as_bytes()) }

    pub fn exists(name: &str) -> bool {
        Self::path(name).is_some_and(|p| p.is_file())
    }

    pub fn size(name: &str) -> u32 {
        Self::path(name)
            .and_then(|p| fs::metadata(p).ok())
            .map(|m| m.len().min(u32::MAX as u64) as u32)
            .unwrap_or(0)
    }

    pub fn checksum(name: &str) -> u32 {
        let Some(path)=Self::path(name) else { return 0; };
        let Ok(bytes)=fs::read(path) else { return 0; };
        let mut h=0x811c9dc5u32;
        for b in bytes { h ^= b as u32; h=h.wrapping_mul(0x01000193); }
        h
    }

    pub fn remove(name: &str) -> bool {
        let Some(path)=Self::path(name) else { return false; };
        match fs::remove_file(path) { Ok(())=>true, Err(e)=>e.kind()==io::ErrorKind::NotFound }
    }

    // std does not expose portable filesystem free-space queries. These stay
    // zero on desktop unless a future host-storage backend adds such support.
    pub fn free_bytes() -> u32 { 0 }
    pub fn total_bytes() -> u32 { 0 }
}

pub struct Draw;
impl Draw {
    pub fn camera(x:i16,y:i16){CAMERA_X.store(x as i32,Ordering::Relaxed);CAMERA_Y.store(y as i32,Ordering::Relaxed);}
    pub fn clear(c: Color) { metric_draw(PIXELS as u32);framebuffer().lock().unwrap().fill(c.0); }
    pub fn rect(x:i16,y:i16,w:i16,h:i16,c:Color) {
        if w<=0 || h<=0 { return; }
        let x0=x.clamp(0,WIDTH as i16) as usize; let y0=y.clamp(0,HEIGHT as i16) as usize;
        let x1=(x+w).clamp(0,WIDTH as i16) as usize; let y1=(y+h).clamp(0,HEIGHT as i16) as usize;
        if x1<=x0 || y1<=y0 { return; }
        metric_draw(((x1-x0)*(y1-y0)) as u32);
        let mut fb=framebuffer().lock().unwrap();
        for yy in y0..y1 { let row=yy*WIDTH; for xx in x0..x1 { fb[row+xx]=c.0; } }
    }
    pub fn sprite(name:&str,x:i16,y:i16) {
        let Ok(pack)=crate::project_data::AssetPack::new(&crate::project_data::ASSET_PACK) else{return;};
        let Some(asset)=pack.get_named(name) else{return;};
        let(x,y)=world_to_screen(x,y);draw_sprite_data(asset.data,x,y,0,0,u16::MAX,u16::MAX);
    }
    pub fn sprite_region(name:&str,x:i16,y:i16,sx:u16,sy:u16,w:u16,h:u16) {
        let Ok(pack)=crate::project_data::AssetPack::new(&crate::project_data::ASSET_PACK) else{return;};
        let Some(asset)=pack.get_named(name) else{return;};
        let(x,y)=world_to_screen(x,y);draw_sprite_data(asset.data,x,y,sx,sy,w,h);
    }
    pub fn sprite_frame(sheet:&str,frame:u16,x:i16,y:i16) {
        let Ok(pack)=crate::project_data::AssetPack::new(&crate::project_data::ASSET_PACK) else{return;};
        let Some(meta)=pack.get_named(sheet) else{return;};
        if meta.kind!=3||meta.data.len()!=12{return;}
        let image=u64::from_le_bytes(meta.data[..8].try_into().unwrap());
        let fw=u16::from_le_bytes(meta.data[8..10].try_into().unwrap());let fh=u16::from_le_bytes(meta.data[10..12].try_into().unwrap());
        let Some(sprite)=pack.get(image) else{return;};if sprite.data.len()<4||fw==0||fh==0{return;}
        let width=u16::from_le_bytes([sprite.data[0],sprite.data[1]]);let cols=width/fw;if cols==0{return;}
        let(x,y)=world_to_screen(x,y);draw_sprite_data(sprite.data,x,y,(frame%cols)*fw,(frame/cols)*fh,fw,fh);
    }
    pub fn tilemap(map:&str,tileset:&str,tile_w:u16,tile_h:u16,x:i16,y:i16) {
        let Ok(pack)=crate::project_data::AssetPack::new(&crate::project_data::ASSET_PACK)else{return;};let Some(map)=pack.get_named(map)else{return;};let Some(sprite)=pack.get_named(tileset)else{return;};
        if map.kind!=2||sprite.kind!=1||map.data.len()<4||sprite.data.len()<4||tile_w==0||tile_h==0{return;}let mw=u16::from_le_bytes([map.data[0],map.data[1]]) as usize;let mh=u16::from_le_bytes([map.data[2],map.data[3]]) as usize;let sw=u16::from_le_bytes([sprite.data[0],sprite.data[1]]);let cols=sw/tile_w;if cols==0{return;}
        TILES.fetch_add((mw*mh).min(u32::MAX as usize)as u32,Ordering::Relaxed);for row in 0..mh{for col in 0..mw{let at=4+(row*mw+col)*2;if at+2>map.data.len(){return;}let tile=u16::from_le_bytes([map.data[at],map.data[at+1]]);let(dx,dy)=world_to_screen(x+col as i16*tile_w as i16,y+row as i16*tile_h as i16);draw_sprite_data(sprite.data,dx,dy,(tile%cols)*tile_w,(tile/cols)*tile_h,tile_w,tile_h);}}
    }
    pub fn pixel_at(x:i16,y:i16)->u32 {
        if x<0 || y<0 || x>=WIDTH as i16 || y>=HEIGHT as i16 { return 0; }
        framebuffer().lock().unwrap()[y as usize*WIDTH+x as usize] as u32
    }
    pub fn text(text:&str,x:i16,y:i16,c:Color,bg:Color) {
        metric_draw((text.len()*35).min(u32::MAX as usize)as u32);
        let mut px=x;
        for b in text.bytes() {
            if b==b'\n' { px=x; continue; }
            draw_char(b,px,y,c,bg); px+=6;
        }
    }
    pub fn number<T: Into<u64> + Copy>(value:T,x:i16,y:i16,c:Color,bg:Color) {
        let mut value:u64=value.into();
        let mut buf=[0u8;10]; let mut n=0usize;
        if value==0 { Self::text("0",x,y,c,bg); return; }
        while value>0 && n<buf.len() { buf[n]=b'0'+(value%10) as u8; value/=10; n+=1; }
        let mut px=x;
        while n>0 { n-=1; draw_char(buf[n],px,y,c,bg); px+=6; }
    }
}

fn world_to_screen(x:i16,y:i16)->(i16,i16){(x.saturating_sub(CAMERA_X.load(Ordering::Relaxed) as i16),y.saturating_sub(CAMERA_Y.load(Ordering::Relaxed) as i16))}

fn draw_sprite_data(data:&[u8],x:i16,y:i16,sx:u16,sy:u16,requested_w:u16,requested_h:u16) {
    if data.len()<4{return;}let width=u16::from_le_bytes([data[0],data[1]]);let height=u16::from_le_bytes([data[2],data[3]]);
    let sw=requested_w.min(width.saturating_sub(sx));let sh=requested_h.min(height.saturating_sub(sy));if sw==0||sh==0{return;}
    SPRITES.fetch_add(1,Ordering::Relaxed);metric_draw(sw as u32*sh as u32);
    let mut pixel=0usize;let mut at=4usize;let mut fb=framebuffer().lock().unwrap();
    while at+4<=data.len(){let count=data[at] as usize;let transparent=data[at+1]!=0;let color=u16::from_le_bytes([data[at+2],data[at+3]]);at+=4;if count==0{return;}
        let row=pixel/width as usize;let col=pixel%width as usize;pixel+=count;
        if transparent||row<sy as usize||row>=sy as usize+sh as usize{continue;}
        let run_start=col.max(sx as usize);let run_end=(col+count).min(sx as usize+sw as usize);if run_end<=run_start{continue;}
        let dy=y+row as i16-sy as i16;if dy<0||dy>=HEIGHT as i16{continue;}let dx=x+run_start as i16-sx as i16;let end=x+run_end as i16-sx as i16;
        let x0=dx.max(0) as usize;let x1=end.min(WIDTH as i16).max(0) as usize;if x1>x0{fb[dy as usize*WIDTH+x0..dy as usize*WIDTH+x1].fill(color);}
    }
}

fn glyph(c:u8)->[u8;7] {
    match c.to_ascii_uppercase() {
        b'A'=>[14,17,17,31,17,17,17], b'B'=>[30,17,17,30,17,17,30], b'C'=>[14,17,16,16,16,17,14],
        b'D'=>[30,17,17,17,17,17,30], b'E'=>[31,16,16,30,16,16,31], b'F'=>[31,16,16,30,16,16,16],
        b'G'=>[14,17,16,23,17,17,15], b'H'=>[17,17,17,31,17,17,17], b'I'=>[14,4,4,4,4,4,14],
        b'J'=>[7,2,2,2,2,18,12], b'K'=>[17,18,20,24,20,18,17], b'L'=>[16,16,16,16,16,16,31],
        b'M'=>[17,27,21,21,17,17,17], b'N'=>[17,25,21,19,17,17,17], b'O'=>[14,17,17,17,17,17,14],
        b'P'=>[30,17,17,30,16,16,16], b'Q'=>[14,17,17,17,21,18,13], b'R'=>[30,17,17,30,20,18,17],
        b'S'=>[15,16,16,14,1,1,30], b'T'=>[31,4,4,4,4,4,4], b'U'=>[17,17,17,17,17,17,14],
        b'V'=>[17,17,17,17,17,10,4], b'W'=>[17,17,17,21,21,21,10], b'X'=>[17,17,10,4,10,17,17],
        b'Y'=>[17,17,10,4,4,4,4], b'Z'=>[31,1,2,4,8,16,31],
        b'0'=>[14,17,19,21,25,17,14], b'1'=>[4,12,4,4,4,4,14], b'2'=>[14,17,1,2,4,8,31],
        b'3'=>[30,1,1,14,1,1,30], b'4'=>[2,6,10,18,31,2,2], b'5'=>[31,16,16,30,1,1,30],
        b'6'=>[14,16,16,30,17,17,14], b'7'=>[31,1,2,4,8,8,8], b'8'=>[14,17,17,14,17,17,14],
        b'9'=>[14,17,17,15,1,1,14], b'-'=>[0,0,0,31,0,0,0], b':'=>[0,4,4,0,4,4,0],
        b'/'=>[1,2,2,4,8,8,16], b'.'=>[0,0,0,0,0,4,4], b'%'=>[17,2,4,8,17,0,0],
        b'('=>[2,4,8,8,8,4,2], b')'=>[8,4,2,2,2,4,8], b'='=>[0,31,0,31,0,0,0],
        _=>[0,0,0,0,0,0,0],
    }
}
fn draw_char(ch:u8,x:i16,y:i16,c:Color,bg:Color) {
    let g=glyph(ch);
    rect_raw(x,y,6,8,bg);
    for (yy,row) in g.iter().enumerate() { for xx in 0..5 { if row & (1 << (4-xx)) != 0 { rect_raw(x+xx,y+yy as i16,1,1,c); } } }
}
fn rect_raw(x:i16,y:i16,w:i16,h:i16,c:Color) {
    if w<=0||h<=0{return;} let x0=x.clamp(0,WIDTH as i16) as usize; let y0=y.clamp(0,HEIGHT as i16) as usize;
    let x1=(x+w).clamp(0,WIDTH as i16) as usize; let y1=(y+h).clamp(0,HEIGHT as i16) as usize; if x1<=x0||y1<=y0{return;}
    let mut fb=framebuffer().lock().unwrap(); for yy in y0..y1 { let row=yy*WIDTH; for xx in x0..x1 { fb[row+xx]=c.0; } }
}

#[inline]
fn rgb565_to_xrgb8888(p: u16) -> u32 {
    let r=((p>>11)&31) as u32; let g=((p>>5)&63) as u32; let b=(p&31) as u32;
    let r=(r<<3)|(r>>2); let g=(g<<2)|(g>>4); let b=(b<<3)|(b>>2);
    (r<<16)|(g<<8)|b
}

pub fn host_copy_xrgb8888(dst: &mut [u32]) {
    assert_eq!(dst.len(), PIXELS);
    let fb=framebuffer().lock().unwrap();
    for (out, &pixel) in dst.iter_mut().zip(fb.iter()) { *out=rgb565_to_xrgb8888(pixel); }
}

fn metric_draw(pixels:u32){DRAW_CALLS.fetch_add(1,Ordering::Relaxed);DIRTY_PIXELS.fetch_add(pixels,Ordering::Relaxed);}
#[derive(Clone,Copy,Default)]pub struct EngineMetrics{pub draw_calls:u32,pub dirty_pixels:u32,pub dirty_regions:u32,pub sprites:u32,pub tiles:u32,pub collision_queries:u32,pub physics_us:u32}
pub fn metrics()->EngineMetrics{EngineMetrics{draw_calls:DRAW_CALLS.load(Ordering::Relaxed),dirty_pixels:DIRTY_PIXELS.load(Ordering::Relaxed),dirty_regions:DRAW_CALLS.load(Ordering::Relaxed),sprites:SPRITES.load(Ordering::Relaxed),tiles:TILES.load(Ordering::Relaxed),collision_queries:COLLISION_QUERIES.load(Ordering::Relaxed),physics_us:(PHYSICS_NS.load(Ordering::Relaxed)/1000).min(u32::MAX as u64)as u32}}
pub fn frame_begin() {DRAW_CALLS.store(0,Ordering::Relaxed);DIRTY_PIXELS.store(0,Ordering::Relaxed);SPRITES.store(0,Ordering::Relaxed);TILES.store(0,Ordering::Relaxed);COLLISION_QUERIES.store(0,Ordering::Relaxed);PHYSICS_NS.store(0,Ordering::Relaxed);}
pub fn frame_end() { PREV_KEYS.store(KEYS.load(Ordering::Relaxed),Ordering::Relaxed); }
pub fn save_ppm(path: &str) -> io::Result<()> {
    let fb=framebuffer().lock().unwrap();
    let mut f=File::create(path)?;
    write!(f,"P6\n{} {}\n255\n",WIDTH,HEIGHT)?;
    for &p in fb.iter() {
        let r=((p>>11)&31) as u8; let g=((p>>5)&63) as u8; let b=(p&31) as u8;
        f.write_all(&[(r<<3)|(r>>2),(g<<2)|(g>>4),(b<<3)|(b>>2)])?;
    }
    Ok(())
}
"#;

const MAIN: &str = r#"mod platform;
mod runtime;
mod stdlib;
mod project_data;
#[allow(dead_code, unused_mut)]
mod scene_runtime;
#[allow(dead_code, non_camel_case_types, non_snake_case, unused_imports, unused_mut, unused_parens, unused_variables)]
mod game;

use minifb::{Key as HostKey, ScaleMode, Window, WindowOptions};
use std::{env, fs::File, io::Write, thread, time::{Duration, Instant}};

const APP_NAME: &str = "__APP_NAME__";
const DEFAULT_SCALE: usize = 3;
const DEFAULT_FPS: u32 = 60;

#[derive(Debug)]
struct Options {
    scale: usize,
    fps: u32,
    headless: bool,
    frames: u64,
    screenshot: String,
    profile: Option<String>,
}

fn options() -> Options {
    let mut out=Options{scale:DEFAULT_SCALE,fps:DEFAULT_FPS,headless:false,frames:180,screenshot:"kalcite-frame.ppm".into(),profile:None};
    let mut args=env::args().skip(1);
    while let Some(arg)=args.next() {
        match arg.as_str() {
            "--scale" => out.scale=args.next().and_then(|v|v.parse().ok()).unwrap_or(DEFAULT_SCALE).clamp(1,8),
            "--fps" => out.fps=args.next().and_then(|v|v.parse().ok()).unwrap_or(DEFAULT_FPS).clamp(1,240),
            "--headless" => out.headless=true,
            "--frames" => out.frames=args.next().and_then(|v|v.parse().ok()).unwrap_or(180),
            "--screenshot" => out.screenshot=args.next().unwrap_or_else(||"kalcite-frame.ppm".into()),
            "--profile" => out.profile=args.next(),
            "--help"|"-h" => {
                println!("{APP_NAME} (Kalcite desktop runner)\n  --scale N       integer window scale (1..8)\n  --fps N         target FPS (1..240)\n  --headless      run without a window\n  --frames N      frames in headless mode\n  --screenshot P  PPM output used by F12/headless mode\n  --profile P     write per-frame CSV timings\n\nKeys: arrows, Enter/Space=OK, Escape/Backspace=Back, H=Home, F12=screenshot, Q=quit");
                std::process::exit(0);
            }
            other => eprintln!("warning: unknown runner option `{other}`"),
        }
    }
    out
}

fn set_inputs(window: &Window) {
    use platform::{host_set_key, Key};
    host_set_key(Key::Left, window.is_key_down(HostKey::Left));
    host_set_key(Key::Up, window.is_key_down(HostKey::Up));
    host_set_key(Key::Down, window.is_key_down(HostKey::Down));
    host_set_key(Key::Right, window.is_key_down(HostKey::Right));
    host_set_key(Key::Ok, window.is_key_down(HostKey::Enter) || window.is_key_down(HostKey::Space));
    host_set_key(Key::Back, window.is_key_down(HostKey::Escape) || window.is_key_down(HostKey::Backspace));
    host_set_key(Key::Home, window.is_key_down(HostKey::H));
}

fn scale_nearest(src: &[u32], dst: &mut [u32], scale: usize) {
    let out_width=platform::WIDTH*scale;
    for y in 0..platform::HEIGHT {
        for sy in 0..scale {
            let dst_row=(y*scale+sy)*out_width;
            let src_row=y*platform::WIDTH;
            for x in 0..platform::WIDTH {
                let p=src[src_row+x];
                let start=dst_row+x*scale;
                dst[start..start+scale].fill(p);
            }
        }
    }
}

#[derive(Clone,Copy,Default)]
struct FrameStats { update_us:u64, render_us:u64, engine:platform::EngineMetrics, static_ram:usize }

fn frame(game: &mut __SCENE__) -> FrameStats {
    platform::frame_begin();
    let update_started=Instant::now();
    __UPDATE_CALL__
    let update_us=update_started.elapsed().as_micros() as u64;
    let render_started=Instant::now();
    __DRAW_CALL__
    platform::frame_end();
    FrameStats{update_us,render_us:render_started.elapsed().as_micros() as u64,engine:platform::metrics(),static_ram:core::mem::size_of_val(game)}
}


fn open_profile(opts: &Options) -> Option<File> {
    let path=opts.profile.as_ref()?;
    match File::create(path) {
        Ok(mut f) => { let _=writeln!(f,"frame,frame_us,update_us,render_us,physics_us,draw_calls,dirty_pixels,dirty_regions,sprites,tiles,collision_queries,pool_used,static_ram,fps"); Some(f) },
        Err(e) => { eprintln!("profile open failed for {path}: {e}"); None }
    }
}

fn write_profile(file: &mut Option<File>, frame_index:u64, start:Instant,stats:FrameStats) {
    let us=start.elapsed().as_micros() as u64;
    let fps=if us>0 { 1_000_000.0/us as f64 } else { 0.0 };
    let e=stats.engine;
    if let Some(f)=file.as_mut() { let _=writeln!(f,"{frame_index},{us},{},{},{},{},{},{},{},{},{},{},{},{fps:.3}",stats.update_us,stats.render_us,e.physics_us,e.draw_calls,e.dirty_pixels,e.dirty_regions,e.sprites,e.tiles,e.collision_queries,0,stats.static_ram); }
}

fn run_headless(opts: &Options) {
    let mut game=__SCENE__::default();
    let mut profile=open_profile(opts);
    for i in 0..opts.frames { let started=Instant::now(); let stats=frame(&mut game); write_profile(&mut profile,i,started,stats); }
    platform::save_ppm(&opts.screenshot).expect("failed to save screenshot");
    println!("Kalcite headless run complete: {} frames -> {}",opts.frames,opts.screenshot);
}

fn run_window(opts: &Options) {
    let width=platform::WIDTH*opts.scale;
    let height=platform::HEIGHT*opts.scale;
    let mut window=Window::new(
        &format!("{} — Kalcite",APP_NAME),
        width,
        height,
        WindowOptions { resize:false, scale_mode:ScaleMode::Stretch, ..WindowOptions::default() },
    ).expect("unable to create Kalcite desktop window");

    let mut game=__SCENE__::default();
    let mut logical=vec![0u32;platform::PIXELS];
    let mut presented=vec![0u32;width*height];
    let frame_time=Duration::from_secs_f64(1.0/opts.fps as f64);
    let mut next_frame=Instant::now();
    let mut screenshot_down=false;
    let mut profile=open_profile(opts);
    let mut frame_index=0u64;

    while window.is_open() && !window.is_key_down(HostKey::Q) {
        let started=Instant::now();
        set_inputs(&window);
        let stats=frame(&mut game);
        platform::host_copy_xrgb8888(&mut logical);
        scale_nearest(&logical,&mut presented,opts.scale);
        window.update_with_buffer(&presented,width,height).expect("failed to present framebuffer");

        let now_screenshot=window.is_key_down(HostKey::F12);
        if now_screenshot && !screenshot_down {
            match platform::save_ppm(&opts.screenshot) {
                Ok(())=>println!("saved {}",opts.screenshot),
                Err(e)=>eprintln!("screenshot failed: {e}"),
            }
        }
        screenshot_down=now_screenshot;
        write_profile(&mut profile,frame_index,started,stats);
        frame_index+=1;

        next_frame+=frame_time;
        let now=Instant::now();
        if next_frame>now { thread::sleep(next_frame-now); }
        else if now.duration_since(next_frame)>frame_time*4 { next_frame=now; }
    }
}

fn main() {
    let opts=options();
    if opts.headless { run_headless(&opts); } else { run_window(&opts); }
}
"#;

#[cfg(test)]
mod generated_module_regression_tests {
    use super::*;

    #[test]
    fn generated_game_module_owns_codegen_lint_policy() {
        assert!(MAIN.contains("#[allow(dead_code, non_camel_case_types, non_snake_case, unused_imports, unused_mut, unused_parens, unused_variables)]\nmod game;"));
    }

    #[test]
    fn scene_lifecycle_hooks_are_template_gated() {
        assert!(MAIN.contains("__UPDATE_CALL__"));
        assert!(MAIN.contains("__DRAW_CALL__"));
    }

    #[test]
    fn desktop_runner_exposes_csv_profiling() {
        assert!(MAIN.contains("--profile"));
        assert!(MAIN.contains("frame,frame_us,update_us,render_us,physics_us"));
    }

    #[test]
    fn ui_runner_is_resizable_and_independent_from_the_game_framebuffer() {
        let root = std::env::temp_dir().join(format!(
            "kalcite-ui-surface-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let options = UiSurfaceOptions::default();
        emit_ui_settings_project(&root, &options).unwrap();
        let main = std::fs::read_to_string(root.join("src/main.rs")).unwrap();
        assert!(main.contains("resize: true"));
        assert!(main.contains("window.get_size()"));
        assert!(main.contains("DARK MODE ENABLED"));
        assert!(main.contains("USER NAME"));
        assert!(!main.contains("platform::WIDTH"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn ui_runner_rejects_a_surface_too_small_for_its_controls() {
        let root = std::env::temp_dir().join("kalcite-ui-surface-too-small");
        let options = UiSurfaceOptions {
            initial_width: 319,
            initial_height: 240,
            ..UiSurfaceOptions::default()
        };
        assert!(matches!(
            emit_ui_settings_project(&root, &options),
            Err(Error::InvalidUiSurfaceSize { .. })
        ));
    }

    #[test]
    fn generated_project_data_embeds_exact_resources() {
        let root = std::env::temp_dir().join(format!(
            "kalcite-desktop-data-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(root.join("src")).unwrap();
        write_project_data(&root, Some(b"KSC2scene"), Some(b"KAP0assets"), None, None).unwrap();
        assert_eq!(
            std::fs::read(root.join("src/entry.ksc2")).unwrap(),
            b"KSC2scene"
        );
        assert_eq!(
            std::fs::read(root.join("src/assets.kap")).unwrap(),
            b"KAP0assets"
        );
        let module = std::fs::read_to_string(root.join("src/project_data.rs")).unwrap();
        assert!(module.contains("ENTRY_SCENE: [u8; 9]"));
        assert!(module.contains("ASSET_PACK: [u8; 10]"));
        std::fs::remove_dir_all(root).unwrap();
    }
}
