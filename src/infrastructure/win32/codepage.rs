//! Reading text in the OEM code page, which hidden console programs write in. Measured on
//! 2026-09-27 (OEM code page 850): pwsh, Windows PowerShell and cmd all wrote `é` as the single
//! byte 0x82 (SRS Amendment 2).

use std::ptr::null_mut;

use windows_sys::Win32::Globalization::{GetOEMCP, MultiByteToWideChar};

use crate::domain::text::lossy_utf8;

/// `bytes` decoded in the OEM code page; lossy UTF-8 if Windows cannot decode them.
pub fn decode_oem(bytes: &[u8]) -> String {
    let Ok(length) = i32::try_from(bytes.len()) else {
        return lossy_utf8(bytes);
    };
    if length == 0 {
        return String::new();
    }
    // SAFETY: plain call with no arguments.
    let code_page = unsafe { GetOEMCP() };
    // SAFETY: `bytes` is valid for `length` bytes; a null output buffer asks for the size.
    let needed =
        unsafe { MultiByteToWideChar(code_page, 0, bytes.as_ptr(), length, null_mut(), 0) };
    let Ok(capacity) = usize::try_from(needed) else {
        return lossy_utf8(bytes);
    };
    if capacity == 0 {
        return lossy_utf8(bytes);
    }
    let mut wide = vec![0u16; capacity];
    // SAFETY: `wide` holds exactly `needed` UTF-16 units, the size Windows asked for.
    let written = unsafe {
        MultiByteToWideChar(
            code_page,
            0,
            bytes.as_ptr(),
            length,
            wide.as_mut_ptr(),
            needed,
        )
    };
    match usize::try_from(written) {
        Ok(count) if count > 0 => String::from_utf16_lossy(&wide[..count]),
        _ => lossy_utf8(bytes),
    }
}
