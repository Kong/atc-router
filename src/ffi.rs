use crate::ast::{Type, Value};
use crate::context::Context;
use crate::router::Router;
use crate::schema::Schema;
use cidr::IpCidr;
use std::ffi;
use std::net::IpAddr;
use std::slice::from_raw_parts_mut;
use uuid::fmt::Hyphenated;
use uuid::Uuid;

pub const ERR_BUF_MAX_LEN: usize = 2048;

#[derive(Debug)]
#[repr(C)]
#[allow(clippy::enum_variant_names)]
pub enum CValue {
    CString(*const c_char),
    CIpCidr(*const c_char),
    CIpAddr(*const c_char),
    CInt(i64),
}

impl TryFrom<&CValue> for Value {
    type Error = String;

    fn try_from(v: &CValue) -> Result<Self, Self::Error> {
        Ok(match v {
            CValue::CString(s) => {
                Self::String(unsafe { ffi::CStr::from_ptr(*s).to_str().unwrap().to_string() })
            }
            CValue::CIpCidr(s) => Self::IpCidr(
                unsafe { ffi::CStr::from_ptr(*s).to_str().unwrap().to_string() }
                    .parse::<IpCidr>()
                    .map_err(|e| e.to_string())?,
            ),
            CValue::CIpAddr(s) => Self::IpAddr(
                unsafe { ffi::CStr::from_ptr(*s).to_str().unwrap().to_string() }
                    .parse::<IpAddr>()
                    .map_err(|e| e.to_string())?,
            ),
            CValue::CInt(i) => Self::Int(*i),
        })
    }
}

#[no_mangle]
pub extern "C" fn schema_new() -> *mut Schema {
    Box::into_raw(Box::new(Schema::default()))
}

#[no_mangle]
pub extern "C" fn schema_free(schema: *mut Schema) {
    unsafe { Box::from_raw(schema) };
}

#[no_mangle]
pub extern "C" fn schema_add_field(schema: &mut Schema, field: *const c_char, typ: Type) {
    let field = unsafe { ffi::CStr::from_ptr(field).to_str().unwrap() };

    schema.add_field(field, typ)
}

#[no_mangle]
pub extern "C" fn router_new(schema: &Schema) -> *mut Router {
    Box::into_raw(Box::new(Router::new(schema)))
}

#[no_mangle]
pub extern "C" fn router_free(router: *mut Router) {
    unsafe { Box::from_raw(router) };
}

#[no_mangle]
// uuid must be ASCII representation of 128-bit UUID
pub extern "C" fn router_add_matcher(
    router: &mut Router,
    priority: usize,
    uuid: *const c_char,
    atc: *const c_char,
    errbuf: *mut u8,
    errbuf_len: *mut usize,
) -> bool {
    let uuid = unsafe { ffi::CStr::from_ptr(uuid).to_str().unwrap() };
    let atc = unsafe { ffi::CStr::from_ptr(atc).to_str().unwrap() };
    let errbuf = unsafe { from_raw_parts_mut(errbuf, ERR_BUF_MAX_LEN) };

    let uuid = Uuid::try_parse(uuid).expect("invalid UUID format");

    if let Err(e) = router.add_matcher(priority, uuid, atc) {
        errbuf[..e.len()].copy_from_slice(e.as_bytes());
        unsafe {
            *errbuf_len = e.len();
        }
        return false;
    }

    true
}

#[no_mangle]
// uuid must be ASCII representation of 128-bit UUID
pub extern "C" fn router_remove_matcher(
    router: &mut Router,
    priority: usize,
    uuid: *const c_char,
) -> bool {
    let uuid = unsafe { ffi::CStr::from_ptr(uuid).to_str().unwrap() };
    let uuid = Uuid::try_parse(uuid).expect("invalid UUID format");

    router.remove_matcher(priority, uuid)
}

#[no_mangle]
pub extern "C" fn router_execute(router: &Router, context: &mut Context) -> bool {
    router.execute(context)
}

#[no_mangle]
pub extern "C" fn router_get_fields(
    router: &Router,
    fields: *mut *const u8,
    fields_len: *mut usize,
) -> usize {
    if !fields.is_null() {
        assert!(!fields_len.is_null());
        assert!(unsafe { *fields_len } >= router.fields.len());

        let fields = unsafe { from_raw_parts_mut(fields, *fields_len) };
        let fields_len = unsafe { from_raw_parts_mut(fields_len, *fields_len) };

        for (i, k) in router.fields.keys().enumerate() {
            fields[i] = k.as_bytes().as_ptr();
            fields_len[i] = k.len()
        }
    }

    router.fields.len()
}

#[no_mangle]
pub extern "C" fn context_new(schema: &Schema) -> *mut Context {
    Box::into_raw(Box::new(Context::new(schema)))
}

#[no_mangle]
pub extern "C" fn context_free(context: *mut Context) {
    unsafe { Box::from_raw(context) };
}

#[no_mangle]
pub extern "C" fn context_add_value(
    context: &mut Context,
    field: *const c_char,
    value: &CValue,
    errbuf: *mut u8,
    errbuf_len: *mut usize,
) -> bool {
    let field = unsafe { ffi::CStr::from_ptr(field).to_str().unwrap() };
    let errbuf = unsafe { from_raw_parts_mut(errbuf, ERR_BUF_MAX_LEN) };

    let value: Result<Value, _> = value.try_into();
    if let Err(e) = value {
        errbuf[..e.len()].copy_from_slice(e.as_bytes());
        unsafe {
            *errbuf_len = e.len();
        }
        return false;
    }

    context.add_value(field, value.unwrap());

    true
}

#[no_mangle]
pub extern "C" fn context_get_result(
    context: &Context,
    uuid_hex: *mut u8,
    matched_field: *const c_char,
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
        let uuid_hex = unsafe { from_raw_parts_mut(uuid_hex, Hyphenated::LENGTH) };
        let res = context.result.as_ref().unwrap();

        res.uuid.as_hyphenated().encode_lower(uuid_hex);

        if !matched_field.is_null() {
            let matched_field = unsafe { ffi::CStr::from_ptr(matched_field).to_str().unwrap() };
            assert!(!matched_value.is_null());
            assert!(!matched_value_len.is_null());
            if let Some(Value::String(v)) = res.matches.get(matched_field) {
                unsafe { *matched_value = v.as_bytes().as_ptr() };
                unsafe { *matched_value_len = v.len() };
            } else {
                unsafe { *matched_value_len = 0 };
            }
        }

        if !context.result.as_ref().unwrap().captures.is_empty() {
            assert!(unsafe { *capture_names_len } >= res.captures.len());
            assert!(unsafe { *capture_names_len == *capture_values_len });
            assert!(!capture_names.is_null());
            assert!(!capture_names_len.is_null());
            assert!(!capture_values.is_null());
            assert!(!capture_values_len.is_null());

            let capture_names = unsafe { from_raw_parts_mut(capture_names, *capture_names_len) };
            let capture_names_len =
                unsafe { from_raw_parts_mut(capture_names_len, *capture_names_len) };
            let capture_values = unsafe { from_raw_parts_mut(capture_values, *capture_values_len) };
            let capture_values_len =
                unsafe { from_raw_parts_mut(capture_values_len, *capture_values_len) };

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
