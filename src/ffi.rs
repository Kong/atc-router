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

/// Deallocate the schema object.
///
/// # Errors
///
/// This function never fails.
///
/// # Safety
///
/// Violating any of the following constraints will result in undefined behavior:
///
/// - `schema` must be a valid pointer returned by [`schema_new`].
#[no_mangle]
pub unsafe extern "C" fn schema_free(schema: *mut Schema) {
    drop(Box::from_raw(schema));
}

/// Add a new field with the specified type to the schema.
///
/// # Arguments
///
/// - `schema`: a valid pointer to the [`Schema`] object returned by [`schema_new`].
/// - `field`: the C-style string representing the field name.
/// - `typ`: the type of the field.
///
/// # Panics
///
/// This function will panic if the C-style string
/// pointed by `field` is not a valid UTF-8 string.
///
/// # Safety
///
/// Violating any of the following constraints will result in undefined behavior:
///
/// - `schema` must be a valid pointer returned by [`schema_new`].
/// - `field` must be a valid pointer to a C-style string, must be properly aligned,
///   and must not have '\0' in the middle.
#[no_mangle]
pub unsafe extern "C" fn schema_add_field(schema: &mut Schema, field: *const i8, typ: Type) {
    let field = ffi::CStr::from_ptr(field as *const c_char)
        .to_str()
        .unwrap();

    schema.add_field(field, typ)
}

/// Create a new router object associated with the schema.
///
/// # Arguments
///
/// - `schema`: a valid pointer to the [`Schema`] object returned by [`schema_new`].
///
/// # Errors
///
/// This function never fails.
///
/// # Safety
///
/// Violating any of the following constraints will result in undefined behavior:
///
/// - `schema` must be a valid pointer returned by [`schema_new`].
#[no_mangle]
pub unsafe extern "C" fn router_new(schema: &Schema) -> *mut Router {
    Box::into_raw(Box::new(Router::new(schema)))
}

/// Deallocate the router object.
///
/// # Errors
///
/// This function never fails.
///
/// # Safety
///
/// Violating any of the following constraints will result in undefined behavior:
///
/// - `router` must be a valid pointer returned by [`router_new`].
#[no_mangle]
pub unsafe extern "C" fn router_free(router: *mut Router) {
    drop(Box::from_raw(router));
}

/// Add a new matcher to the router.
///
/// # Arguments
///
/// - `router`: a pointer to the [`Router`] object returned by [`router_new`].
/// - `priority`: the priority of the matcher, higher value means higher priority,
///   and the matcher with the highest priority will be executed first.
/// - `uuid`: the C-style string representing the UUID of the matcher.
/// - `atc`: the C-style string representing the ATC expression.
/// - `errbuf`: a buffer to store the error message.
/// - `errbuf_len`: a pointer to the length of the error message buffer.
///
/// # Returns
///
/// Returns `true` if the matcher was added successfully, otherwise `false`,
/// and the error message will be stored in the `errbuf`,
/// and the length of the error message will be stored in `errbuf_len`.
///
/// # Errors
///
/// This function will return `false` if the matcher could not be added to the router,
/// such as duplicate UUID, and invalid ATC expression.
///
/// # Panics
///
/// This function will panic when:
///
/// - `uuid` doesn't point to a ASCII sequence representing a valid 128-bit UUID.
/// - `atc` doesn't point to a valid C-style string.
///
/// # Safety
///
/// Violating any of the following constraints will result in undefined behavior:
///
/// - `router` must be a valid pointer returned by [`router_new`].
/// - `uuid` must be a valid pointer to a C-style string, must be properly aligned,
///    and must not have '\0' in the middle.
/// - `atc` must be a valid pointer to a C-style string, must be properly aligned,
///    and must not have '\0' in the middle.
/// - `errbuf` must be valid to read and write for `errbuf_len * size_of::<u8>()` bytes,
///    and it must be properly aligned.
/// - `errbuf_len` must be valid to read and write for `size_of::<usize>()` bytes,
///    and it must be properly aligned.
#[no_mangle]
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

/// Remove a matcher from the router.
///
/// # Arguments
/// - `router`: a pointer to the [`Router`] object returned by [`router_new`].
/// - `priority`: the priority of the matcher to be removed.
/// - `uuid`: the C-style string representing the UUID of the matcher to be removed.
///
/// # Returns
///
/// Returns `true` if the matcher was removed successfully, otherwise `false`,
/// such as when the matcher with the specified UUID doesn't exist or
/// the priority doesn't match the UUID.
///
/// # Panics
///
/// This function will panic when `uuid` doesn't point to a ASCII sequence
///
/// # Safety
///
/// Violating any of the following constraints will result in undefined behavior:
///
/// - `router` must be a valid pointer returned by [`router_new`].
/// - `uuid` must be a valid pointer to a C-style string, must be properly aligned,
///    and must not have '\0' in the middle.
#[no_mangle]
pub unsafe extern "C" fn router_remove_matcher(
    router: &mut Router,
    priority: usize,
    uuid: *const i8,
) -> bool {
    let uuid = ffi::CStr::from_ptr(uuid as *const c_char).to_str().unwrap();
    let uuid = Uuid::try_parse(uuid).expect("invalid UUID format");

    router.remove_matcher(priority, uuid)
}

/// Execute the router with the context.
///
/// # Arguments
///
/// - `router`: a pointer to the [`Router`] object returned by [`router_new`].
/// - `context`: a pointer to the [`Context`] object.
///
/// # Returns
///
/// Returns `true` if found a match, `false` means no match found.
///
/// # Safety
///
/// Violating any of the following constraints will result in undefined behavior:
///
/// - `router` must be a valid pointer returned by [`router_new`].
/// - `context` must be a valid pointer returned by [`context_new`],
///    and must be reset by [`context_reset`] before calling this function
///    if you want to reuse the same context for multiple matches.
#[no_mangle]
pub unsafe extern "C" fn router_execute(router: &Router, context: &mut Context) -> bool {
    router.execute(context)
}

/// Get the de-duplicated fields that are actually used in the router.
/// This is useful when you want to know what fields are actually used in the router,
/// so you can generate their values on-demand.
///
/// # Arguments
///
/// - `router`: a pointer to the [`Router`] object returned by [`router_new`].
/// - `fields`: a pointer to an array of pointers to the field names
///    (NOT C-style strings) that are actually used in the router, which will be filled in.
///    if `fields` is `NULL`, this function will only return the number of fields used
///    in the router.
/// - `fields_len`: a pointer to an array of the length of each field name.
///
/// # Lifetimes
///
/// The string pointers stored in `fields` might be invalidated if any of the following
/// operations are happened:
///
/// - The `router` was deallocated.
/// - A new matcher was added to the `router`.
/// - A matcher was removed from the `router`.
///
/// # Returns
///
/// Returns the number of fields that are actually used in the router.
///
/// # Errors
///
/// This function never fails.
///
/// # Safety
///
/// Violating any of the following constraints will result in undefined behavior:
///
/// - `router` must be a valid pointer returned by [`router_new`].
/// - If `fields` is not `NULL`, `fields` must be valid to read and write for
///   `fields_len * size_of::<*const u8>()` bytes, and it must be properly aligned.
/// - If `fields` is not `NULL`, `fields_len` must be valid to read and write for
///   `size_of::<usize>()` bytes, and it must be properly aligned.
/// - DO NOT write the memory pointed by the elements of `fields`.
/// - DO NOT access the memory pointed by the elements of `fields`
///   after it becomes invalid, see the `Lifetimes` section.
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

/// Allocate a new context object associated with the schema.
///
/// # Errors
///
/// This function never fails.
///
/// # Safety
///
/// Violating any of the following constraints will result in undefined behavior:
///
/// - `schema` must be a valid pointer returned by [`schema_new`].
#[no_mangle]
pub unsafe extern "C" fn context_new(schema: &Schema) -> *mut Context {
    Box::into_raw(Box::new(Context::new(schema)))
}

/// Deallocate the context object.
///
/// # Errors
///
/// This function never fails.
///
/// # Safety
///
/// Violating any of the following constraints will result in undefined behavior:
///
/// - `context` must be a valid pointer returned by [`context_new`].
#[no_mangle]
pub unsafe extern "C" fn context_free(context: *mut Context) {
    drop(Box::from_raw(context));
}

/// Add a value associated with a field to the context.
/// This is useful when you want to match a value against a field in the schema.
///
/// # Arguments
///
/// - `context`: a pointer to the [`Context`] object.
/// - `field`: the C-style string representing the field name.
/// - `value`: the value to be added to the context.
/// - `errbuf`: a buffer to store the error message.
/// - `errbuf_len`: a pointer to the length of the error message buffer.
///
/// # Returns
///
/// Returns `true` if the value was added successfully, otherwise `false`,
/// and the error message will be stored in the `errbuf`,
/// and the length of the error message will be stored in `errbuf_len`.
///
/// # Errors
///
/// This function will return `false` if the value could not be added to the context,
/// such as when a String value is not a valid UTF-8 string.
///
/// # Panics
///
/// This function will panic if the provided value does not match the schema.
///
/// # Safety
///
/// Violating any of the following constraints will result in undefined behavior:
///
/// * `context` must be a valid pointer returned by [`context_new`].
/// * `field` must be a valid pointer to a C-style string,
///   must be properply aligned, and must not have '\0' in the middle.
/// * `value` must be a valid pointer to a [`CValue`].
/// * `errbuf` must be valid to read and write for `errbuf_len * size_of::<u8>()` bytes,
///   and it must be properly aligned.
/// * `errbuf_len` must be vlaid to read and write for `size_of::<usize>()` bytes,
///   and it must be properly aligned.
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

/// Reset the context so that it can be reused.
/// This is useful when you want to reuse the same context for multiple matches.
/// This will clear all the values that were added to the context,
/// but keep the memory allocated for the context.
///
/// # Errors
///
/// This function never fails.
///
/// # Safety
///
/// Violating any of the following constraints will result in undefined behavior:
///
/// - `context` must be a valid pointer returned by [`context_new`].
#[no_mangle]
pub unsafe extern "C" fn context_reset(context: &mut Context) {
    context.reset();
}

/// Get the result of the context.
///
/// # Arguments
///
/// - `context`: a pointer to the [`Context`] object.
/// - `uuid_hex`: If not `NULL`, the UUID of the matched matcher will be stored.
/// - `matched_field`: If not `NULL`, the field name (C-style string) of the matched value will be stored.
/// - `matched_value`: If the `matched_field` is not `NULL`, the value of the matched field will be stored.
/// - `matched_value_len`: If the `matched_field` is not `NULL`, the length of the value of the matched field will be stored.
/// - `capture_names`: A pointer to an array of pointers to the capture names, each element is a non-C-style string pointer.
/// - `capture_names_len`: A pointer to an array of the length of each capture name.
/// - `capture_values`: A pointer to an array of pointers to the capture values, each element is a non-C-style string pointer.
/// - `capture_values_len`: A pointer to an array of the length of each capture value.
///
/// # Returns
///
/// Returns the number of captures that are stored in the context.
///
/// # Lifetimes
///
/// The string pointers stored in `matched_value`, `capture_names`, and `capture_values`
/// might be invalidated if any of the following operations are happened:
///
/// - The `context` was deallocated.
/// - The `context` was reset by [`context_reset`].
///
/// # Panics
///
/// This function will panic if the `matched_field` is not a valid UTF-8 string.
///
/// # Safety
///
/// Violating any of the following constraints will result in undefined behavior:
///
/// - `context` must be a valid pointer returned by [`context_new`],
///    must be passed to [`router_execute`] before calling this function,
///    and must not be reset by [`context_reset`] before calling this function.
/// - If `uuid_hex` is not `NULL`, `uuid_hex` must be valid to read and write for
///   `16 * size_of::<u8>()` bytes, and it must be properly aligned.
/// - If `matched_field` is not `NULL`,
///   `matched_field` must be a vlaid pointer to a C-style string,
///   must be properly aligned, and must not have '\0' in the middle.
/// - If `matched_value` is not `NULL`,
///   `matched_value` must be valid to read and write for
///   `mem::size_of::<*const u8>()` bytes, and it must be properly aligned.
/// - If `matched_value` is not `NULL`, `matched_value_len` must be valid to read and write for
///   `size_of::<usize>()` bytes, and it must be properly aligned.
/// - If `uuid_hex` is not `NULL`, `capture_names` must be valid to read and write for
///   `<captures> * size_of::<*const u8>()` bytes, and it must be properly aligned.
/// - If `uuid_hex` is not `NULL`, `capture_names_len` must be valid to read and write for
///   `<captures> * size_of::<usize>()` bytes, and it must be properly aligned.
/// - If `uuid_hex` is not `NULL`, `capture_values` must be valid to read and write for
///   `<captures> * size_of::<*const u8>()` bytes, and it must be properly aligned.
/// - If `uuid_hex` is not `NULL`, `capture_values_len` must be valid to read and write for
///   `<captures> * size_of::<usize>()` bytes, and it must be properly aligned.
///
/// Note: You should get the `<captures>` by calling this function and set every pointer
/// except the `context` to `NULL` to get the number of captures.
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
