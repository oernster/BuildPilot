//! One HTTPS GET through WinHTTP, the HTTP client built into Windows (NFR-SEC-001): no HTTP or
//! TLS crate is needed. Used by the update check (UI-010) and nothing else.

use std::ffi::c_void;
use std::iter::once;
use std::ptr::{null, null_mut};

use windows_sys::Win32::Networking::WinHttp::{
    INTERNET_DEFAULT_HTTPS_PORT, WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY, WINHTTP_FLAG_SECURE,
    WINHTTP_QUERY_FLAG_NUMBER, WINHTTP_QUERY_STATUS_CODE, WinHttpCloseHandle, WinHttpConnect,
    WinHttpOpen, WinHttpOpenRequest, WinHttpQueryHeaders, WinHttpReadData, WinHttpReceiveResponse,
    WinHttpSendRequest, WinHttpSetTimeouts,
};

/// Win32 BOOL false.
const FALSE: i32 = 0;
/// The HTTP status of a successful GET.
const HTTP_OK: u32 = 200;
/// How much is read from the response at a time.
const CHUNK_BYTES: usize = 16 * 1024;

/// A request to make.
pub struct Get<'a> {
    /// The server, such as `api.github.com`.
    pub host: &'a str,
    /// The path and query, such as `/repos/owner/name/releases/latest`.
    pub path: &'a str,
    /// Extra request headers, each ending in CRLF.
    pub headers: &'a str,
    /// What the request calls itself; some servers refuse a request without one.
    pub user_agent: &'a str,
    /// How long each stage (resolving, connecting, sending, receiving) may take.
    pub timeout_ms: i32,
    /// The most bytes of body accepted; a longer answer is refused rather than held.
    pub max_body: usize,
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(once(0)).collect()
}

/// A WinHTTP handle, closed when dropped.
struct Handle(*mut c_void);

impl Handle {
    fn checked(raw: *mut c_void, stage: &str) -> Result<Self, String> {
        if raw.is_null() {
            Err(format!(
                "{stage} failed: {}",
                std::io::Error::last_os_error()
            ))
        } else {
            Ok(Self(raw))
        }
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        // SAFETY: the handle came from WinHTTP and is closed exactly once, here.
        unsafe { WinHttpCloseHandle(self.0) };
    }
}

fn ok(result: i32, stage: &str) -> Result<(), String> {
    if result == FALSE {
        Err(format!(
            "{stage} failed: {}",
            std::io::Error::last_os_error()
        ))
    } else {
        Ok(())
    }
}

/// Makes `request` over HTTPS; the body of a 200 answer, else what went wrong in words.
pub fn get(request: &Get<'_>) -> Result<Vec<u8>, String> {
    let (agent, host, path) = (
        wide(request.user_agent),
        wide(request.host),
        wide(request.path),
    );
    let (verb, headers) = (wide("GET"), wide(request.headers));
    let timeout = request.timeout_ms;
    // SAFETY: every string is NUL-terminated UTF-16 that outlives the calls using it; every
    // handle is checked before use and closed by its guard; buffers are sized as passed.
    unsafe {
        let session = Handle::checked(
            WinHttpOpen(
                agent.as_ptr(),
                WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
                null(),
                null(),
                0,
            ),
            "Opening WinHTTP",
        )?;
        ok(
            WinHttpSetTimeouts(session.0, timeout, timeout, timeout, timeout),
            "Setting the timeouts",
        )?;
        let connection = Handle::checked(
            WinHttpConnect(session.0, host.as_ptr(), INTERNET_DEFAULT_HTTPS_PORT, 0),
            "Connecting",
        )?;
        let call = Handle::checked(
            WinHttpOpenRequest(
                connection.0,
                verb.as_ptr(),
                path.as_ptr(),
                null(),
                null(),
                null(),
                WINHTTP_FLAG_SECURE,
            ),
            "Opening the request",
        )?;
        let header_units = u32::try_from(headers.len() - 1).unwrap_or(0);
        ok(
            WinHttpSendRequest(call.0, headers.as_ptr(), header_units, null(), 0, 0, 0),
            "Sending the request",
        )?;
        ok(
            WinHttpReceiveResponse(call.0, null_mut()),
            "Receiving the answer",
        )?;
        let mut status = 0u32;
        let mut size = size_of::<u32>() as u32;
        ok(
            WinHttpQueryHeaders(
                call.0,
                WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
                null(),
                (&raw mut status).cast::<c_void>(),
                &mut size,
                null_mut(),
            ),
            "Reading the status",
        )?;
        if status != HTTP_OK {
            return Err(format!("the server answered {status}"));
        }
        let mut body = Vec::new();
        let mut chunk = vec![0u8; CHUNK_BYTES];
        loop {
            let mut read = 0u32;
            ok(
                WinHttpReadData(
                    call.0,
                    chunk.as_mut_ptr().cast::<c_void>(),
                    chunk.len() as u32,
                    &mut read,
                ),
                "Reading the answer",
            )?;
            if read == 0 {
                return Ok(body);
            }
            body.extend_from_slice(&chunk[..read as usize]);
            if body.len() > request.max_body {
                return Err(format!(
                    "the answer was longer than {} bytes",
                    request.max_body
                ));
            }
        }
    }
}
