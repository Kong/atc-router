use crate::ast::Value;
use crate::context::Context;
use crate::ffi::{write_errbuf, CValue};
use crate::schema::Schema;
use std::ffi;
use std::os::raw::c_char;
use std::slice::from_raw_parts_mut;
use uuid::fmt::Hyphenated;

/// Allocate a new context object associated with the schema.
///
/// # Errors
///
/// This function never returns an error, however, it can panic if memory allocation failed.
///
/// # Safety
///
/// Violating any of the following constraints will result in undefined behavior:
///
/// - `schema` must be a valid pointer returned by [`schema_new`].
///
/// [`schema_new`]: crate::ffi::schema::schema_new
#[no_mangle]
pub unsafe extern "C" fn context_new(schema: &Schema) -> *mut Context<'_> {
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
/// * `errbuf` must be valid to read and write for `*errbuf_len` bytes.
/// * `errbuf_len` must be valid to read and write for `size_of::<usize>()` bytes,
///   and it must be properly aligned.
#[no_mangle]
pub unsafe extern "C" fn context_add_value(
    context: &mut Context,
    field: *const i8,
    value: &CValue,
    errbuf: *mut u8,
    errbuf_len: &mut usize,
) -> bool {
    let field = ffi::CStr::from_ptr(field as *const c_char)
        .to_str()
        .unwrap();

    let value: Result<Value, _> = value.try_into();
    if let Err(e) = value {
        write_errbuf(e, errbuf, errbuf_len);
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
///   must be passed to [`router_execute`] before calling this function,
///   and must not be reset by [`context_reset`] before calling this function.
/// - If `uuid_hex` is not `NULL`, `uuid_hex` must be valid to read and write for
///   `36` bytes.
/// - If `matched_field` is not `NULL`,
///   `matched_field` must be a valid pointer to a C-style string,
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
///
/// [`router_execute`]: crate::ffi::router::router_execute
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
