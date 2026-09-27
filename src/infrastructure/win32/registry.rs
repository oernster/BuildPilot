//! Values under `HKEY_CURRENT_USER`, for the setup program's Apps list entry (INST-002). Per
//! user, so nothing here needs administrator rights.

use std::ffi::c_void;
use std::io;
use std::iter::once;
use std::mem::size_of;
use std::ptr::null_mut;

use windows_sys::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS, WIN32_ERROR};
use windows_sys::Win32::System::Registry::{
    HKEY_CURRENT_USER, REG_DWORD, REG_SZ, RRF_RT_REG_SZ, RegDeleteTreeW, RegGetValueW,
    RegSetKeyValueW,
};

/// Bytes in one UTF-16 code unit.
const UNIT_BYTES: usize = size_of::<u16>();
/// Longest string value read, in code units; the entry's own values are far shorter.
const MAX_VALUE_UNITS: usize = 2048;

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(once(0)).collect()
}

fn check(status: WIN32_ERROR) -> io::Result<()> {
    if status == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(io::Error::from_raw_os_error(status as i32))
    }
}

/// The string `value` under `key`; `None` when the key or the value is absent or unreadable.
pub fn read_string(key: &str, value: &str) -> Option<String> {
    let (key, value) = (wide(key), wide(value));
    let mut buffer = vec![0u16; MAX_VALUE_UNITS];
    let mut size = (buffer.len() * UNIT_BYTES) as u32;
    // SAFETY: the names are NUL-terminated; `buffer` holds `size` bytes.
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            key.as_ptr(),
            value.as_ptr(),
            RRF_RT_REG_SZ,
            null_mut(),
            buffer.as_mut_ptr().cast::<c_void>(),
            &mut size,
        )
    };
    if status != ERROR_SUCCESS {
        return None;
    }
    let units = (size as usize / UNIT_BYTES).saturating_sub(1);
    Some(String::from_utf16_lossy(&buffer[..units]))
}

/// Writes `value` = `text` under `key`, creating the key.
pub fn write_string(key: &str, value: &str, text: &str) -> io::Result<()> {
    let (key, value, text) = (wide(key), wide(value), wide(text));
    // SAFETY: the names and the data are NUL-terminated UTF-16 that outlive the call.
    check(unsafe {
        RegSetKeyValueW(
            HKEY_CURRENT_USER,
            key.as_ptr(),
            value.as_ptr(),
            REG_SZ,
            text.as_ptr().cast::<c_void>(),
            (text.len() * UNIT_BYTES) as u32,
        )
    })
}

/// Writes `value` = `number` under `key`, creating the key.
pub fn write_number(key: &str, value: &str, number: u32) -> io::Result<()> {
    let (key, value) = (wide(key), wide(value));
    // SAFETY: the names are NUL-terminated; the data is one DWORD.
    check(unsafe {
        RegSetKeyValueW(
            HKEY_CURRENT_USER,
            key.as_ptr(),
            value.as_ptr(),
            REG_DWORD,
            (&raw const number).cast::<c_void>(),
            size_of::<u32>() as u32,
        )
    })
}

/// Deletes `key` and everything under it; a key already gone is not an error.
pub fn delete_tree(key: &str) -> io::Result<()> {
    let key = wide(key);
    // SAFETY: the name is NUL-terminated.
    let status = unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, key.as_ptr()) };
    if status == ERROR_FILE_NOT_FOUND {
        return Ok(());
    }
    check(status)
}
