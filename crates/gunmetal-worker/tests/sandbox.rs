//! Integration tests of the sandbox launcher and a confined worker.
//!
//! This file is its own executable (`harness = false`) so the launcher can
//! start it again as the worker. No extra binary ships, and the hostile
//! hooks exist only in this test executable.
//!
//! The tests and their kernel machinery are Linux's ([`linux`]): the
//! confinement under test exists there, and a kernel that lacks part of it
//! is emulated per call, so the reduced tier is proven against a kernel
//! that really refuses rather than skipped. On any other platform the
//! executable asserts the one behaviour there is: the worker refuses to
//! confine itself, and so never serves ([`elsewhere`]) (SEC-MED-024).

fn main() {
    #[cfg(target_os = "linux")]
    linux::run();
    #[cfg(not(target_os = "linux"))]
    elsewhere::run();
}

#[cfg(target_os = "linux")]
#[path = "sandbox/linux.rs"]
mod linux;

#[cfg(not(target_os = "linux"))]
mod elsewhere {
    //! Where the worker cannot confine itself, it refuses to serve.

    use gunmetal_worker::sandbox::{Profile, confine};

    /// Verifies: SEC-MED-024
    pub fn run() {
        let error = confine(Profile::Scan).unwrap_err();
        assert_eq!(
            error.to_string(),
            "the kernel refused a confinement step (Threads, None)"
        );
        println!("the worker refuses to serve where it cannot confine itself");
    }
}
