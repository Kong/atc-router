use crate::ast::{Type, Value};
use crate::context::Context;
use crate::router::Router;
use crate::schema::Schema;
use cidr::IpCidr;
use std::ffi::{self};
use std::net::IpAddr;
use std::os::raw::c_char;
use std::slice::from_raw_parts_mut;
use std::str::Utf8Error;
use uuid::fmt::Hyphenated;
use uuid::Uuid;

pub const ERR_BUF_MAX_LEN: usize = 2048;

#[derive(Debug)]
#[repr(C)]
#[allow(clippy::enum_variant_names)]
pub enum CValue {
    CString(*const i8),
    CIpCidr(*const i8),
    CIpAddr(*const i8),
    CInt(i64),
}

impl TryFrom<&CValue> for Value {
    type Error = String;

    fn try_from(v: &CValue) -> Result<Self, Self::Error> {
        Ok(match v {
            CValue::CString(s) => {
                let str: Result<&str, Utf8Error>;
                unsafe {
                    str = ffi::CStr::from_ptr(*s as *const c_char).to_str();
                }
                Self::String(match str {
                    Ok(val) => val.to_string(),
                    Err(err) => return Err(err.to_string()),
                })
            }
            CValue::CIpCidr(s) => {
                let str: Result<&str, Utf8Error>;
                unsafe {
                    str = ffi::CStr::from_ptr(*s as *const c_char).to_str();
                }

                Self::IpCidr(match str {
                    Ok(val) => val
                        .to_string()
                        .parse::<IpCidr>()
                        .map_err(|e| e.to_string())?,
                    Err(err) => return Err(err.to_string()),
                })
            }
            CValue::CIpAddr(s) => {
                let str: Result<&str, Utf8Error>;
                unsafe {
                    str = ffi::CStr::from_ptr(*s as *const c_char).to_str();
                }

                Self::IpAddr(match str {
                    Ok(val) => val
                        .to_string()
                        .parse::<IpAddr>()
                        .map_err(|e| e.to_string())?,
                    Err(err) => return Err(err.to_string()),
                })
            }
            CValue::CInt(i) => Self::Int(*i),
        })
    }
}

#[no_mangle]
pub extern "C" fn schema_new() -> *mut Schema {
    Box::into_raw(Box::new(Schema::default()))
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
    let uuid = match ffi::CStr::from_ptr(uuid as *const c_char).to_str() {
        Ok(str) => str,
        Err(_) => return false,
    };

    let atc = match ffi::CStr::from_ptr(atc as *const c_char).to_str() {
        Ok(str) => str,
        Err(_) => return false,
    };

    let errbuf = from_raw_parts_mut(errbuf, ERR_BUF_MAX_LEN);

    let uuid = Uuid::try_parse(uuid).expect("invalid UUID format");

    if let Err(e) = router.add_matcher(priority, uuid, atc) {
        errbuf[..e.len()].copy_from_slice(e.as_bytes());
        *errbuf_len = e.len();
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
    let uuid = match ffi::CStr::from_ptr(uuid as *const c_char).to_str() {
        Ok(str) => str,
        Err(_) => return false,
    };
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
    let field = match ffi::CStr::from_ptr(field as *const c_char).to_str() {
        Ok(str) => str,
        Err(_) => return false,
    };
    let errbuf = from_raw_parts_mut(errbuf, ERR_BUF_MAX_LEN);

    let value: Result<Value, _> = value.try_into();
    if let Err(e) = value {
        errbuf[..e.len()].copy_from_slice(e.as_bytes());
        *errbuf_len = e.len();
        return false;
    }

    context.add_value(field, value.unwrap());

    true
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

#[cfg(test)]
mod tests_ffi {
    use super::ffi;
    use super::{CValue, Value};

    #[test]
    fn test_try_from_works_for_cstring() {
        {
            let raw = b"qwerty".to_vec();
            let c_string: ffi::CString;
            unsafe {
                c_string = ffi::CString::from_vec_unchecked(raw);
            }
            let c_value = CValue::CString(c_string.as_ptr());
            let result = Value::try_from(&c_value).unwrap();
            _ = result;
        }
        {
            let raw = b"\xe2\x28\xa1\n\n".to_vec();
            let c_string: ffi::CString;
            unsafe {
                c_string = ffi::CString::from_vec_unchecked(raw);
            }
            let c_value = CValue::CString(c_string.as_ptr());
            let err = Value::try_from(&c_value).unwrap_err();
            let expected_err = String::from("invalid utf-8 sequence of 1 bytes from index 0");
            assert_eq!(err, expected_err);
        }
    }

    #[test]
    fn test_try_from_works_for_cipcidr() {
        {
            let raw = b"192.168.1.1/32".to_vec();
            let c_string: ffi::CString;
            unsafe {
                c_string = ffi::CString::from_vec_unchecked(raw);
            }
            let c_value = CValue::CIpCidr(c_string.as_ptr());
            let result = Value::try_from(&c_value).unwrap();
            _ = result;
        }
        {
            let raw = b"not_an_ip".to_vec();
            let c_string: ffi::CString;
            unsafe {
                c_string = ffi::CString::from_vec_unchecked(raw);
            }
            let c_value = CValue::CIpCidr(c_string.as_ptr());
            let err = Value::try_from(&c_value).unwrap_err();
            let expected_err =
                String::from("couldn't parse address in network: invalid IP address syntax");
            assert_eq!(err, expected_err);
        }
    }

    #[test]
    fn test_try_from_works_for_cipaddr() {
        {
            let raw = b"192.168.1.1".to_vec();
            let c_string: ffi::CString;
            unsafe {
                c_string = ffi::CString::from_vec_unchecked(raw);
            }
            let c_value = CValue::CIpCidr(c_string.as_ptr());
            let result = Value::try_from(&c_value).unwrap();
            _ = result;
        }
        {
            let raw = b"not_an_ip".to_vec();
            let c_string: ffi::CString;
            unsafe {
                c_string = ffi::CString::from_vec_unchecked(raw);
            }
            let c_value = CValue::CIpCidr(c_string.as_ptr());
            let err = Value::try_from(&c_value).unwrap_err();
            let expected_err =
                String::from("couldn't parse address in network: invalid IP address syntax");
            assert_eq!(err, expected_err);
        }
    }
}
