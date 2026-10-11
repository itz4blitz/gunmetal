//! Integration tests of the worker protocol with a real worker process.
//!
//! This file is its own executable (`harness = false`), as the sandbox's
//! tests are, so that the launcher can start it again as the worker. No
//! extra binary ships.
//!
//! The protocol tests run against a confined worker, and the confinement
//! is Linux's ([`linux`]). On any other platform the executable asserts
//! the one behaviour there is: the worker refuses to confine itself, so
//! nothing serves ([`elsewhere`]) (SEC-MED-024).

fn main() {
    #[cfg(target_os = "linux")]
    linux::run();
    #[cfg(not(target_os = "linux"))]
    elsewhere::run();
}

#[cfg(target_os = "linux")]
#[path = "ipc/linux.rs"]
mod linux;

#[cfg(not(target_os = "linux"))]
mod elsewhere {
    //! Where the worker cannot confine itself, nothing serves.

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
