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

#[derive(Debug)]
#[repr(C)]
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
pub extern "C" fn schema_add_field(schema: &mut Schema, field: *const i8, typ: Type) {
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
    uuid: *const i8,
    atc: *const i8,
    errbuf: *mut u8,
    errbuf_len: *mut usize,
) -> bool {
    let uuid = unsafe { ffi::CStr::from_ptr(uuid).to_str().unwrap() };
    let atc = unsafe { ffi::CStr::from_ptr(atc).to_str().unwrap() };
    let errbuf = unsafe { from_raw_parts_mut(errbuf, 2048) };

    let uuid = Uuid::try_parse(uuid).expect("invalid UUID format");

    if let Err(e) = router.add_matcher(uuid, atc) {
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
pub extern "C" fn router_remove_matcher(router: &mut Router, uuid: *const i8) -> bool {
    let uuid = unsafe { ffi::CStr::from_ptr(uuid).to_str().unwrap() };
    let uuid = Uuid::try_parse(uuid).expect("invalid UUID format");

    router.remove_matcher(&uuid)
}

#[no_mangle]
pub extern "C" fn router_execute(router: &Router, context: &mut Context) -> bool {
    router.execute(context)
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
    field: *const i8,
    value: &CValue,
    errbuf: *mut u8,
    errbuf_len: *mut usize,
) -> bool {
    let field = unsafe { ffi::CStr::from_ptr(field).to_str().unwrap() };
    let errbuf = unsafe { from_raw_parts_mut(errbuf, 2048) };

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
pub extern "C" fn context_get_matched_count(context: &Context) -> usize {
    context.matches.len()
}

#[no_mangle]
pub extern "C" fn context_get_match(
    context: &Context,
    index: usize,
    uuid: *mut u8,
    prefix: *mut u8,
    prefix_len: *mut usize,
) {
    let uuid = unsafe { from_raw_parts_mut(uuid, Hyphenated::LENGTH) };
    let prefix = unsafe { from_raw_parts_mut(prefix, 2048) };

    let m = &context.matches[index];
    m.uuid.as_hyphenated().encode_lower(uuid);
    if let Some(p) = &m.prefix {
        prefix[..p.len()].copy_from_slice(p.as_bytes());
        unsafe {
            *prefix_len = p.len();
        }
    } else {
        unsafe {
            *prefix_len = 0;
        }
    }
}
