use crate::router::Router;

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
