use crate::ast::{Type, Value};
use crate::context::Context;
use crate::router::Router;
use crate::schema::Schema;
use cidr::IpCidr;
use std::cmp::min;
use std::convert::TryFrom;
use std::ffi;
use std::net::IpAddr;
use std::os::raw::c_char;
use std::slice::{from_raw_parts, from_raw_parts_mut};
use uuid::fmt::Hyphenated;
use uuid::Uuid;

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

#[no_mangle]
pub extern "C" fn schema_new() -> *mut Schema {
    Box::into_raw(Box::default())
}

#[no_mangle]
pub unsafe extern "C" fn schema_free(schema: *mut Schema) {
    drop(Box::from_raw(schema));
}

#[no_mangle]
pub unsafe extern "C" fn schema_add_field(schema: &mut Schema, field: *const i8, typ: Type) {
    let field = ffi::CStr::from_ptr(field as *const c_char)
        .to_str()
        .unwrap();

    schema.add_field(field, typ)
}

#[no_mangle]
pub unsafe extern "C" fn router_new(schema: &Schema) -> *mut Router {
    Box::into_raw(Box::new(Router::new(schema)))
}

#[no_mangle]
pub unsafe extern "C" fn router_free(router: *mut Router) {
    drop(Box::from_raw(router));
}

#[no_mangle]
// uuid must be ASCII representation of 128-bit UUID
pub unsafe extern "C" fn router_add_matcher(
    router: &mut Router,
    priority: usize,
    uuid: *const i8,
    atc: *const i8,
    errbuf: *mut u8,
    errbuf_len: *mut usize,
) -> bool {
    let uuid = ffi::CStr::from_ptr(uuid as *const c_char).to_str().unwrap();
    let atc = ffi::CStr::from_ptr(atc as *const c_char).to_str().unwrap();
    let errbuf = from_raw_parts_mut(errbuf, ERR_BUF_MAX_LEN);

    let uuid = Uuid::try_parse(uuid).expect("invalid UUID format");

    if let Err(e) = router.add_matcher(priority, uuid, atc) {
        let errlen = min(e.len(), *errbuf_len);
        errbuf[..errlen].copy_from_slice(&e.as_bytes()[..errlen]);
        *errbuf_len = errlen;
        return false;
    }

    true
}

#[no_mangle]
// uuid must be ASCII representation of 128-bit UUID
pub unsafe extern "C" fn router_remove_matcher(
    router: &mut Router,
    priority: usize,
    uuid: *const i8,
) -> bool {
    let uuid = ffi::CStr::from_ptr(uuid as *const c_char).to_str().unwrap();
    let uuid = Uuid::try_parse(uuid).expect("invalid UUID format");

    router.remove_matcher(priority, uuid)
}

#[no_mangle]
pub unsafe extern "C" fn router_execute(router: &Router, context: &mut Context) -> bool {
    router.execute(context)
}

#[no_mangle]
pub unsafe extern "C" fn router_get_fields(
    router: &Router,
    fields: *mut *const u8,
    fields_len: *mut usize,
) -> usize {
    if !fields.is_null() {
        assert!(!fields_len.is_null());
        assert!(*fields_len >= router.fields.len());

        let fields = from_raw_parts_mut(fields, *fields_len);
        let fields_len = from_raw_parts_mut(fields_len, *fields_len);

        for (i, k) in router.fields.keys().enumerate() {
            fields[i] = k.as_bytes().as_ptr();
            fields_len[i] = k.len()
        }
    }

    router.fields.len()
}

#[no_mangle]
pub unsafe extern "C" fn context_new(schema: &Schema) -> *mut Context {
    Box::into_raw(Box::new(Context::new(schema)))
}

#[no_mangle]
pub unsafe extern "C" fn context_free(context: *mut Context) {
    drop(Box::from_raw(context));
}

#[no_mangle]
pub unsafe extern "C" fn context_add_value(
    context: &mut Context,
    field: *const i8,
    value: &CValue,
    errbuf: *mut u8,
    errbuf_len: *mut usize,
) -> bool {
    let field = ffi::CStr::from_ptr(field as *const c_char)
        .to_str()
        .unwrap();
    let errbuf = from_raw_parts_mut(errbuf, ERR_BUF_MAX_LEN);

    let value: Result<Value, _> = value.try_into();
    if let Err(e) = value {
        let errlen = min(e.len(), *errbuf_len);
        errbuf[..errlen].copy_from_slice(&e.as_bytes()[..errlen]);
        *errbuf_len = errlen;
        return false;
    }

    context.add_value(field, value.unwrap());

    true
}

#[no_mangle]
pub unsafe extern "C" fn context_reset(context: &mut Context) {
    context.reset();
}

#[no_mangle]
pub unsafe extern "C" fn context_get_result(
    context: &Context,
    uuid_hex: *mut u8,
    matched_field: *const i8,
    matched_value: *mut *const u8,
    matched_value_len: *mut usize,
    capture_names: *mut *const u8,
    capture_names_len: *mut usize,
    capture_values: *mut *const u8,
    capture_values_len: *mut usize,
) -> isize {
    if context.result.is_none() {
        return -1;
    }

    if !uuid_hex.is_null() {
        let uuid_hex = from_raw_parts_mut(uuid_hex, Hyphenated::LENGTH);
        let res = context.result.as_ref().unwrap();

        res.uuid.as_hyphenated().encode_lower(uuid_hex);

        if !matched_field.is_null() {
            let matched_field = ffi::CStr::from_ptr(matched_field as *const c_char)
                .to_str()
                .unwrap();
            assert!(!matched_value.is_null());
            assert!(!matched_value_len.is_null());
            if let Some(Value::String(v)) = res.matches.get(matched_field) {
                *matched_value = v.as_bytes().as_ptr();
                *matched_value_len = v.len();
            } else {
                *matched_value_len = 0;
            }
        }

        if !context.result.as_ref().unwrap().captures.is_empty() {
            assert!(*capture_names_len >= res.captures.len());
            assert!(*capture_names_len == *capture_values_len);
            assert!(!capture_names.is_null());
            assert!(!capture_names_len.is_null());
            assert!(!capture_values.is_null());
            assert!(!capture_values_len.is_null());

            let capture_names = from_raw_parts_mut(capture_names, *capture_names_len);
            let capture_names_len = from_raw_parts_mut(capture_names_len, *capture_names_len);
            let capture_values = from_raw_parts_mut(capture_values, *capture_values_len);
            let capture_values_len = from_raw_parts_mut(capture_values_len, *capture_values_len);

            for (i, (k, v)) in res.captures.iter().enumerate() {
                capture_names[i] = k.as_bytes().as_ptr();
                capture_names_len[i] = k.len();

                capture_values[i] = v.as_bytes().as_ptr();
                capture_values_len[i] = v.len();
            }
        }
    }

    context
        .result
        .as_ref()
        .unwrap()
        .captures
        .len()
        .try_into()
        .unwrap()
}

/// # Safety
/// This function dereferences raw pointers. The caller must ensure that the pointers
/// are valid and point to writable memory of the correct type.
#[no_mangle]
pub unsafe extern "C" fn debug_router_get_duration(
    router: &mut Router,
    // Durations are in nanoseconds
    add_matcher_duration: *mut u64,
    remove_matcher_duration: *mut u64,
    execute_duration: *mut u64,
) {
    // Get
    *add_matcher_duration = router.add_matcher_duration.as_nanos() as u64;
    *remove_matcher_duration = router.remove_matcher_duration.as_nanos() as u64;
    *execute_duration = (*router.execute_duration.get()).as_nanos() as u64;
    // Reset
    router.add_matcher_duration = Default::default();
    router.remove_matcher_duration = Default::default();
    *router.execute_duration.get_mut() = Default::default();
}

/// # Safety
/// This function dereferences raw pointers. The caller must ensure that the pointers
/// are valid and point to writable memory of the correct type.
#[no_mangle]
pub unsafe extern "C" fn debug_router_get_counter(
    router: &mut Router,
    add_matcher_counter: *mut u64,
    remove_matcher_counter: *mut u64,
    execute_counter: *mut u64,
) {
    // Get
    *add_matcher_counter = router.add_matcher_counter as u64;
    *remove_matcher_counter = router.remove_matcher_counter as u64;
    *execute_counter = *router.execute_counter.get() as u64;
    // Reset
    router.add_matcher_counter = 0;
    router.remove_matcher_counter = 0;
    *router.execute_counter.get_mut() = 0;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_long_error_message() {
        unsafe {
            let schema = Schema::default();
            let mut router = Router::new(&schema);
            let uuid = ffi::CString::new("a921a9aa-ec0e-4cf3-a6cc-1aa5583d150c").unwrap();
            let junk = ffi::CString::new(vec![b'a'; ERR_BUF_MAX_LEN * 2]).unwrap();
            let mut errbuf = vec![b'X'; ERR_BUF_MAX_LEN];
            let mut errbuf_len = ERR_BUF_MAX_LEN;

            let result = router_add_matcher(
                &mut router,
                1,
                uuid.as_ptr() as *const i8,
                junk.as_ptr() as *const i8,
                errbuf.as_mut_ptr(),
                &mut errbuf_len,
            );
            assert_eq!(result, false);
            assert_eq!(errbuf_len, ERR_BUF_MAX_LEN);
        }
    }

    #[test]
    fn test_short_error_message() {
        unsafe {
            let schema = Schema::default();
            let mut router = Router::new(&schema);
            let uuid = ffi::CString::new("a921a9aa-ec0e-4cf3-a6cc-1aa5583d150c").unwrap();
            let junk = ffi::CString::new("aaaa").unwrap();
            let mut errbuf = vec![b'X'; ERR_BUF_MAX_LEN];
            let mut errbuf_len = ERR_BUF_MAX_LEN;

            let result = router_add_matcher(
                &mut router,
                1,
                uuid.as_ptr() as *const i8,
                junk.as_ptr() as *const i8,
                errbuf.as_mut_ptr(),
                &mut errbuf_len,
            );
            assert_eq!(result, false);
            assert!(errbuf_len < ERR_BUF_MAX_LEN);
        }
    }
}
