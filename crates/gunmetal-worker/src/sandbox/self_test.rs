//! The sandbox self-test (SEC-MED-024): start a worker, let it confine
//! itself, and report the tier it reached.
//!
//! The worker answers with two octets. The server treats them as it treats
//! everything a worker sends, as untrusted input: anything that is not a
//! well-formed answer, no answer in time, and a worker that could not be
//! started all count as nothing enforced, which the tier table turns into
//! "off".

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

/// The high bits of an answer's first octet. The second octet is the
/// first's complement, so it is below `0x20`. No text is like that: in
/// ASCII the first octet never has these bits, and in UTF-8 an octet that
/// has them is a lead octet (or none at all) that must be followed by a
/// continuation octet, `0x80` to `0xBF`. So a message a broken worker
/// prints, in any language, is not mistaken for an answer.
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

/// The answer for the outcome of confinement: the mark with one bit per
/// control, then its complement. A worker that could not confine itself
/// answers that nothing is enforced.
fn encode(outcome: &Result<Enforced, ConfineError>) -> [u8; 2] {
    let enforced = outcome
        .as_ref()
        .map_or(Enforced::NONE, |&enforced| enforced);
    let first = [
        (enforced.limits, LIMITS),
        (enforced.no_new_privs, NO_NEW_PRIVS),
        (enforced.seccomp, SECCOMP),
        (enforced.landlock, LANDLOCK),
        (enforced.namespaces, NAMESPACES),
    ]
    .into_iter()
    .filter(|(holds, _)| *holds)
    .map(|(_, bit)| bit)
    .fold(MARK, u8::saturating_add);
    [first, !first]
}

/// Reads an answer back. The process is separate by the fact that it
/// answered over the launcher's socket.
fn decode([first, second]: [u8; 2]) -> Option<Enforced> {
    (first & MARK == MARK && second == !first).then_some(Enforced {
        process: true,
        limits: first & LIMITS != 0,
        no_new_privs: first & NO_NEW_PRIVS != 0,
        seccomp: first & SECCOMP != 0,
        landlock: first & LANDLOCK != 0,
        namespaces: first & NAMESPACES != 0,
    })
}

/// Writes the answer for `outcome` to the worker's socket.
fn answer(outcome: &Result<Enforced, ConfineError>, channel: &mut impl Write) -> io::Result<()> {
    channel
        .write_all(&encode(outcome))
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

/// Waits for the two octets of an answer from the worker. Fewer before
/// the stream ends or the wait runs out is no answer.
fn read_answer(channel: &mut UnixStream) -> Option<[u8; 2]> {
    let mut octets = [0_u8; 2];
    channel
        .set_read_timeout(Some(WAIT))
        .and_then(|()| channel.read_exact(&mut octets))
        .ok()
        .map(|()| octets)
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
    use super::{answer, answer_self_test, decode, encode, read_answer, self_test};
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
    fn an_answer_is_the_mark_and_one_bit_per_control_then_its_complement() {
        assert_eq!(encode(&Ok(TYPICAL)), [0b1110_1111, 0b0001_0000]);
        assert_eq!(
            encode(&Ok(Enforced {
                namespaces: true,
                ..TYPICAL
            })),
            [0b1111_1111, 0b0000_0000]
        );
        assert_eq!(
            encode(&Ok(Enforced {
                seccomp: false,
                ..TYPICAL
            })),
            [0b1110_1011, 0b0001_0100]
        );
        assert_eq!(
            encode(&Ok(Enforced {
                landlock: false,
                ..TYPICAL
            })),
            [0b1110_0111, 0b0001_1000]
        );
        assert_eq!(
            encode(&Ok(Enforced {
                limits: false,
                ..TYPICAL
            })),
            [0b1110_1110, 0b0001_0001]
        );
        assert_eq!(
            encode(&Ok(Enforced {
                no_new_privs: false,
                ..TYPICAL
            })),
            [0b1110_1101, 0b0001_0010]
        );
    }

    #[test]
    fn a_worker_that_could_not_confine_itself_answers_nothing_enforced() {
        assert_eq!(
            encode(&Err(ConfineError::Refused {
                step: Step::Limits,
                errno: Some(1)
            })),
            [0b1110_0000, 0b0001_1111]
        );
    }

    #[test]
    fn an_answer_reads_back_as_a_separate_process_with_its_controls() {
        assert_eq!(decode([0b1110_1111, 0b0001_0000]), Some(TYPICAL));
        assert_eq!(
            decode([0b1111_1111, 0b0000_0000]),
            Some(Enforced {
                namespaces: true,
                ..TYPICAL
            })
        );
        assert_eq!(
            decode([0b1110_0000, 0b0001_1111]),
            Some(Enforced {
                process: true,
                ..Enforced::NONE
            })
        );
        assert_eq!(
            decode([0b1110_0001, 0b0001_1110]),
            Some(Enforced {
                process: true,
                limits: true,
                ..Enforced::NONE
            })
        );
        assert_eq!(
            decode([0b1110_0010, 0b0001_1101]),
            Some(Enforced {
                process: true,
                no_new_privs: true,
                ..Enforced::NONE
            })
        );
        assert_eq!(
            decode([0b1110_0100, 0b0001_1011]),
            Some(Enforced {
                process: true,
                seccomp: true,
                ..Enforced::NONE
            })
        );
        assert_eq!(
            decode([0b1110_1000, 0b0001_0111]),
            Some(Enforced {
                process: true,
                landlock: true,
                ..Enforced::NONE
            })
        );
    }

    /// Of all 65,536 pairs of octets, exactly the 32 whose first octet has
    /// the whole mark and whose second is its complement are answers.
    #[test]
    fn only_the_mark_and_its_complement_is_an_answer() {
        let mut answers = Vec::new();
        for first in 0..=u8::MAX {
            for second in 0..=u8::MAX {
                if decode([first, second]).is_some() {
                    answers.push([first, second]);
                }
            }
        }
        let expected: Vec<[u8; 2]> = (0xe0..=0xff_u8).map(|first| [first, !first]).collect();
        assert_eq!(answers, expected);
    }

    /// Text a broken worker prints is never an answer, in any language:
    /// a first octet with the whole mark is a UTF-8 lead octet or no
    /// UTF-8 at all, and the complement that follows is never a
    /// continuation octet.
    #[test]
    fn the_start_of_any_text_is_not_an_answer() {
        for text in [
            "error: no such file",
            "\u{feff}byte-order mark",
            "\u{ff25}\u{ff32}\u{ff32}",
            "\u{3042}\u{3044}",
            "\u{7fa9}",
            "\u{d55c}",
            "\u{1f600}",
            "\u{e9}",
        ] {
            let octets = text.as_bytes();
            assert_eq!(decode([octets[0], octets[1]]), None, "{text}");
        }
    }

    #[test]
    fn the_answer_is_written_as_two_octets() {
        let mut channel = Vec::new();
        answer(&Ok(TYPICAL), &mut channel).unwrap();
        assert_eq!(channel, [0b1110_1111, 0b0001_0000]);
    }

    /// One octet and then the end of the stream is not an answer, even
    /// when that octet is a whole one's first half.
    #[test]
    fn a_single_octet_is_not_an_answer() {
        use std::io::Write;
        use std::os::unix::net::UnixStream;

        let (mut ours, mut theirs) = UnixStream::pair().unwrap();
        theirs.write_all(&[0b1110_1111]).unwrap();
        drop(theirs);
        assert_eq!(read_answer(&mut ours), None);

        let (mut ours, mut theirs) = UnixStream::pair().unwrap();
        theirs.write_all(&[0b1110_1111, 0b0001_0000]).unwrap();
        assert_eq!(read_answer(&mut ours), Some([0b1110_1111, 0b0001_0000]));
    }

    /// The test process has more than one thread, so confinement refuses
    /// it before changing anything, and the answer says so.
    #[test]
    fn a_process_that_cannot_be_confined_answers_nothing_enforced() {
        let (keep, parked) = std::sync::mpsc::channel::<()>();
        let helper = std::thread::spawn(move || parked.recv());
        let mut channel = Vec::new();
        answer_self_test(Profile::Scan, &mut channel).unwrap();
        assert_eq!(channel, [0b1110_0000, 0b0001_1111]);
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
        let _serial = crate::sandbox::descriptors::SERIAL.lock().unwrap();
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
