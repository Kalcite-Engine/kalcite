use kalcite_mir::Program;
use std::{fs, path::Path};

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    NoScene,
    InvalidName,
    Rust(kalcite_backend_rust::EmitError),
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::NoScene => write!(f, "no @scene class found"),
            Self::InvalidName => write!(f, "NumWorks app name must be 1..=9 ASCII bytes"),
            Self::Rust(error) => write!(f, "{error}"),
        }
    }
}

/// Emit a standalone Rust/EADK project. The generated project deliberately
/// mirrors NumWorks' official `epsilon-sample-app-rust` layout: no custom NWA
/// writer exists here. Cargo produces the relocatable ARM ELF consumed by
/// `nwlink install-nwa` and by the NumWorks third-party app uploader.
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
    if app_name.is_empty() || app_name.len() > 9 || !app_name.is_ascii() {
        return Err(Error::InvalidName);
    }

    fs::create_dir_all(root.join("src"))?;
    fs::create_dir_all(root.join(".cargo"))?;
    fs::write(root.join("Cargo.toml"), CARGO)?;
    fs::write(root.join("rust-toolchain.toml"), TOOLCHAIN)?;
    fs::write(root.join(".cargo/config.toml"), CONFIG)?;
    fs::write(root.join("build.rs"), BUILD_RS)?;
    fs::write(root.join("src/eadk.rs"), EADK)?;
    fs::write(root.join("src/platform.rs"), PLATFORM)?;
    fs::write(root.join("src/numworks.rs"), NUMWORKS_ADVANCED)?;
    fs::write(root.join("src/runtime.rs"), RUNTIME)?;
    fs::write(root.join("src/stdlib.rs"), kalcite_stdlib::RUST_SOURCE)?;
    write_project_data(root, scene_data, assets, input_runtime, save_runtime)?;
    fs::write(
        root.join("src/scene_runtime.rs"),
        scene_runtime
            .map(str::to_string)
            .unwrap_or_else(|| format!("pub type SceneRuntime = crate::game::{};\n", scene.name)),
    )?;
    let game = kalcite_backend_rust::emit_game(program).map_err(Error::Rust)?;
    fs::write(
        root.join("src/game.rs"),
        format!("use crate::numworks::NumWorks;\n{game}"),
    )?;
    fs::write(
        root.join("src/icon.png"),
        include_bytes!("numworks_icon.png"),
    )?;

    let mut name = vec![0u8; app_name.len() + 1];
    name[..app_name.len()].copy_from_slice(app_name.as_bytes());
    let name_len = name.len();
    let name_bytes = name.iter().map(u8::to_string).collect::<Vec<_>>().join(",");

    let update_hook = if scene_runtime.is_some() {
        Some("Update")
    } else {
        lifecycle_name(scene, "Update", "update")
    };
    let draw_hook = if scene_runtime.is_some() {
        Some("Draw")
    } else {
        lifecycle_name(scene, "Draw", "draw")
    };
    let root_type = if scene_runtime.is_some() {
        "scene_runtime::SceneRuntime".to_string()
    } else {
        format!("game::{}", scene.name)
    };
    let main = MAIN
        .replace("__SCENE__", &root_type)
        .replace("__NAME_LEN__", &name_len.to_string())
        .replace("__NAME_BYTES__", &name_bytes)
        .replace(
            "__UPDATE_CALL__",
            &update_hook
                .map(|hook| format!("game.{hook}();"))
                .unwrap_or_default(),
        )
        .replace(
            "__DRAW_CALL__",
            &draw_hook
                .map(|hook| format!("game.{hook}();"))
                .unwrap_or_default(),
        );
    fs::write(root.join("src/main.rs"), main)?;
    Ok(())
}

fn lifecycle_name<'a>(
    scene: &'a kalcite_mir::Class,
    canonical: &'a str,
    legacy: &'a str,
) -> Option<&'a str> {
    scene
        .functions
        .iter()
        .any(|function| function.name == canonical)
        .then_some(canonical)
        .or_else(|| {
            scene
                .functions
                .iter()
                .any(|function| function.name == legacy)
                .then_some(legacy)
        })
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

const CARGO: &str = r#"[package]
name = "kalcite-game"
version = "0.1.0"
edition = "2021"
build = "build.rs"

[profile.release]
opt-level = "z"
lto = "fat"
codegen-units = 1
panic = "abort"
strip = "debuginfo"

[workspace]
"#;

// Keep the generated project isolated from the parent workspace's pinned
// toolchain. Stable is intentional: the official sample only requires an
// embedded ARM Rust compiler and does not depend on nightly features.
const TOOLCHAIN: &str = r#"[toolchain]
channel = "stable"
profile = "minimal"
targets = ["thumbv7em-none-eabihf"]
"#;

// Same link mode as NumWorks' official Rust sample. The output is a relocatable
// ARM ELF; nwlink is responsible for installation, not Kalcite.
const CONFIG: &str = r#"[target.thumbv7em-none-eabihf]
runner = "npx --yes -- nwlink@0.0.19 install-nwa"
rustflags = ["-C", "link-arg=--relocatable", "-C", "link-arg=-no-gc-sections"]

[build]
target = "thumbv7em-none-eabihf"
"#;

const BUILD_RS: &str = r###"use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=src/icon.png");
    std::fs::create_dir_all("target").expect("unable to create target directory");

    // Prefer an installed nwlink. If unavailable (or incompatible with the
    // system Node version), use an isolated Node 18 + nwlink invocation.
    let direct = Command::new("nwlink")
        .args(["png-nwi", "src/icon.png", "target/icon.nwi"])
        .output();

    let output = match direct {
        Ok(output) if output.status.success() => output,
        _ => Command::new("npx")
            .args([
                "--yes",
                "--package=node@18.20.8",
                "--package=nwlink@0.0.19",
                "--",
                "nwlink",
                "png-nwi",
                "src/icon.png",
                "target/icon.nwi",
            ])
            .output()
            .expect("unable to launch nwlink through npx"),
    };

    if !output.status.success() {
        panic!(
            "nwlink png-nwi failed:\n{}{}",
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout),
        );
    }

    let icon = std::fs::read("target/icon.nwi")
        .expect("nwlink did not produce target/icon.nwi");
    assert!(!icon.is_empty(), "nwlink produced an empty icon");

    let out_dir = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    std::fs::write(out_dir.join("icon.nwi"), &icon).unwrap();
    std::fs::write(
        out_dir.join("icon.rs"),
        format!(
            "#[used]\n#[link_section = \".rodata.eadk_app_icon\"]\n\
             pub static EADK_APP_ICON: [u8; {}] = *include_bytes!(concat!(env!(\"OUT_DIR\"), \"/icon.nwi\"));\n",
            icon.len()
        ),
    )
    .unwrap();
}
"###;

const RUNTIME: &str = include_str!("../../kalcite-runtime-core/src/pool.rs");
const ASSET_RUNTIME: &str = include_str!("../../kalcite-engine-assets/src/lib.rs");

// Small Rust transcription of the public EADK ABI used by the engine. Keeping
// this layer separate makes it obvious which calls cross into Epsilon.
const EADK: &str = r#"#![allow(dead_code)]

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Point { pub x:u16, pub y:u16 }

extern "C" {
    fn eadk_display_push_rect_uniform(rect: Rect, color: u16);
    fn eadk_display_push_rect(rect: Rect, pixels: *const u16);
    fn eadk_display_pull_rect(rect: Rect, pixels: *mut u16);
    fn eadk_display_wait_for_vblank() -> bool;
    fn eadk_display_draw_string(text:*const u8, point:Point, large_font:bool, text_color:u16, background_color:u16);
    fn eadk_keyboard_scan() -> u64;
    fn eadk_timing_msleep(ms: u32);
    fn eadk_timing_millis() -> u64;
    fn eadk_backlight_brightness() -> u8;
    fn eadk_random() -> u32;
}

#[inline]
pub fn keyboard_scan() -> u64 { unsafe { eadk_keyboard_scan() } }

#[inline]
fn screen_rect(rect:Rect)->Option<Rect>{
    if rect.x>=320||rect.y>=240||rect.width==0||rect.height==0{return None;}
    let width=core::cmp::min(rect.width,320-rect.x);
    let height=core::cmp::min(rect.height,240-rect.y);
    if width==0||height==0{None}else{Some(Rect{x:rect.x,y:rect.y,width,height})}
}

#[inline]
pub fn push_rect_uniform(rect: Rect, color: u16) {
    if let Some(rect)=screen_rect(rect){unsafe { eadk_display_push_rect_uniform(rect, color) }}
}

#[inline]
pub fn push_rect(rect: Rect, pixels: *const u16) {
    if let Some(rect)=screen_rect(rect){unsafe { eadk_display_push_rect(rect, pixels) }}
}
#[inline]
pub fn pull_rect(rect:Rect,pixels:*mut u16){if let Some(rect)=screen_rect(rect){unsafe{eadk_display_pull_rect(rect,pixels)}}}
#[inline]
pub fn draw_string(text:*const u8,point:Point,large:bool,fg:u16,bg:u16){if point.x<320&&point.y<240{unsafe{eadk_display_draw_string(text,point,large,fg,bg)}}}

#[inline]
pub fn wait_for_vblank() -> bool { unsafe { eadk_display_wait_for_vblank() } }

#[inline]
pub fn msleep(ms: u32) { unsafe { eadk_timing_msleep(ms) } }

#[inline]
pub fn millis() -> u32 { unsafe { eadk_timing_millis() as u32 } }
#[inline] pub fn backlight()->u32{unsafe{eadk_backlight_brightness() as u32}}
// Battery/USB telemetry is not part of the public external-app EADK ABI
// linked by nwlink. Keep deterministic fallbacks here so merely referencing
// Hardware never leaves unresolved firmware symbols in a .nwa.
#[inline] pub fn telemetry_supported()->bool{false}
#[inline] pub fn battery_level()->u32{0}
#[inline] pub fn battery_mv()->u32{0}
#[inline] pub fn charging()->bool{false}
#[inline] pub fn usb_plugged()->bool{false}
#[inline] pub fn random()->u32{unsafe{eadk_random()}}
"#;

const PLATFORM: &str = r#"#![allow(non_upper_case_globals, dead_code)]
use core::ops::{Add, AddAssign, Sub, SubAssign};
use crate::eadk;

#[derive(Clone, Copy, Default)]
pub struct Vec2fx { pub x: i16, pub y: i16 }
impl Vec2fx { pub const fn new(x: i16, y: i16) -> Self { Self { x, y } } }
impl Add for Vec2fx { type Output = Self; fn add(self, r: Self) -> Self { Self { x: self.x + r.x, y: self.y + r.y } } }
impl AddAssign for Vec2fx { fn add_assign(&mut self, r: Self) { self.x += r.x; self.y += r.y; } }
impl Sub for Vec2fx { type Output = Self; fn sub(self, r: Self) -> Self { Self { x: self.x - r.x, y: self.y - r.y } } }
impl SubAssign for Vec2fx { fn sub_assign(&mut self, r: Self) { self.x -= r.x; self.y -= r.y; } }

#[derive(Clone, Copy)]
pub struct Color(pub u16);
impl Color {
    pub const Black: Self = Self(0x0000);
    pub const White: Self = Self(0xffff);
    pub const Red: Self = Self(0xf800);
    pub const Green: Self = Self(0x07e0);
    pub const Blue: Self = Self(0x001f);
    pub const Orange: Self = Self(0xfd20);
    pub const Yellow: Self = Self(0xffe0);
    pub const Cyan: Self = Self(0x07ff);
    pub const Gray: Self = Self(0x8410);
}

#[derive(Clone, Copy)]
pub struct Key(pub u8);
impl Key {
    pub const Left: Self = Self(0);
    pub const Up: Self = Self(1);
    pub const Down: Self = Self(2);
    pub const Right: Self = Self(3);
    pub const Ok: Self = Self(4);
    pub const Back: Self = Self(5);
    pub const Home: Self = Self(6);
}

static mut INPUT_STATE: u64 = 0;
static mut PREV_INPUT_STATE: u64 = 0;

pub struct Input;
impl Input {
    #[inline]
    pub fn begin_frame() { unsafe { PREV_INPUT_STATE=INPUT_STATE;INPUT_STATE = eadk::keyboard_scan(); } }

    #[inline]
    pub fn held(key: Key) -> bool {
        ((unsafe { INPUT_STATE } >> key.0) & 1) != 0
    }
    #[inline] pub fn pressed(key:Key)->bool{let b=1u64<<key.0;unsafe{INPUT_STATE&b!=0&&PREV_INPUT_STATE&b==0}}
    #[inline] pub fn released(key:Key)->bool{let b=1u64<<key.0;unsafe{INPUT_STATE&b==0&&PREV_INPUT_STATE&b!=0}}
    #[inline] pub fn action_held(action:&str)->bool{unsafe{INPUT_STATE&crate::project_data::action_mask(action)!=0}}
    #[inline] pub fn action_pressed(action:&str)->bool{let m=crate::project_data::action_mask(action);unsafe{INPUT_STATE&m!=0&&PREV_INPUT_STATE&m==0}}
    #[inline] pub fn action_released(action:&str)->bool{let m=crate::project_data::action_mask(action);unsafe{INPUT_STATE&m==0&&PREV_INPUT_STATE&m!=0}}
    #[inline] pub fn action_axis(negative:&str,positive:&str)->i16{Self::action_held(positive) as i16-Self::action_held(negative) as i16}
}

pub struct Physics;
impl Physics {
    #[inline] pub fn hit(ax:i16,ay:i16,aw:i16,ah:i16,bx:i16,by:i16,bw:i16,bh:i16)->bool{ax<bx.saturating_add(bw)&&ax.saturating_add(aw)>bx&&ay<by.saturating_add(bh)&&ay.saturating_add(ah)>by}
    #[inline] pub fn move_x(x:i16,y:i16,w:i16,h:i16,dx:i16,sx:i16,sy:i16,sw:i16,sh:i16)->i16{let next=x.saturating_add(dx);if Self::hit(next,y,w,h,sx,sy,sw,sh){x}else{next}}
    #[inline] pub fn move_y(x:i16,y:i16,w:i16,h:i16,dy:i16,sx:i16,sy:i16,sw:i16,sh:i16)->i16{let next=y.saturating_add(dy);if Self::hit(x,next,w,h,sx,sy,sw,sh){y}else{next}}
    #[inline] pub fn circle_hit(ax:i16,ay:i16,ar:i16,bx:i16,by:i16,br:i16)->bool{let dx=i64::from(bx)-i64::from(ax);let dy=i64::from(by)-i64::from(ay);let rr=i64::from(ar.max(0).saturating_add(br.max(0)));dx*dx+dy*dy<rr*rr}
}

static mut AUDIO_COMMANDS:u32=0;
pub struct Audio;
impl Audio { pub fn tone(_hz:u16,_ms:u16,_volume:u8){unsafe{AUDIO_COMMANDS=AUDIO_COMMANDS.saturating_add(1);}}pub fn stop(){}pub fn command_count()->u32{unsafe{AUDIO_COMMANDS}} }

pub struct System;
impl System {
    #[inline]
    pub fn millis() -> u32 { eadk::millis() as u32 }

    #[inline]
    pub fn sleep_ms(ms: u32) { eadk::msleep(ms); }
}

pub struct Hardware;
impl Hardware {
    pub fn is_numworks()->bool{true}
    pub fn telemetry_supported()->bool{eadk::telemetry_supported()}
    pub fn battery_level()->u32{eadk::battery_level()}
    pub fn battery_mv()->u32{eadk::battery_mv()}
    pub fn charging()->bool{eadk::charging()}
    pub fn usb_plugged()->bool{eadk::usb_plugged()}
    pub fn backlight()->u32{eadk::backlight()}
    pub fn random()->u32{eadk::random()}
}

/// Persistent document storage capability.
///
/// EADK does not expose this filesystem. This adapter intentionally mirrors
/// the reverse-engineered Epsilon RAM storage layout used by the community
/// numworks-extapp-storage implementation. Every operation validates SlotInfo,
/// UserlandHeader and filesystem magic values before touching storage.
#[repr(C)]
struct StorageSlotInfo { magic:u32, kernel:*const u8, userland:*const StorageUserlandHeader, footer:u32 }
#[repr(C)]
struct StorageUserlandHeader {
    magic:u32, version:[u8;8], storage:*mut u8, storage_size:u32,
    apps_flash_start:*const u8, apps_flash_end:*const u8,
    apps_ram_start:*const u8, apps_ram_end:*const u8,
    device_name_start:*const u8, device_name_end:*const u8, footer:u32,
}
#[derive(Clone,Copy)]
struct StorageFs { start:*mut u8, usable:*mut u8, end:*mut u8, size:u32 }
#[derive(Clone,Copy)]
struct StorageEntry { start:*mut u8, size:usize, content:*mut u8, content_size:usize }

pub struct Storage;
impl Storage {
    const SLOT_MAGIC:u32=0xEFEEDBBA;
    const USERLAND_MAGIC:u32=0xDEC0EDFE;
    const APP_MAGIC:u32=0xDEC0EDFE;
    const FS_MAGIC:u32=0xEE0BDDBA;

    fn read_u32(addr:usize)->u32 { unsafe { core::ptr::read_unaligned(addr as *const u32) } }
    fn model_ram()->usize {
        let a=[0x90010000usize,0x90410000usize]; let b=[0x90020000usize,0x90420000usize];
        let ca=a.iter().filter(|&&p|Self::read_u32(p)==Self::APP_MAGIC).count();
        let cb=b.iter().filter(|&&p|Self::read_u32(p)==Self::APP_MAGIC).count();
        if ca>cb {0x20000000} else {0x24000000}
    }
    fn fs()->Option<StorageFs> {
        unsafe {
            let slot=&*(Self::model_ram() as *const StorageSlotInfo);
            if slot.magic!=Self::SLOT_MAGIC || slot.footer!=Self::SLOT_MAGIC || slot.userland.is_null(){return None;}
            let u=&*slot.userland;
            if u.magic!=Self::USERLAND_MAGIC || u.footer!=Self::USERLAND_MAGIC || u.storage.is_null() || u.storage_size<4{return None;}
            let header=core::ptr::read_unaligned(u.storage as *const u32);
            let footer=core::ptr::read_unaligned(u.storage.add(4+u.storage_size as usize) as *const u32);
            if header!=Self::FS_MAGIC || footer!=Self::FS_MAGIC{return None;}
            Some(StorageFs{start:u.storage,usable:u.storage.add(4),end:u.storage.add(4+u.storage_size as usize-2),size:u.storage_size-2})
        }
    }
    fn valid_name(name:&str)->bool { !name.is_empty() && name.len()<255 && !name.as_bytes().contains(&0) }
    fn next_free(fs:StorageFs)->Option<*mut u8> {
        unsafe {
            let mut p=fs.usable;
            while p<fs.end {
                let size=core::ptr::read_unaligned(p as *const u16) as usize;
                if size==0{return Some(p);} if size<4 || p.add(size)>fs.end{return None;} p=p.add(size);
            }
            Some(fs.end)
        }
    }
    fn find(name:&str)->Option<StorageEntry> {
        if !Self::valid_name(name){return None;} let fs=Self::fs()?;
        unsafe {
            let mut p=fs.usable;
            while p<fs.end {
                let size=core::ptr::read_unaligned(p as *const u16) as usize;
                if size==0{return None;} if size<4 || p.add(size)>fs.end{return None;}
                let name_start=p.add(2); let record_end=p.add(size); let mut nul=name_start;
                while nul<record_end && *nul!=0 { nul=nul.add(1); }
                if nul>=record_end{return None;}
                let name_len=nul.offset_from(name_start) as usize;
                if name_len==name.len() && core::slice::from_raw_parts(name_start,name_len)==name.as_bytes() {
                    let content=nul.add(1); return Some(StorageEntry{start:p,size,content,content_size:record_end.offset_from(content) as usize});
                }
                p=p.add(size);
            }
        } None
    }
    pub fn supported()->bool{Self::fs().is_some()}
    pub fn write_bytes(name:&str,bytes:&[u8])->bool {
        if !Self::valid_name(name) || bytes.is_empty(){return false;}
        let total=2usize+name.len()+1+bytes.len();
        if total>u16::MAX as usize{return false;}

        // Check capacity before deleting an existing document so a failed
        // overwrite does not destroy the old value.
        let Some(fs)=Self::fs() else{return false;};
        let Some(free)=Self::next_free(fs) else{return false;};
        let free_now=unsafe{fs.end.offset_from(free)};
        if free_now<0{return false;}
        let reclaim=Self::find(name).map(|e|e.size).unwrap_or(0);
        if (free_now as usize).saturating_add(reclaim)<total{return false;}
        if reclaim!=0 && !Self::remove(name){return false;}

        let Some(fs)=Self::fs() else{return false;};
        let Some(p)=Self::next_free(fs) else{return false;};
        unsafe {
            if p.add(total)>fs.end{return false;}
            core::ptr::write_unaligned(p as *mut u16,total as u16);
            core::ptr::copy_nonoverlapping(name.as_ptr(),p.add(2),name.len());
            *p.add(2+name.len())=0;
            core::ptr::copy_nonoverlapping(bytes.as_ptr(),p.add(3+name.len()),bytes.len());
            // Epsilon reserves two bytes after the usable range for the zero
            // size end marker, including when the file reaches fs.end exactly.
            core::ptr::write_unaligned(p.add(total) as *mut u16,0);
        }
        true
    }
    pub fn read_into(name:&str,out:&mut[u8])->usize { let Some(e)=Self::find(name) else{return 0;};let n=core::cmp::min(e.content_size,out.len());unsafe{core::ptr::copy_nonoverlapping(e.content,out.as_mut_ptr(),n);}n }
    pub fn write_text(name:&str,text:&str)->bool{Self::write_bytes(name,text.as_bytes())}
    pub fn exists(name:&str)->bool{Self::find(name).is_some()}
    pub fn size(name:&str)->u32{Self::find(name).map(|e|e.content_size.min(u32::MAX as usize) as u32).unwrap_or(0)}
    pub fn checksum(name:&str)->u32{let Some(e)=Self::find(name) else{return 0;};let mut h=0x811c9dc5u32;unsafe{for &b in core::slice::from_raw_parts(e.content,e.content_size){h^=b as u32;h=h.wrapping_mul(0x01000193);}}h}
    pub fn remove(name:&str)->bool {
        let Some(fs)=Self::fs() else{return false;}; let Some(e)=Self::find(name) else{return true;}; let Some(free)=Self::next_free(fs) else{return false;};
        unsafe { let next=e.start.add(e.size); let tail=free.offset_from(next); if tail<0{return false;} core::ptr::copy(next,e.start,tail as usize); let new_free=free.sub(e.size); core::ptr::write_bytes(new_free,0,e.size); } true
    }
    pub fn free_bytes()->u32{let Some(fs)=Self::fs() else{return 0;};let Some(free)=Self::next_free(fs) else{return 0;};unsafe{fs.end.offset_from(free).max(0) as u32}}
    pub fn total_bytes()->u32{Self::fs().map(|f|f.size).unwrap_or(0)}
}

const SCREEN_W:i16=320;
const SCREEN_H:i16=240;
const SMALL_FONT_W:i16=7;
const SMALL_FONT_H:i16=14;
const MAX_DRAW_COMMANDS:usize=128;
const MAX_DIRTY_RECTS:usize=24;
const TARGET_FRAME_MS:u32=33; // ~30 FPS game cadence; LCD VBlank remains authoritative

#[derive(Clone,Copy,PartialEq,Eq)]
struct ClipRect { x:i16,y:i16,w:i16,h:i16 }
impl ClipRect {
    const EMPTY:Self=Self{x:0,y:0,w:0,h:0};
    const SCREEN:Self=Self{x:0,y:0,w:SCREEN_W,h:SCREEN_H};
    fn clipped(x:i16,y:i16,w:i16,h:i16)->Self {
        if w<=0||h<=0{return Self::EMPTY;}
        let x0=x.clamp(0,SCREEN_W); let y0=y.clamp(0,SCREEN_H);
        let x1=x.saturating_add(w).clamp(0,SCREEN_W); let y1=y.saturating_add(h).clamp(0,SCREEN_H);
        if x1<=x0||y1<=y0{Self::EMPTY}else{Self{x:x0,y:y0,w:x1-x0,h:y1-y0}}
    }
    fn screen_clipped(self)->Self{Self::clipped(self.x,self.y,self.w,self.h)}
    fn empty(self)->bool{self.w<=0||self.h<=0}
    fn valid_screen(self)->bool{!self.empty()&&self.x>=0&&self.y>=0&&self.x+self.w<=SCREEN_W&&self.y+self.h<=SCREEN_H}
    fn intersects(self,o:Self)->bool{let a=self.screen_clipped();let b=o.screen_clipped();!a.empty()&&!b.empty()&&a.x<b.x+b.w&&b.x<a.x+a.w&&a.y<b.y+b.h&&b.y<a.y+a.h}
    fn touches(self,o:Self)->bool{let a=self.screen_clipped();let b=o.screen_clipped();!a.empty()&&!b.empty()&&a.x<=b.x+b.w&&b.x<=a.x+a.w&&a.y<=b.y+b.h&&b.y<=a.y+a.h}
    fn union(self,o:Self)->Self{let a=self.screen_clipped();let b=o.screen_clipped();if a.empty(){return b;}if b.empty(){return a;}let x=a.x.min(b.x);let y=a.y.min(b.y);let x1=(a.x+a.w).max(b.x+b.w);let y1=(a.y+a.h).max(b.y+b.h);Self::clipped(x,y,x1-x,y1-y)}
}

#[derive(Clone,Copy,PartialEq,Eq)]
enum DrawKind { None, Rect, Text, Sprite }
#[derive(Clone,Copy,PartialEq,Eq)]
struct DrawCommand { kind:DrawKind,bounds:ClipRect,color:u16,bg:u16,text:[u8;64],text_len:u8,asset:&'static[u8],source:ClipRect,origin_x:i16,origin_y:i16 }
impl DrawCommand { const EMPTY:Self=Self{kind:DrawKind::None,bounds:ClipRect::EMPTY,color:0,bg:0,text:[0;64],text_len:0,asset:&[],source:ClipRect::EMPTY,origin_x:0,origin_y:0}; }

struct Renderer {
    current:[DrawCommand;MAX_DRAW_COMMANDS], previous:[DrawCommand;MAX_DRAW_COMMANDS],
    current_len:usize, previous_len:usize, background:u16, previous_background:u16,
    first_frame:bool, immediate:bool,
}
impl Renderer {
    const fn new()->Self{Self{current:[DrawCommand::EMPTY;MAX_DRAW_COMMANDS],previous:[DrawCommand::EMPTY;MAX_DRAW_COMMANDS],current_len:0,previous_len:0,background:0,previous_background:0,first_frame:true,immediate:false}}
    fn begin(&mut self){self.current_len=0;self.immediate=false;self.background=self.previous_background;}
    fn push(&mut self,c:DrawCommand){
        if c.bounds.empty(){return;}
        if self.immediate{let screen=ClipRect::SCREEN;Self::render_command(c,screen);return;}
        if self.current_len<MAX_DRAW_COMMANDS{self.current[self.current_len]=c;self.current_len+=1;return;}
        // A pathological frame must remain correct: flush the display list once,
        // then stream the rest directly instead of dropping draw calls.
        self.full_present();self.immediate=true;let screen=ClipRect::SCREEN;Self::render_command(c,screen);
    }
    fn dirty_add(dirty:&mut[ClipRect;MAX_DIRTY_RECTS],len:&mut usize,mut r:ClipRect)->bool{
        r=r.screen_clipped();
        if r.empty(){return true;} let mut i=0;
        while i<*len {if dirty[i].touches(r){r=dirty[i].union(r).screen_clipped();dirty[i]=dirty[*len-1];*len-=1;i=0;}else{i+=1;}}
        if !r.valid_screen()||*len>=MAX_DIRTY_RECTS{return false;}dirty[*len]=r;*len+=1;true
    }
    fn render_command(c:DrawCommand,clip:ClipRect){
        let b=c.bounds.screen_clipped();
        let clip=clip.screen_clipped();
        if b.empty()||!b.valid_screen()||!b.intersects(clip){return;}
        match c.kind {
            DrawKind::Rect=>{eadk::push_rect_uniform(eadk::Rect{x:b.x as u16,y:b.y as u16,width:b.w as u16,height:b.h as u16},c.color);}
            DrawKind::Text=>{
                if b.y<0||b.y+SMALL_FONT_H>SCREEN_H||c.text_len==0{return;}
                eadk::draw_string(c.text.as_ptr(),eadk::Point{x:b.x as u16,y:b.y as u16},false,c.color,c.bg);
            }
            DrawKind::Sprite=>Self::render_sprite(c,clip),
            DrawKind::None=>{}
        }
    }
    fn render_sprite(c:DrawCommand,clip:ClipRect){
        let data=c.asset;if data.len()<4{return;}let width=u16::from_le_bytes([data[0],data[1]]) as usize;let mut pixel=0usize;let mut at=4usize;
        while at+4<=data.len(){let count=data[at] as usize;let transparent=data[at+1]!=0;let color=u16::from_le_bytes([data[at+2],data[at+3]]);at+=4;if count==0{return;}
            let row=pixel/width;let col=pixel%width;pixel+=count;if transparent||row<c.source.y as usize||row>=(c.source.y+c.source.h) as usize{continue;}
            let start=col.max(c.source.x as usize);let end=(col+count).min((c.source.x+c.source.w) as usize);if end<=start{continue;}
            let dy=c.origin_y+row as i16-c.source.y;if dy<clip.y||dy>=clip.y+clip.h{continue;}let dx=c.origin_x+start as i16-c.source.x;let dx1=c.origin_x+end as i16-c.source.x;
            let x0=dx.max(clip.x).max(0);let x1=dx1.min(clip.x+clip.w).min(SCREEN_W);if x1>x0{eadk::push_rect_uniform(eadk::Rect{x:x0 as u16,y:dy as u16,width:(x1-x0) as u16,height:1},color);}
        }
    }
    fn full_present(&mut self){
        eadk::push_rect_uniform(eadk::Rect{x:0,y:0,width:SCREEN_W as u16,height:SCREEN_H as u16},self.background);
        let screen=ClipRect::SCREEN; for i in 0..self.current_len{Self::render_command(self.current[i],screen);}
    }
    fn present(&mut self){
        if self.immediate{self.previous_len=0;self.previous_background=self.background;self.first_frame=true;return;}
        if self.first_frame||self.background!=self.previous_background{self.full_present();}
        else {
            let mut dirty=[ClipRect::EMPTY;MAX_DIRTY_RECTS];let mut dirty_len=0usize;let n=self.current_len.max(self.previous_len);let mut fallback=false;
            for i in 0..n {
                let a=if i<self.previous_len{self.previous[i]}else{DrawCommand::EMPTY};
                let b=if i<self.current_len{self.current[i]}else{DrawCommand::EMPTY};
                if a!=b {
                    // EADK text drawing is opaque but its exact glyph advance is firmware-owned.
                    // A changed text command therefore gets a conservative full redraw instead
                    // of trying to erase a guessed glyph rectangle. Unchanged text stays free.
                    if a.kind==DrawKind::Text||b.kind==DrawKind::Text{fallback=true;break;}
                    if !Self::dirty_add(&mut dirty,&mut dirty_len,a.bounds)||!Self::dirty_add(&mut dirty,&mut dirty_len,b.bounds){fallback=true;break;}
                }
            }
            if !fallback {
                // If a command crosses a dirty boundary, include its complete bounds.
                // This prevents opaque text/rect replay from damaging unchanged pixels.
                let mut changed=true;while changed{changed=false;let before=dirty_len;for i in 0..self.current_len{for d in 0..dirty_len{if self.current[i].bounds.intersects(dirty[d])&&!dirty[d].union(self.current[i].bounds).eq(&dirty[d]){if !Self::dirty_add(&mut dirty,&mut dirty_len,self.current[i].bounds){fallback=true;break;}changed=true;break;}}if fallback{break;}}if dirty_len==before&&!changed{break;}}
            }
            let area:i32=dirty[..dirty_len].iter().map(|r|(r.w as i32)*(r.h as i32)).sum();
            if fallback||area>(SCREEN_W as i32*SCREEN_H as i32)/2{self.full_present();}else{
                for raw in dirty[..dirty_len].iter().copied(){
                    let d=raw.screen_clipped();
                    if d.empty()||!d.valid_screen(){self.full_present();break;}
                    eadk::push_rect_uniform(eadk::Rect{x:d.x as u16,y:d.y as u16,width:d.w as u16,height:d.h as u16},self.background);
                    for i in 0..self.current_len{Self::render_command(self.current[i],d);}
                }
            }
        }
        self.previous[..self.current_len].copy_from_slice(&self.current[..self.current_len]);
        self.previous_len=self.current_len;self.previous_background=self.background;self.first_frame=false;
    }
}
static mut RENDERER:Renderer=Renderer::new();
const MAX_RAYTRACE_BLOCKS:usize=6;
#[derive(Clone,Copy)]struct RaytraceBlock{x:i16,y:i16,w:i16,h:i16,color:u16}
impl RaytraceBlock{const EMPTY:Self=Self{x:0,y:0,w:0,h:0,color:0};}
static mut RAYTRACE_BLOCKS:[RaytraceBlock;MAX_RAYTRACE_BLOCKS]=[RaytraceBlock::EMPTY;MAX_RAYTRACE_BLOCKS];
static mut RAYTRACE_BLOCK_LEN:usize=0;
static mut CAMERA_X:i16=0;static mut CAMERA_Y:i16=0;
fn world_to_screen(x:i16,y:i16)->(i16,i16){unsafe{(x.saturating_sub(CAMERA_X),y.saturating_sub(CAMERA_Y))}}

pub struct Draw;
impl Draw {
    pub fn camera(x:i16,y:i16){unsafe{CAMERA_X=x;CAMERA_Y=y;}}
    #[inline] pub fn begin_frame(){unsafe{RENDERER.begin();}}
    #[inline] pub fn clear(color:Color){unsafe{RENDERER.background=color.0;}}
    pub fn rect(x:i16,y:i16,width:i16,height:i16,color:Color){let b=ClipRect::clipped(x,y,width,height);unsafe{RENDERER.push(DrawCommand{kind:DrawKind::Rect,bounds:b,color:color.0,bg:0,text:[0;64],text_len:0,asset:&[],source:ClipRect::EMPTY,origin_x:0,origin_y:0});}}
    // EADK's display queue is bounded. Keep one command per particle; the
    // desktop backend provides the exact filled-circle rasterization.
    pub fn circle(cx:i16,cy:i16,r:i16,color:Color){if r>0{Self::rect(cx.saturating_sub(r),cy.saturating_sub(r),r.saturating_mul(2).saturating_add(1),r.saturating_mul(2).saturating_add(1),color);}}
    pub fn line(x0:i16,y0:i16,x1:i16,y1:i16,color:Color){let steps=(x1.saturating_sub(x0).abs().max(y1.saturating_sub(y0).abs())/8).max(1);for step in 0..=steps{let x=x0.saturating_add(x1.saturating_sub(x0).saturating_mul(step)/steps);let y=y0.saturating_add(y1.saturating_sub(y0).saturating_mul(step)/steps);Self::rect(x.saturating_sub(1),y.saturating_sub(1),3,3,color);}}
    pub fn glow(cx:i16,cy:i16,r:u16,color:Color,energy:u16){let radius=(u16::min(r,120)as i16).saturating_mul(energy.min(100)as i16)/100;for scale in [3i16,2,1]{let size=radius.saturating_mul(scale)/3;if size>0{Self::rect(cx.saturating_sub(size),cy.saturating_sub(size),size.saturating_mul(2),size.saturating_mul(2),color);}}}
    pub fn raytrace_block(x:i16,y:i16,w:i16,h:i16,color:Color){let b=ClipRect::clipped(x,y,w,h);if b.empty(){return;}unsafe{if RAYTRACE_BLOCK_LEN<MAX_RAYTRACE_BLOCKS{RAYTRACE_BLOCKS[RAYTRACE_BLOCK_LEN]=RaytraceBlock{x:b.x,y:b.y,w:b.w,h:b.h,color:color.0};RAYTRACE_BLOCK_LEN+=1;}}}
    pub fn sprite(name:&str,x:i16,y:i16){let Ok(pack)=crate::project_data::AssetPack::new(&crate::project_data::ASSET_PACK)else{return;};let Some(asset)=pack.get_named(name)else{return;};let(x,y)=world_to_screen(x,y);Self::sprite_data(asset.data,x,y,0,0,u16::MAX,u16::MAX);}
    pub fn sprite_region(name:&str,x:i16,y:i16,sx:u16,sy:u16,w:u16,h:u16){let Ok(pack)=crate::project_data::AssetPack::new(&crate::project_data::ASSET_PACK)else{return;};let Some(asset)=pack.get_named(name)else{return;};let(x,y)=world_to_screen(x,y);Self::sprite_data(asset.data,x,y,sx,sy,w,h);}
    pub fn sprite_frame(sheet:&str,frame:u16,x:i16,y:i16){let Ok(pack)=crate::project_data::AssetPack::new(&crate::project_data::ASSET_PACK)else{return;};let Some(meta)=pack.get_named(sheet)else{return;};if meta.kind!=3||meta.data.len()!=12{return;}let image=u64::from_le_bytes(meta.data[..8].try_into().unwrap());let fw=u16::from_le_bytes(meta.data[8..10].try_into().unwrap());let fh=u16::from_le_bytes(meta.data[10..12].try_into().unwrap());let Some(sprite)=pack.get(image)else{return;};if sprite.data.len()<4||fw==0||fh==0{return;}let width=u16::from_le_bytes([sprite.data[0],sprite.data[1]]);let cols=width/fw;if cols==0{return;}let(x,y)=world_to_screen(x,y);Self::sprite_data(sprite.data,x,y,(frame%cols)*fw,(frame/cols)*fh,fw,fh);}
    pub fn tilemap(map:&str,tileset:&str,tile_w:u16,tile_h:u16,x:i16,y:i16){let Ok(pack)=crate::project_data::AssetPack::new(&crate::project_data::ASSET_PACK)else{return;};let Some(map)=pack.get_named(map)else{return;};let Some(sprite)=pack.get_named(tileset)else{return;};if map.kind!=2||sprite.kind!=1||map.data.len()<4||sprite.data.len()<4||tile_w==0||tile_h==0{return;}let mw=u16::from_le_bytes([map.data[0],map.data[1]])as usize;let mh=u16::from_le_bytes([map.data[2],map.data[3]])as usize;let sw=u16::from_le_bytes([sprite.data[0],sprite.data[1]]);let cols=sw/tile_w;if cols==0{return;}for row in 0..mh{for col in 0..mw{let at=4+(row*mw+col)*2;if at+2>map.data.len(){return;}let tile=u16::from_le_bytes([map.data[at],map.data[at+1]]);let(dx,dy)=world_to_screen(x+col as i16*tile_w as i16,y+row as i16*tile_h as i16);Self::sprite_data(sprite.data,dx,dy,(tile%cols)*tile_w,(tile/cols)*tile_h,tile_w,tile_h);}}}
    fn sprite_data(data:&'static[u8],x:i16,y:i16,sx:u16,sy:u16,requested_w:u16,requested_h:u16){if data.len()<4{return;}let width=u16::from_le_bytes([data[0],data[1]]);let height=u16::from_le_bytes([data[2],data[3]]);let sw=requested_w.min(width.saturating_sub(sx));let sh=requested_h.min(height.saturating_sub(sy));if sw==0||sh==0{return;}let bounds=ClipRect::clipped(x,y,sw as i16,sh as i16);let source=ClipRect{x:sx as i16,y:sy as i16,w:sw as i16,h:sh as i16};unsafe{RENDERER.push(DrawCommand{kind:DrawKind::Sprite,bounds,color:0,bg:0,text:[0;64],text_len:0,asset:data,source,origin_x:x,origin_y:y});}}
    pub fn pixel_at(x:i16,y:i16)->u32{if x<0||y<0||x>=SCREEN_W||y>=SCREEN_H{return 0;}let _=eadk::wait_for_vblank();unsafe{RENDERER.present();}let mut pixel=0u16;eadk::pull_rect(eadk::Rect{x:x as u16,y:y as u16,width:1,height:1},&mut pixel as *mut u16);pixel as u32}
    pub fn text(text:&str,x:i16,y:i16,c:Color,bg:Color){
        if y<0||y+SMALL_FONT_H>SCREEN_H||x>=SCREEN_W{return;}
        let bytes=text.as_bytes();let n=core::cmp::min(bytes.len(),63);if n==0{return;}
        // Never ask EADK to draw a partially off-screen glyph. Skip whole glyphs
        // on the left and truncate whole glyphs on the right before queuing.
        let skip=if x<0{(((-x) as i32+SMALL_FONT_W as i32-1)/SMALL_FONT_W as i32) as usize}else{0};
        if skip>=n{return;}
        let draw_x=x.saturating_add((skip as i16).saturating_mul(SMALL_FONT_W)).max(0);
        let max_chars=((SCREEN_W-draw_x)/SMALL_FONT_W).max(0) as usize;
        let count=(n-skip).min(max_chars).min(63);if count==0{return;}
        let width=(count as i16).saturating_mul(SMALL_FONT_W);
        let b=ClipRect::clipped(draw_x,y,width,SMALL_FONT_H);if b.empty()||!b.valid_screen(){return;}
        let mut buf=[0u8;64];buf[..count].copy_from_slice(&bytes[skip..skip+count]);buf[count]=0;
        unsafe{RENDERER.push(DrawCommand{kind:DrawKind::Text,bounds:b,color:c.0,bg:bg.0,text:buf,text_len:count as u8,asset:&[],source:ClipRect::EMPTY,origin_x:0,origin_y:0});}
    }
    pub fn number<T:Into<u64>+Copy>(value:T,x:i16,y:i16,c:Color,bg:Color){let mut value:u64=value.into();let mut tmp=[0u8;20];let mut n=0usize;if value==0{tmp[0]=b'0';n=1;}else{while value>0&&n<19{tmp[n]=b'0'+(value%10) as u8;value/=10;n+=1;}tmp[..n].reverse();}let s=unsafe{core::str::from_utf8_unchecked(&tmp[..n])};Self::text(s,x,y,c,bg);}
    #[inline] pub fn present(){unsafe{RENDERER.present();for index in 0..RAYTRACE_BLOCK_LEN{let block=RAYTRACE_BLOCKS[index];eadk::push_rect_uniform(eadk::Rect{x:block.x as u16,y:block.y as u16,width:block.w as u16,height:block.h as u16},block.color);}RAYTRACE_BLOCK_LEN=0;}}
}

pub struct Time;
impl Time {
    #[inline]
    pub fn millis() -> u32 { eadk::millis() as u32 }
}

static mut FRAME_START:u32=0;
#[inline]
pub fn frame_begin(){unsafe{FRAME_START=eadk::millis();}Input::begin_frame();Draw::begin_frame();}
#[inline]
pub fn frame_end() {
    // KLC draw calls only build a tiny display list during the frame. Do not
    // touch the LCD while it is scanning: first pace the game at ~30 FPS,
    // then wait for the next VBlank and present immediately afterwards.
    // On a ~50 Hz panel VBlank is authoritative, so the real cadence may
    // quantize to the display refresh instead of tearing mid-scan.
    let elapsed=eadk::millis().wrapping_sub(unsafe{FRAME_START});
    if elapsed<TARGET_FRAME_MS{eadk::msleep(TARGET_FRAME_MS-elapsed);}
    let _ = eadk::wait_for_vblank();
    Draw::present();
}
"#;

// Platform-specific escape hatch. Manual SVCs are intentionally kept out of
// the portable System API because their indexes are not stable EADK ABI.
// Nwagyu documents SVC 44 as POWER_SUSPEND, but also warns that manual SVC
// numbers can change between Epsilon releases. Use only in NumWorks-specific
// game code and test on every supported firmware version.
const NUMWORKS_ADVANCED: &str = r#"#![allow(dead_code)]

pub struct NumWorks;
impl NumWorks {
    /// Suspend through the kernel SVC table instead of EADK.
    ///
    /// This is an unofficial, firmware-sensitive API. It is deliberately
    /// named `unsafe_suspend` so the portability cost is visible in Kalcite.
    #[inline(never)]
    pub fn unsafe_suspend() {
        #[cfg(target_arch = "arm")]
        unsafe { core::arch::asm!("svc 44", options(nomem, nostack)); }
    }
}
"#;

const MAIN: &str = r#"#![no_std]
#![no_main]
mod eadk;
mod platform;
mod runtime;
mod stdlib;
mod numworks;
mod project_data;
#[allow(dead_code, unused_mut)]
mod scene_runtime;
#[allow(dead_code, non_camel_case_types, non_snake_case, unused_imports, unused_mut, unused_parens, unused_variables)]
mod game;

use core::panic::PanicInfo;
use platform::{Input, Key};

#[used]
#[link_section = ".rodata.eadk_app_name"]
pub static EADK_APP_NAME: [u8; __NAME_LEN__] = [__NAME_BYTES__];

#[used]
#[link_section = ".rodata.eadk_api_level"]
pub static EADK_APP_API_LEVEL: u32 = 0;

include!(concat!(env!("OUT_DIR"), "/icon.rs"));

#[no_mangle]
pub fn main() {
    let mut game = __SCENE__::default();
    loop {
        platform::frame_begin();
        if Input::held(Key::Back) || Input::held(Key::Home) { break; }
        __UPDATE_CALL__
        __DRAW_CALL__
        platform::frame_end();
    }
}

#[panic_handler]
fn panic(_panic: &PanicInfo<'_>) -> ! {
    // External apps cannot unwind. Keep panic behavior deterministic and avoid
    // touching heap/formatting paths on the constrained target.
    loop {}
}
"#;

#[cfg(test)]
mod abi_regression_tests {
    use super::*;

    #[test]
    fn dirty_renderer_never_keeps_unclipped_text_bounds() {
        assert!(PLATFORM.contains("bounds:b,color:c.0"));
        assert!(!PLATFORM.contains("bounds:ClipRect{x,y,w:full_w"));
        assert!(PLATFORM.contains("screen_clipped"));
        assert!(PLATFORM.contains("a.kind==DrawKind::Text||b.kind==DrawKind::Text"));
        assert!(EADK.contains("fn screen_rect(rect:Rect)->Option<Rect>"));
    }

    #[test]
    fn unsupported_battery_symbols_are_not_emitted() {
        for symbol in [
            "eadk_battery_level",
            "eadk_battery_voltage",
            "eadk_usb_is_plugged",
            "eadk_battery_is_charging",
        ] {
            assert!(
                !EADK.contains(symbol),
                "unsupported external-app ABI symbol leaked: {symbol}"
            );
        }
        assert!(EADK.contains("telemetry_supported()->bool{false}"));
    }

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
    fn numworks_presents_after_vblank_at_30fps_cadence() {
        assert!(PLATFORM.contains("const TARGET_FRAME_MS:u32=33"));
        let frame_end = PLATFORM
            .split("pub fn frame_end() {")
            .nth(1)
            .expect("frame_end");
        let wait = frame_end.find("wait_for_vblank").expect("vblank wait");
        let present = frame_end.find("Draw::present()").expect("present");
        assert!(wait < present, "LCD writes must start after VBlank");
        assert!(PLATFORM.contains("if elapsed<TARGET_FRAME_MS"));
    }

    #[test]
    fn numworks_renderer_is_bounded_and_incremental() {
        assert!(PLATFORM.contains("MAX_DRAW_COMMANDS"));
        assert!(PLATFORM.contains("MAX_DIRTY_RECTS"));
        assert!(PLATFORM.contains("ClipRect::clipped"));
        assert!(PLATFORM.contains("self.full_present();self.immediate=true"));
        assert!(PLATFORM.contains("SMALL_FONT_W"));
        assert!(MAIN.contains("platform::frame_begin();"));
    }

    #[test]
    fn sprites_render_as_bounded_horizontal_runs() {
        assert!(PLATFORM.contains("DrawKind::Sprite=>Self::render_sprite"));
        assert!(PLATFORM.contains("height:1},color"));
        assert!(PLATFORM.contains("Self::sprite_data(asset.data"));
        assert!(!PLATFORM.contains("for pixel in"));
    }

    #[test]
    fn generated_project_data_embeds_exact_resources() {
        let root = std::env::temp_dir().join(format!(
            "kalcite-numworks-data-{}-{}",
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
