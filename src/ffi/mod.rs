pub mod context;
pub mod expression;
pub mod router;
pub mod schema;

use crate::ast::Value;
use cidr::IpCidr;
use std::convert::TryFrom;
use std::ffi;
use std::fmt::Display;
use std::net::IpAddr;
use std::os::raw::c_char;
use std::slice::from_raw_parts;
use std::slice::from_raw_parts_mut;

/// A _suggestion_ of the value to use for `errbuf_len`
///
/// This value is actually not used for anything in this library,
/// any length can be passed.
#[deprecated]
pub const ERR_BUF_MAX_LEN: usize = 4096;

#[derive(Debug)]
#[repr(C)]
/// cbindgen:prefix-with-name
pub enum CValue {
    Str(*const u8, usize),
    IpCidr(*const u8),
    IpAddr(*const u8),
    Int(i64),
}

impl TryFrom<&CValue> for Value {
    type Error = String;

    fn try_from(v: &CValue) -> Result<Self, Self::Error> {
        Ok(match v {
            CValue::Str(s, len) => Self::String(unsafe {
                std::str::from_utf8(from_raw_parts(*s, *len))
                    .map_err(|e| e.to_string())?
                    .to_string()
            }),
            CValue::IpCidr(s) => Self::IpCidr(
                unsafe {
                    ffi::CStr::from_ptr(*s as *const c_char)
                        .to_str()
                        .map_err(|e| e.to_string())?
                        .to_string()
                }
                .parse::<IpCidr>()
                .map_err(|e| e.to_string())?,
            ),
            CValue::IpAddr(s) => Self::IpAddr(
                unsafe {
                    ffi::CStr::from_ptr(*s as *const c_char)
                        .to_str()
                        .map_err(|e| e.to_string())?
                        .to_string()
                }
                .parse::<IpAddr>()
                .map_err(|e| e.to_string())?,
            ),
            CValue::Int(i) => Self::Int(*i),
        })
    }
}

/// Write displayable error message to the error buffer.
///
/// # Arguments
///
/// - `err`: the displayable error message.
/// - `errbuf`: a buffer to store the error message.
/// - `errbuf_len`: a pointer to the length of the error message buffer.
///
/// # Safety
///
/// Violating any of the following constraints will result in undefined behavior:
///
/// * `errbuf` must be valid to read and write for `*errbuf_len` bytes.
unsafe fn write_errbuf(err: impl Display, errbuf: *mut u8, errbuf_len: &mut usize) {
    use std::io::Write;

    let orig_len = *errbuf_len;
    let mut errbuf = from_raw_parts_mut(errbuf, orig_len);
    // Ignore truncation error
    let _ = write!(errbuf, "{}", err);
    let remaining_len = errbuf.len();
    *errbuf_len = orig_len - remaining_len;
}
