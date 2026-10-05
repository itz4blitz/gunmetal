//! Marks this process's extra descriptors close-on-exec: the one `unsafe`
//! in Gunmetal's own crates (ADR 13).
//!
//! A server can inherit descriptors without close-on-exec from whatever
//! started it (systemd socket activation, a wrapper's lock file), and a
//! process started by `Command` would inherit them too. Setting the flag on
//! a descriptor this process knows only by number needs a borrowed
//! descriptor, and making one from a number is `unsafe`
//! (`BorrowedFd::borrow_raw`). No maintained crate offers it safely
//! (ADR 13), so this module is the only door for it: one `unsafe` block,
//! allowed by an `#[expect]` on that statement alone and by one row of
//! `xtask lint-exceptions`. Any other `unsafe` in this crate fails to
//! build, and any other crate forbids it outright.

use rustix::io::{FdFlags, fcntl_setfd};
use std::os::fd::{BorrowedFd, RawFd};

/// Sets close-on-exec on each descriptor in `open`, the numbers listed
/// from `/proc/self/fd`, that is numbered `first` or higher.
///
/// A number that no longer names an open descriptor is skipped: the call
/// fails with `EBADF` and nothing else happens. Confinement still refuses
/// a worker that ends up holding an extra descriptor, so this is never the
/// only check.
pub(crate) fn mark_from(first: u32, open: &[u32]) {
    open.iter()
        .filter(|&&number| number >= first)
        .filter_map(|&number| RawFd::try_from(number).ok())
        .for_each(|number| {
            #[expect(
                unsafe_code,
                reason = "the one unsafe block in Gunmetal's crates: borrow a listed descriptor number to set close-on-exec on it (ADR 13, SEC-MED-022)"
            )]
            // SAFETY: `borrow_raw` asks that the number name an open
            // descriptor for as long as the borrow lives. The number comes
            // from this process's own `/proc/self/fd` listing, and the
            // borrow lives for one `fcntl(F_SETFD)` call. If another thread
            // closed it since the listing, the call fails with `EBADF`; if
            // the number was reused, close-on-exec is set on the new
            // descriptor, which is the flag this function exists to set on
            // every descriptor from `first` up. `F_SETFD` reads and writes
            // no memory, and the borrow is never stored or closed.
            let descriptor = unsafe { BorrowedFd::borrow_raw(number) };
            let _ = fcntl_setfd(descriptor, FdFlags::CLOEXEC);
        });
}

/// Serialises the unit tests that mark descriptors, or launch (which
/// marks every descriptor of the test process), so one cannot mark a
/// descriptor another test is checking.
#[cfg(test)]
pub(crate) static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
mod tests {
    use super::{SERIAL, mark_from};
    use rustix::io::{FdFlags, fcntl_getfd, fcntl_setfd};
    use std::os::fd::AsRawFd;
    use std::os::unix::net::UnixStream;

    /// The number of `stream`'s descriptor.
    fn number(stream: &UnixStream) -> u32 {
        u32::try_from(stream.as_raw_fd()).unwrap()
    }

    /// Two descriptors without close-on-exec, the second numbered higher.
    fn two_without_close_on_exec() -> (UnixStream, UnixStream) {
        let (low, high) = UnixStream::pair().unwrap();
        fcntl_setfd(&low, FdFlags::empty()).unwrap();
        fcntl_setfd(&high, FdFlags::empty()).unwrap();
        assert!(number(&low) < number(&high));
        (low, high)
    }

    #[test]
    fn every_listed_descriptor_from_the_first_number_up_is_marked() {
        let _serial = SERIAL.lock().unwrap();
        let (low, high) = two_without_close_on_exec();
        mark_from(number(&low), &[number(&low), number(&high)]);
        assert_eq!(fcntl_getfd(&low).unwrap(), FdFlags::CLOEXEC);
        assert_eq!(fcntl_getfd(&high).unwrap(), FdFlags::CLOEXEC);
    }

    #[test]
    fn a_descriptor_below_the_first_number_is_left_alone() {
        let _serial = SERIAL.lock().unwrap();
        let (low, high) = two_without_close_on_exec();
        mark_from(number(&high), &[number(&low), number(&high)]);
        assert_eq!(fcntl_getfd(&low).unwrap(), FdFlags::empty());
        assert_eq!(fcntl_getfd(&high).unwrap(), FdFlags::CLOEXEC);
    }

    #[test]
    fn a_descriptor_that_is_not_listed_is_left_alone() {
        let _serial = SERIAL.lock().unwrap();
        let (low, high) = two_without_close_on_exec();
        mark_from(0, &[number(&high)]);
        assert_eq!(fcntl_getfd(&low).unwrap(), FdFlags::empty());
        assert_eq!(fcntl_getfd(&high).unwrap(), FdFlags::CLOEXEC);
    }

    /// A number that is not open (closed since it was listed, say) and one
    /// no descriptor can have are skipped; the rest are still marked.
    #[test]
    fn a_closed_or_impossible_number_is_skipped() {
        let _serial = SERIAL.lock().unwrap();
        let (low, high) = two_without_close_on_exec();
        let not_open = u32::try_from(i32::MAX).unwrap();
        mark_from(0, &[not_open, u32::MAX, number(&low), number(&high)]);
        assert_eq!(fcntl_getfd(&low).unwrap(), FdFlags::CLOEXEC);
        assert_eq!(fcntl_getfd(&high).unwrap(), FdFlags::CLOEXEC);
    }
}
