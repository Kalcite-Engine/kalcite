#![no_std]

use kalcite_platform_api::{Buttons, Platform};

/// Portable NumWorks platform adapter.
///
/// The stable surface only exposes public EADK-backed hooks. Firmware-sensitive
/// SVC helpers belong in the generated `kalcite-backend-numworks` project and
/// must not leak into the cross-platform engine contract.
pub struct NumWorks;

#[cfg(feature = "numworks-ffi")]
unsafe extern "C" {
    fn kalcite_nw_ticks_ms() -> u32;
    fn kalcite_nw_scan_keys() -> u32;
    fn kalcite_nw_present_rgb565(ptr: *const u16, len: usize);
}

impl Platform for NumWorks {
    fn width(&self) -> u16 { 320 }
    fn height(&self) -> u16 { 240 }

    fn ticks_ms(&self) -> u32 {
        #[cfg(feature = "numworks-ffi")]
        unsafe { return kalcite_nw_ticks_ms(); }
        #[cfg(not(feature = "numworks-ffi"))]
        { 0 }
    }

    fn buttons(&mut self) -> Buttons {
        #[cfg(feature = "numworks-ffi")]
        unsafe { return Buttons(kalcite_nw_scan_keys()); }
        #[cfg(not(feature = "numworks-ffi"))]
        { Buttons(0) }
    }

    fn present(&mut self, pixels: &[u16]) {
        #[cfg(feature = "numworks-ffi")]
        unsafe { kalcite_nw_present_rgb565(pixels.as_ptr(), pixels.len()); }
        #[cfg(not(feature = "numworks-ffi"))]
        let _ = pixels;
    }
}
