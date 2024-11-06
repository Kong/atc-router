pub mod context;
pub mod expression;
pub mod router;
pub mod schema;

use crate::ast::Value;
use cidr::IpCidr;
use std::cmp::min;
use std::convert::TryFrom;
use std::ffi;
use std::fmt::Display;
use std::net::IpAddr;
use std::os::raw::c_char;
use std::slice::from_raw_parts;
use std::slice::from_raw_parts_mut;

pub const ERR_BUF_MAX_LEN: usize = 4096;

#[derive(Debug)]
#[repr(C)]
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
/// * `errbuf` must be valid to read and write for `errbuf_len * size_of::<u8>()` bytes,
///   and it must be properly aligned.
/// * `errbuf_len` must be vlaid to read and write for `size_of::<usize>()` bytes,
///   and it must be properly aligned.
unsafe fn write_errbuf(err: impl Display, errbuf: *mut u8, errbuf_len: *mut usize) {
    let errbuf = from_raw_parts_mut(errbuf, ERR_BUF_MAX_LEN);
    // Replace internal '\0' to space.
    let err = err.to_string().replace('\0', " ");
    // Unwrap is safe since we already remove all internal '\0's.
    let err_cstring = std::ffi::CString::new(err.to_string()).unwrap();
    let err_bytes = err_cstring.as_bytes_with_nul();
    let errlen = min(err_bytes.len(), *errbuf_len);
    errbuf[..errlen].copy_from_slice(&err_bytes[..errlen]);
    *errbuf_len = errlen;
}
