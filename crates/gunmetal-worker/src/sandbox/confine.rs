//! The confinement a worker applies to itself before it reads a job
//! (SEC-MED-021, SEC-MED-022).
//!
//! The order is fixed: check that the process has one thread; set the
//! resource limits and clear the dumpable flag; refuse to go on if any
//! descriptor but the socket is open; set `no_new_privs`; enforce
//! Landlock; install the seccomp
//! filter last, because every earlier step makes calls the filter
//! refuses. A step of the floor that fails stops the worker with a typed
//! error. Landlock or seccomp being unavailable does not: the worker
//! reports them missing and the tier table decides what may run
//! (SEC-MED-024). There is no argument that skips a step (SEC-TM-045).

use super::kernel::{Kernel, Linux};
use super::limits::Profile;
use super::tier::Enforced;
use std::fmt;
use std::io;

/// A step of the floor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Counting the process's threads.
    Threads,
    /// Setting the resource limits.
    Limits,
    /// Clearing the dumpable flag.
    Dumpable,
    /// Listing the open descriptors.
    Descriptors,
    /// Setting `no_new_privs`.
    NoNewPrivs,
}

/// Why a worker could not confine itself. It must not read a job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfineError {
    /// The process has more than one thread, and the per-thread controls
    /// would leave the others unconfined (SEC-MED-021).
    NotSingleThreaded {
        /// How many threads it has.
        threads: usize,
    },
    /// The kernel refused a step of the floor.
    Refused {
        /// The step.
        step: Step,
        /// The kernel's error number, when it gave one.
        errno: Option<i32>,
    },
    /// Descriptors other than the socket were open. The launcher passes
    /// none, so the worker was not started by it.
    StrayDescriptors(Vec<u32>),
}

impl fmt::Display for ConfineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotSingleThreaded { threads } => {
                write!(f, "the worker has {threads} threads and must have one")
            }
            Self::Refused { step, errno } => {
                write!(
                    f,
                    "the kernel refused a confinement step ({step:?}, {errno:?})"
                )
            }
            Self::StrayDescriptors(strays) => {
                write!(f, "descriptors {strays:?} are open besides the socket")
            }
        }
    }
}

impl std::error::Error for ConfineError {}

/// The highest descriptor a worker is meant to have: its socket is
/// descriptors 0, 1 and 2.
const LAST_SOCKET_DESCRIPTOR: u32 = 2;

/// Confines the process with the given kernel. See the module for the
/// order.
pub(crate) fn confine_with(
    kernel: &mut impl Kernel,
    profile: Profile,
) -> Result<Enforced, ConfineError> {
    let refused = |step: Step| {
        move |error: io::Error| ConfineError::Refused {
            step,
            errno: error.raw_os_error(),
        }
    };
    let threads = kernel.threads().map_err(refused(Step::Threads))?;
    if threads != 1 {
        return Err(ConfineError::NotSingleThreaded { threads });
    }
    for limit in profile.limits() {
        kernel.limit(limit).map_err(refused(Step::Limits))?;
    }
    kernel.undumpable().map_err(refused(Step::Dumpable))?;
    let strays: Vec<u32> = kernel
        .descriptors()
        .map_err(refused(Step::Descriptors))?
        .into_iter()
        .filter(|&descriptor| descriptor > LAST_SOCKET_DESCRIPTOR)
        .collect();
    // The worker does not close extras: that needs `unsafe`, and the
    // crate's one block is the launcher's (ADR 13), which marks extras
    // close-on-exec so a worker it starts has none. One that remains
    // stops the worker here.
    if !strays.is_empty() {
        return Err(ConfineError::StrayDescriptors(strays));
    }
    kernel.no_new_privs().map_err(refused(Step::NoNewPrivs))?;
    let landlock = kernel.landlock();
    let seccomp = kernel.seccomp();
    Ok(Enforced {
        process: true,
        limits: true,
        no_new_privs: true,
        seccomp,
        landlock,
        // Nothing here creates namespaces: unsharing them needs `unsafe`,
        // which the workspace forbids. The tier table therefore reports
        // them missing.
        namespaces: false,
    })
}

/// Confines the calling process as a worker and reports what was
/// enforced.
///
/// Call it once, first thing in the worker, before any job is read. It is
/// one-way: nothing undoes it, and the only input is the profile, which
/// chooses limits and cannot turn a control off (SEC-TM-045).
///
/// # Errors
///
/// A [`ConfineError`] when the floor cannot be reached. The worker must
/// then end without reading a job.
pub fn confine(profile: Profile) -> Result<Enforced, ConfineError> {
    confine_with(&mut Linux, profile)
}

#[cfg(test)]
mod tests {
    use super::{ConfineError, Step, confine, confine_with};
    use crate::sandbox::kernel::Kernel;
    use crate::sandbox::limits::{Limit, Profile};
    use crate::sandbox::tier::{Enforced, Tier, TierReport};
    use std::io;

    /// A kernel that plays a script: each method records its call and
    /// returns the prepared result.
    struct Script {
        threads: Result<usize, i32>,
        /// The error for the limit at this position, if it is to fail.
        failing_limit: Option<(usize, i32)>,
        undumpable: Result<(), i32>,
        descriptors: Result<Vec<u32>, i32>,
        no_new_privs: Result<(), i32>,
        landlock: bool,
        seccomp: bool,
        calls: Vec<&'static str>,
        limits: Vec<Limit>,
    }

    impl Script {
        fn ok() -> Self {
            Self {
                threads: Ok(1),
                failing_limit: None,
                undumpable: Ok(()),
                descriptors: Ok(vec![0, 1, 2]),
                no_new_privs: Ok(()),
                landlock: true,
                seccomp: true,
                calls: Vec::new(),
                limits: Vec::new(),
            }
        }

        fn result<T>(result: Result<T, i32>) -> io::Result<T> {
            result.map_err(io::Error::from_raw_os_error)
        }
    }

    impl Kernel for Script {
        fn threads(&mut self) -> io::Result<usize> {
            self.calls.push("threads");
            Self::result(self.threads)
        }

        fn limit(&mut self, limit: Limit) -> io::Result<()> {
            self.calls.push("limit");
            let position = self.limits.len();
            self.limits.push(limit);
            Self::result(
                self.failing_limit
                    .filter(|&(failing, _)| failing == position)
                    .map_or(Ok(()), |(_, errno)| Err(errno)),
            )
        }

        fn undumpable(&mut self) -> io::Result<()> {
            self.calls.push("undumpable");
            Self::result(self.undumpable)
        }

        fn descriptors(&mut self) -> io::Result<Vec<u32>> {
            self.calls.push("descriptors");
            Self::result(self.descriptors.clone())
        }

        fn no_new_privs(&mut self) -> io::Result<()> {
            self.calls.push("no_new_privs");
            Self::result(self.no_new_privs)
        }

        fn landlock(&mut self) -> bool {
            self.calls.push("landlock");
            self.landlock
        }

        fn seccomp(&mut self) -> bool {
            self.calls.push("seccomp");
            self.seccomp
        }
    }

    fn refused(step: Step, errno: i32) -> ConfineError {
        ConfineError::Refused {
            step,
            errno: Some(errno),
        }
    }

    /// Verifies: SEC-MED-022
    #[test]
    fn the_steps_run_in_order_and_everything_that_holds_is_reported() {
        let mut kernel = Script::ok();
        let enforced = confine_with(&mut kernel, Profile::Scan);
        assert_eq!(
            kernel.calls,
            [
                "threads",
                "limit",
                "limit",
                "limit",
                "limit",
                "limit",
                "limit",
                "undumpable",
                "descriptors",
                "no_new_privs",
                "landlock",
                "seccomp",
            ]
        );
        assert_eq!(kernel.limits, Profile::Scan.limits());
        assert_eq!(
            enforced,
            Ok(Enforced {
                process: true,
                limits: true,
                no_new_privs: true,
                seccomp: true,
                landlock: true,
                namespaces: false,
            })
        );
    }

    /// Verifies: SEC-MED-022
    #[test]
    fn a_descriptor_besides_the_socket_stops_the_worker_before_it_is_confined() {
        let mut kernel = Script::ok();
        kernel.descriptors = Ok(vec![9, 0, 7, 1, 2, 3]);
        assert_eq!(
            confine_with(&mut kernel, Profile::Scan),
            Err(ConfineError::StrayDescriptors(vec![9, 7, 3]))
        );
        assert_eq!(kernel.calls.last(), Some(&"descriptors"));
    }

    /// Verifies: SEC-MED-021
    #[test]
    fn a_second_thread_stops_before_any_limit_is_set() {
        let mut kernel = Script::ok();
        kernel.threads = Ok(2);
        assert_eq!(
            confine_with(&mut kernel, Profile::Scan),
            Err(ConfineError::NotSingleThreaded { threads: 2 })
        );
        assert_eq!(kernel.calls, ["threads"]);
    }

    #[test]
    fn a_process_with_no_thread_listed_is_not_single_threaded() {
        let mut kernel = Script::ok();
        kernel.threads = Ok(0);
        assert_eq!(
            confine_with(&mut kernel, Profile::Scan),
            Err(ConfineError::NotSingleThreaded { threads: 0 })
        );
    }

    #[test]
    fn each_floor_step_names_itself_when_the_kernel_refuses() {
        type Prepare = fn(&mut Script);
        let cases: [(Prepare, ConfineError, &str); 5] = [
            (
                |kernel| kernel.threads = Err(5),
                refused(Step::Threads, 5),
                "threads",
            ),
            (
                |kernel| kernel.failing_limit = Some((2, 1)),
                refused(Step::Limits, 1),
                "limit",
            ),
            (
                |kernel| kernel.undumpable = Err(1),
                refused(Step::Dumpable, 1),
                "undumpable",
            ),
            (
                |kernel| kernel.descriptors = Err(2),
                refused(Step::Descriptors, 2),
                "descriptors",
            ),
            (
                |kernel| kernel.no_new_privs = Err(13),
                refused(Step::NoNewPrivs, 13),
                "no_new_privs",
            ),
        ];
        for (prepare, expected, last) in cases {
            let mut kernel = Script::ok();
            prepare(&mut kernel);
            assert_eq!(confine_with(&mut kernel, Profile::Scan), Err(expected));
            assert_eq!(kernel.calls.last(), Some(&last));
        }
    }

    #[test]
    fn a_failing_limit_stops_at_that_limit() {
        let mut kernel = Script::ok();
        kernel.failing_limit = Some((2, 1));
        assert_eq!(
            confine_with(&mut kernel, Profile::Scan),
            Err(refused(Step::Limits, 1))
        );
        assert_eq!(kernel.calls, ["threads", "limit", "limit", "limit"]);
    }

    /// A kernel or an architecture without seccomp or Landlock, 32-bit ARM
    /// among them, leaves the worker at the floor, and the report names
    /// what is missing.
    ///
    /// Verifies: SEC-MED-024
    #[test]
    fn landlock_or_seccomp_missing_is_the_reduced_tier_and_is_named() {
        let floor = Enforced {
            process: true,
            limits: true,
            no_new_privs: true,
            seccomp: false,
            landlock: false,
            namespaces: false,
        };
        let cases = [
            (
                false,
                true,
                Enforced {
                    landlock: true,
                    ..floor
                },
                "Reduced isolation: media workers run without system call filtering (seccomp) \
                 and namespaces. \
                 They still run in a separate process with resource limits and no new privileges.",
            ),
            (
                true,
                false,
                Enforced {
                    seccomp: true,
                    ..floor
                },
                "Reduced isolation: media workers run without Landlock and namespaces. \
                 They still run in a separate process with resource limits and no new privileges.",
            ),
            (
                false,
                false,
                floor,
                "Reduced isolation: media workers run without system call filtering (seccomp), \
                 Landlock and namespaces. \
                 They still run in a separate process with resource limits and no new privileges.",
            ),
        ];
        for (seccomp, landlock, expected, notice) in cases {
            let mut kernel = Script::ok();
            kernel.seccomp = seccomp;
            kernel.landlock = landlock;
            let enforced = confine_with(&mut kernel, Profile::Scan).unwrap();
            assert_eq!(enforced, expected);
            assert_eq!(
                enforced.report(),
                TierReport {
                    tier: Tier::Reduced,
                    notice: Some(notice.to_owned()),
                }
            );
        }
    }

    #[test]
    fn confinement_errors_name_the_step() {
        assert_eq!(
            ConfineError::NotSingleThreaded { threads: 4 }.to_string(),
            "the worker has 4 threads and must have one"
        );
        assert_eq!(
            refused(Step::Limits, 1).to_string(),
            "the kernel refused a confinement step (Limits, Some(1))"
        );
        assert_eq!(
            ConfineError::Refused {
                step: Step::Dumpable,
                errno: None
            }
            .to_string(),
            "the kernel refused a confinement step (Dumpable, None)"
        );
        assert_eq!(
            ConfineError::StrayDescriptors(vec![7, 9]).to_string(),
            "descriptors [7, 9] are open besides the socket"
        );
        let error: &dyn std::error::Error = &refused(Step::NoNewPrivs, 1);
        assert_eq!(
            error.to_string(),
            "the kernel refused a confinement step (NoNewPrivs, Some(1))"
        );
    }

    /// The test process has a second thread for as long as this test
    /// holds one parked. Confinement must refuse it before changing
    /// anything.
    ///
    /// Verifies: SEC-MED-021
    #[test]
    fn confine_refuses_a_multithreaded_process() {
        let (keep, parked) = std::sync::mpsc::channel::<()>();
        let helper = std::thread::spawn(move || parked.recv());
        let error = confine(Profile::Scan).unwrap_err().to_string();
        let threads: Option<usize> = error
            .strip_prefix("the worker has ")
            .and_then(|rest| rest.strip_suffix(" threads and must have one"))
            .and_then(|count| count.parse().ok());
        assert!(threads.is_some_and(|threads| threads > 1));
        drop(keep);
        assert_eq!(helper.join().unwrap(), Err(std::sync::mpsc::RecvError));
    }
}
