//! The confinement a worker applies to itself before it reads a job
//! (SEC-MED-021, SEC-MED-022).
//!
//! The order is fixed: check that the process has one thread; set the
//! resource limits and clear the dumpable flag; account for every open
//! descriptor; set `no_new_privs`; enforce Landlock; install the seccomp
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
    /// Descriptors other than the socket were open, and there is no
    /// seccomp filter to take them out of the worker's reach.
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
                write!(f, "descriptors {strays:?} are open and cannot be refused")
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
    kernel.no_new_privs().map_err(refused(Step::NoNewPrivs))?;
    let landlock = kernel.landlock();
    let seccomp = kernel.seccomp(&strays);
    if !seccomp && !strays.is_empty() {
        return Err(ConfineError::StrayDescriptors(strays));
    }
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

/// Applies Landlock and seccomp to this process, without the floor.
///
/// The sandbox test executable calls this in a child so the two kernel
/// methods run where a coverage file can still be written.
#[doc(hidden)]
#[must_use]
pub fn apply_landlock_and_seccomp() -> (bool, bool) {
    let mut linux = Linux;
    (linux.landlock(), linux.seccomp(&[]))
}

#[cfg(test)]
mod tests {
    use super::{ConfineError, LAST_SOCKET_DESCRIPTOR, Step, confine, confine_with};
    use crate::sandbox::kernel::Kernel;
    use crate::sandbox::limits::{Limit, Profile};
    use crate::sandbox::tier::Enforced;
    use std::io;

    /// A kernel that plays a recorded script. Each method returns the next
    /// prepared result and records what confinement asked for.
    struct Script {
        threads: Result<usize, i32>,
        limits: Vec<Result<(), i32>>,
        undumpable: Result<(), i32>,
        descriptors: Result<Vec<u32>, i32>,
        no_new_privs: Result<(), i32>,
        landlock: bool,
        seccomp: bool,
        seen_limits: Vec<Limit>,
        seen_strays: Option<Vec<u32>>,
    }

    impl Script {
        fn ok() -> Self {
            Self {
                threads: Ok(1),
                limits: Profile::Scan.limits().into_iter().map(|_| Ok(())).collect(),
                undumpable: Ok(()),
                descriptors: Ok(vec![0, 1, 2]),
                no_new_privs: Ok(()),
                landlock: true,
                seccomp: true,
                seen_limits: Vec::new(),
                seen_strays: None,
            }
        }

        fn into_error(result: Result<(), i32>) -> io::Result<()> {
            result.map_err(io::Error::from_raw_os_error)
        }
    }

    impl Kernel for Script {
        fn threads(&mut self) -> io::Result<usize> {
            self.threads.map_err(io::Error::from_raw_os_error)
        }

        fn limit(&mut self, limit: Limit) -> io::Result<()> {
            self.seen_limits.push(limit);
            match self.limits.split_first() {
                Some((result, rest)) => {
                    let result = *result;
                    self.limits = rest.to_vec();
                    Self::into_error(result)
                }
                None => Ok(()),
            }
        }

        fn undumpable(&mut self) -> io::Result<()> {
            Self::into_error(self.undumpable)
        }

        fn descriptors(&mut self) -> io::Result<Vec<u32>> {
            self.descriptors
                .clone()
                .map_err(io::Error::from_raw_os_error)
        }

        fn no_new_privs(&mut self) -> io::Result<()> {
            Self::into_error(self.no_new_privs)
        }

        fn landlock(&mut self) -> bool {
            self.landlock
        }

        fn seccomp(&mut self, strays: &[u32]) -> bool {
            self.seen_strays = Some(strays.to_vec());
            self.seccomp
        }
    }

    fn refused(step: Step, errno: i32) -> ConfineError {
        ConfineError::Refused {
            step,
            errno: Some(errno),
        }
    }

    #[test]
    fn the_floor_and_every_optional_control_are_reported_when_they_hold() {
        let mut kernel = Script::ok();
        let enforced = confine_with(&mut kernel, Profile::Scan).unwrap();
        assert_eq!(kernel.seen_limits, Profile::Scan.limits());
        assert_eq!(kernel.seen_strays.as_deref(), Some([].as_slice()));
        assert_eq!(
            enforced,
            Enforced {
                process: true,
                limits: true,
                no_new_privs: true,
                seccomp: true,
                landlock: true,
                namespaces: false,
            }
        );
    }

    #[test]
    fn stray_descriptors_are_handed_to_seccomp_when_it_holds() {
        let mut kernel = Script::ok();
        kernel.descriptors = Ok(vec![0, 1, 2, 7, 9]);
        assert!(confine_with(&mut kernel, Profile::Scan).unwrap().seccomp);
        assert_eq!(kernel.seen_strays.as_deref(), Some([7, 9].as_slice()));
    }

    #[test]
    fn stray_descriptors_without_seccomp_refuse_confinement() {
        let mut kernel = Script::ok();
        kernel.descriptors = Ok(vec![9, 0, 7, 1]);
        kernel.seccomp = false;
        assert_eq!(
            confine_with(&mut kernel, Profile::Scan).unwrap_err(),
            ConfineError::StrayDescriptors(vec![9, 7])
        );
    }

    #[test]
    fn a_second_thread_stops_before_any_limit_is_set() {
        let mut kernel = Script::ok();
        kernel.threads = Ok(3);
        assert_eq!(
            confine_with(&mut kernel, Profile::Scan).unwrap_err(),
            ConfineError::NotSingleThreaded { threads: 3 }
        );
        assert_eq!(kernel.seen_limits, []);
    }

    #[test]
    fn each_floor_step_names_itself_when_the_kernel_refuses() {
        type Prepare = fn(&mut Script);
        let cases: [(Prepare, ConfineError); 5] = [
            (|kernel| kernel.threads = Err(5), refused(Step::Threads, 5)),
            (|kernel| kernel.limits[2] = Err(1), refused(Step::Limits, 1)),
            (
                |kernel| kernel.undumpable = Err(1),
                refused(Step::Dumpable, 1),
            ),
            (
                |kernel| kernel.descriptors = Err(2),
                refused(Step::Descriptors, 2),
            ),
            (
                |kernel| kernel.no_new_privs = Err(13),
                refused(Step::NoNewPrivs, 13),
            ),
        ];
        for (prepare, expected) in cases {
            let mut kernel = Script::ok();
            prepare(&mut kernel);
            assert_eq!(
                confine_with(&mut kernel, Profile::Scan).unwrap_err(),
                expected
            );
        }
    }

    #[test]
    fn landlock_or_seccomp_missing_is_still_the_floor() {
        let mut kernel = Script::ok();
        kernel.landlock = false;
        kernel.seccomp = false;
        assert_eq!(
            confine_with(&mut kernel, Profile::Scan).unwrap(),
            Enforced {
                process: true,
                limits: true,
                no_new_privs: true,
                seccomp: false,
                landlock: false,
                namespaces: false,
            }
        );
    }

    #[test]
    fn a_script_with_no_prepared_limits_accepts_further_calls() {
        let mut kernel = Script::ok();
        kernel.limits.clear();
        kernel.limit(Profile::Scan.limits()[0]).unwrap();
        assert_eq!(kernel.seen_limits.len(), 1);
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
            "descriptors [7, 9] are open and cannot be refused"
        );
        assert_eq!(LAST_SOCKET_DESCRIPTOR, 2);
        let error: &dyn std::error::Error = &refused(Step::NoNewPrivs, 1);
        assert_eq!(error.to_string(), refused(Step::NoNewPrivs, 1).to_string());
    }

    /// The cargo test process has other threads. Confinement must refuse
    /// it before changing anything.
    ///
    /// Verifies: SEC-MED-021
    #[test]
    fn confine_refuses_a_multithreaded_process() {
        let (keep, parked) = std::sync::mpsc::channel::<()>();
        let helper = std::thread::spawn(move || parked.recv());
        let error = confine(Profile::Scan).unwrap_err();
        assert!(
            matches!(error, ConfineError::NotSingleThreaded { threads } if threads > 1),
            "{error:?}"
        );
        drop(keep);
        assert_eq!(helper.join().unwrap(), Err(std::sync::mpsc::RecvError));
    }
}
