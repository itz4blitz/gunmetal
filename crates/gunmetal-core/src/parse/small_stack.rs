//! Test support: runs code under test on a thread with the 256 KiB stack
//! that SEC-MED-001's property tests name, so a parse that recurses too
//! deeply fails its test instead of passing on the test runner's larger
//! stack.

/// The stack size SEC-MED-001 names, in octets.
const STACK: usize = 262_144;

/// Runs `work` on a fresh thread with a 256 KiB stack and returns what it
/// returned.
///
/// # Panics
///
/// Panics when the thread cannot start or when `work` panics; the panic
/// message of `work` is printed by its own thread first.
pub fn on_small_stack<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(STACK)
        .spawn(work)
        .expect("the test thread starts")
        .join()
        .expect("the code under test returned instead of panicking")
}
