use crate::ast::Type;
use crate::schema::Schema;
use std::ffi;
use std::os::raw::c_char;

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
