use crate::context::Context;
use crate::ffi::write_errbuf;
use crate::router::Router;
use crate::schema::Schema;
use std::ffi;
use std::os::raw::c_char;
use std::slice::from_raw_parts_mut;
use uuid::Uuid;

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
///
/// [`schema_new`]: crate::ffi::schema::schema_new
#[no_mangle]
pub unsafe extern "C" fn router_new(schema: &Schema) -> *mut Router<&Schema> {
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
pub unsafe extern "C" fn router_free(router: *mut Router<&Schema>) {
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
///   and must not have '\0' in the middle.
/// - `atc` must be a valid pointer to a C-style string, must be properly aligned,
///   and must not have '\0' in the middle.
/// - `errbuf` must be valid to read and write for `errbuf_len * size_of::<u8>()` bytes,
///   and it must be properly aligned.
/// - `errbuf_len` must be valid to read and write for `size_of::<usize>()` bytes,
///   and it must be properly aligned.
#[no_mangle]
pub unsafe extern "C" fn router_add_matcher(
    router: &mut Router<&Schema>,
    priority: usize,
    uuid: *const i8,
    atc: *const i8,
    errbuf: *mut u8,
    errbuf_len: *mut usize,
) -> bool {
    let uuid = ffi::CStr::from_ptr(uuid as *const c_char).to_str().unwrap();
    let atc = ffi::CStr::from_ptr(atc as *const c_char).to_str().unwrap();

    let uuid = Uuid::try_parse(uuid).expect("invalid UUID format");

    if let Err(e) = router.add_matcher(priority, uuid, atc) {
        write_errbuf(e, errbuf, errbuf_len);
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
///   and must not have '\0' in the middle.
#[no_mangle]
pub unsafe extern "C" fn router_remove_matcher(
    router: &mut Router<&Schema>,
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
///   and must be reset by [`context_reset`] before calling this function
///   if you want to reuse the same context for multiple matches.
///
/// [`context_new`]: crate::ffi::context::context_new
/// [`context_reset`]: crate::ffi::context::context_reset
#[no_mangle]
pub unsafe extern "C" fn router_execute(router: &Router<&Schema>, context: &mut Context) -> bool {
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
///   (NOT C-style strings) that are actually used in the router, which will be filled in.
///   if `fields` is `NULL`, this function will only return the number of fields used
///   in the router.
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
    router: &Router<&Schema>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ffi::ERR_BUF_MAX_LEN;

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
            assert!(!result);
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
            assert!(!result);
            assert!(errbuf_len < ERR_BUF_MAX_LEN);
        }
    }
}
