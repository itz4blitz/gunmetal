//! The sandbox self-test (SEC-MED-024): start a worker, let it confine
//! itself, and report the tier it reached.
//!
//! The worker answers with one octet. The server treats it as it treats
//! everything a worker sends, as untrusted input: any octet
//! that is not a well-formed answer, no answer in time, and a worker that
//! could not be started all count as nothing enforced, which the tier
//! table turns into "off".

use super::args::{Job, TypedArgs};
use super::confine::ConfineError;
use super::confine::confine;
use super::launch::{Inherited, launch};
use super::limits::Profile;
use super::programs::Program;
use super::tier::{Enforced, TierReport};
use std::io::{self, Read, Write};
use std::os::unix::net::UnixStream;
use std::time::Duration;

/// The high bits of every answer. Printable text never has them all set,
/// so a message a broken worker prints is not mistaken for an answer.
const MARK: u8 = 0b1110_0000;
/// The answer's bit for the resource limits.
const LIMITS: u8 = 0b0_0001;
/// The answer's bit for `no_new_privs`.
const NO_NEW_PRIVS: u8 = 0b0_0010;
/// The answer's bit for seccomp.
const SECCOMP: u8 = 0b0_0100;
/// The answer's bit for Landlock.
const LANDLOCK: u8 = 0b0_1000;
/// The answer's bit for namespaces.
const NAMESPACES: u8 = 0b1_0000;

/// How long the server waits for a worker's answer.
const WAIT: Duration = Duration::from_secs(30);

/// The answer for the outcome of confinement. A worker that could not
/// confine itself answers that nothing is enforced.
fn encode(outcome: &Result<Enforced, ConfineError>) -> u8 {
    let enforced = outcome
        .as_ref()
        .map_or(Enforced::NONE, |&enforced| enforced);
    [
        (enforced.limits, LIMITS),
        (enforced.no_new_privs, NO_NEW_PRIVS),
        (enforced.seccomp, SECCOMP),
        (enforced.landlock, LANDLOCK),
        (enforced.namespaces, NAMESPACES),
    ]
    .into_iter()
    .filter(|(holds, _)| *holds)
    .map(|(_, bit)| bit)
    .fold(MARK, u8::saturating_add)
}

/// Reads an answer back. The process is separate by the fact that it
/// answered over the launcher's socket.
fn decode(answer: u8) -> Option<Enforced> {
    (answer & MARK == MARK).then_some(Enforced {
        process: true,
        limits: answer & LIMITS != 0,
        no_new_privs: answer & NO_NEW_PRIVS != 0,
        seccomp: answer & SECCOMP != 0,
        landlock: answer & LANDLOCK != 0,
        namespaces: answer & NAMESPACES != 0,
    })
}

/// Writes the answer for `outcome` to the worker's socket.
fn answer(outcome: &Result<Enforced, ConfineError>, channel: &mut impl Write) -> io::Result<()> {
    channel
        .write_all(&[encode(outcome)])
        .and_then(|()| channel.flush())
}

/// The worker's side of the self-test: confines the process under
/// `profile` and writes the answer to `channel`, the worker's socket.
///
/// The executable calls this when [`TypedArgs::from_argv`] reports
/// [`Job::SelfTest`], then waits to be stopped.
///
/// # Errors
///
/// The operating system's error when the answer cannot be written.
pub fn answer_self_test(profile: Profile, channel: &mut impl Write) -> io::Result<()> {
    answer(&confine(profile), channel)
}

/// Waits for one octet from the worker.
fn read_answer(channel: &mut UnixStream) -> Option<u8> {
    let mut octet = [0_u8; 1];
    channel
        .set_read_timeout(Some(WAIT))
        .and_then(|()| channel.read_exact(&mut octet))
        .ok()
        .map(|()| octet[0])
}

/// Starts a self-test worker and returns what it says it enforced.
fn ask(profile: Profile) -> Option<Enforced> {
    Inherited::pair()
        .ok()
        .and_then(|(fds, ours)| {
            launch(Program::Worker, TypedArgs::new(Job::SelfTest, profile), fds)
                .ok()
                .map(|child| (child, ours))
        })
        .and_then(|(child, mut ours)| {
            let answer = read_answer(&mut ours);
            drop(child);
            answer
        })
        .and_then(decode)
}

/// Self-tests the sandbox for `profile`: starts a worker that confines
/// itself, and applies the tier table to what it reports.
///
/// The server runs this at startup and on demand, and shows the report on
/// the health page and in `doctor`. It never fails: a worker that cannot
/// be started, does not answer or answers nonsense is reported as the
/// "off" tier.
#[must_use]
pub fn self_test(profile: Profile) -> TierReport {
    ask(profile).unwrap_or(Enforced::NONE).report()
}

#[cfg(test)]
mod tests {
    use super::{answer, answer_self_test, decode, encode, self_test};
    use crate::sandbox::confine::{ConfineError, Step};
    use crate::sandbox::limits::Profile;
    use crate::sandbox::tier::{Enforced, Tier, TierReport};

    /// What a worker on a current kernel enforces.
    const TYPICAL: Enforced = Enforced {
        process: true,
        limits: true,
        no_new_privs: true,
        seccomp: true,
        landlock: true,
        namespaces: false,
    };

    #[test]
    fn an_answer_is_the_mark_and_one_bit_per_control() {
        assert_eq!(encode(&Ok(TYPICAL)), 0b1110_1111);
        assert_eq!(
            encode(&Ok(Enforced {
                namespaces: true,
                ..TYPICAL
            })),
            0b1111_1111
        );
        assert_eq!(
            encode(&Ok(Enforced {
                seccomp: false,
                ..TYPICAL
            })),
            0b1110_1011
        );
        assert_eq!(
            encode(&Ok(Enforced {
                landlock: false,
                ..TYPICAL
            })),
            0b1110_0111
        );
        assert_eq!(
            encode(&Ok(Enforced {
                limits: false,
                ..TYPICAL
            })),
            0b1110_1110
        );
        assert_eq!(
            encode(&Ok(Enforced {
                no_new_privs: false,
                ..TYPICAL
            })),
            0b1110_1101
        );
    }

    #[test]
    fn a_worker_that_could_not_confine_itself_answers_nothing_enforced() {
        assert_eq!(
            encode(&Err(ConfineError::Refused {
                step: Step::Limits,
                errno: Some(1)
            })),
            0b1110_0000
        );
    }

    #[test]
    fn an_answer_reads_back_as_a_separate_process_with_its_controls() {
        assert_eq!(decode(0b1110_1111), Some(TYPICAL));
        assert_eq!(
            decode(0b1111_1111),
            Some(Enforced {
                namespaces: true,
                ..TYPICAL
            })
        );
        assert_eq!(
            decode(0b1110_0000),
            Some(Enforced {
                process: true,
                ..Enforced::NONE
            })
        );
        assert_eq!(
            decode(0b1110_0001),
            Some(Enforced {
                process: true,
                limits: true,
                ..Enforced::NONE
            })
        );
        assert_eq!(
            decode(0b1110_0010),
            Some(Enforced {
                process: true,
                no_new_privs: true,
                ..Enforced::NONE
            })
        );
        assert_eq!(
            decode(0b1110_0100),
            Some(Enforced {
                process: true,
                seccomp: true,
                ..Enforced::NONE
            })
        );
        assert_eq!(
            decode(0b1110_1000),
            Some(Enforced {
                process: true,
                landlock: true,
                ..Enforced::NONE
            })
        );
    }

    #[test]
    fn an_octet_without_the_whole_mark_is_not_an_answer() {
        for octet in [0x00, 0x1f, b'e', b'\n', 0x7f, 0x80, 0xc0, 0xdf, 0x6f, 0xaf] {
            assert_eq!(decode(octet), None);
        }
    }

    #[test]
    fn the_answer_is_written_as_one_octet() {
        let mut channel = Vec::new();
        answer(&Ok(TYPICAL), &mut channel).unwrap();
        assert_eq!(channel, [0b1110_1111]);
    }

    /// The test process has more than one thread, so confinement refuses
    /// it before changing anything, and the answer says so.
    #[test]
    fn a_process_that_cannot_be_confined_answers_nothing_enforced() {
        let (keep, parked) = std::sync::mpsc::channel::<()>();
        let helper = std::thread::spawn(move || parked.recv());
        let mut channel = Vec::new();
        answer_self_test(Profile::Scan, &mut channel).unwrap();
        assert_eq!(channel, [0b1110_0000]);
        drop(keep);
        assert_eq!(helper.join().unwrap(), Err(std::sync::mpsc::RecvError));
    }

    /// The unit-test executable is not a worker: started with the worker
    /// arguments it prints an error and exits. The self-test must read
    /// that as nothing enforced, never as a pass.
    ///
    /// Verifies: SEC-MED-024
    #[test]
    fn a_worker_that_does_not_answer_is_reported_as_off() {
        assert_eq!(
            self_test(Profile::Scan),
            TierReport {
                tier: Tier::Off,
                notice: Some(
                    "Media scanning is off: the worker could not start with a separate process, \
                     resource limits and the no-new-privileges flag. \
                     Gunmetal does not read media files without that."
                        .to_owned()
                ),
            }
        );
    }
}
