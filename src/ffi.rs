use crate::ast::{Type, Value};
use crate::context::Context;
use crate::router::Router;
use crate::schema::Schema;
use std::ffi;
use std::slice::from_raw_parts;
use std::slice::from_raw_parts_mut;
use uuid::Uuid;

#[derive(Debug)]
#[repr(C)]
pub enum CValue {
    CString(*const i8),
    //IpCidr(IpCidr),
    CInt(i64),
}

impl From<CValue> for Value {
    fn from(v: CValue) -> Self {
        match v {
            CValue::CString(s) => {
                Self::String(unsafe { ffi::CStr::from_ptr(s).to_str().unwrap().to_string() })
            }
            CValue::CInt(i) => Self::Int(i),
        }
    }
}

#[no_mangle]
pub extern "C" fn schema_new() -> *mut Schema {
    Box::into_raw(Box::new(Schema::new()))
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
        errbuf.copy_from_slice(e.as_bytes());
        unsafe {
            *errbuf_len = e.len();
        }
        return false;
    }

    true
}

#[no_mangle]
pub extern "C" fn router_execute(router: &Router, context: &Context) -> bool {
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
pub extern "C" fn context_add_value(context: &mut Context, field: *const i8, value: CValue) {
    let field = unsafe { ffi::CStr::from_ptr(field).to_str().unwrap() };
    println!("{:?}", value);

    context.add_value(field, value.into())
}
