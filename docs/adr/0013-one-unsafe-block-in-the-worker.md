# 13. One `unsafe` block in the worker, to mark descriptors close-on-exec

Date: 2026-10-04
Status: accepted, through the owner's answer on WP-045 on 2026-10-04

This record narrows point 9 of record 6 ("`unsafe` is forbidden in every
crate") by one block. It does not change that point for any other crate.

## Context

The sandbox launcher (WP-045) starts a worker with `std::process::Command`
and passes it one end of a socket pair as descriptors 0, 1 and 2. The
worker must hold nothing else: confinement refuses to go on if it finds an
extra descriptor (SEC-MED-022).

`Command` passes on every descriptor of the server that lacks the
close-on-exec flag. The server does not choose all of them. A program that
starts it can pass descriptors without the flag: systemd socket activation
does, and so does the build machine's own slot wrapper, which hands its
lock file to every command it runs. Without marking them, every worker of
such a server is refused, and media scanning is off.

Setting the flag, or closing a descriptor, that the process knows only by
number needs a borrowed or owned descriptor, and making one from a number
is `unsafe` in Rust (`BorrowedFd::borrow_raw`, `OwnedFd::from_raw_fd`).
The first version used the `close_fds` crate, which does this in its own
`unsafe` code. The owner refused it on 2026-10-04: one publisher, and no
release since June 2021, is not acceptable at the sandbox boundary.

What we found when looking for a maintained safe route (2026-10-04):

- `rustix` 1.1.5, already in the graph: `fcntl_setfd` is safe but takes
  `AsFd`; `close` and `try_close` on a raw number are `unsafe`; it has no
  `close_range`. `pidfd_getfd` only makes a duplicate, and close-on-exec
  belongs to each descriptor, so marking the duplicate leaves the
  original as it was.
- `nix` 0.30 and 0.31: `fcntl` takes `AsFd` and `close` takes
  `IntoRawFd`. Only older versions took a raw number safely, which is why
  they were changed.
- `command-fds` 0.3.3: maps chosen descriptors into the child; it does not
  close or mark the others.
- `fork` 0.10.0: its own `Command` closes every descriptor above 2 in the
  child. It would replace `std::process::Command` in the launcher, has
  one publisher and ten breaking releases since July 2025, and is itself
  `unsafe` over `libc`; that moves the single-publisher question instead
  of removing it.

## Decision

`gunmetal-worker` carries exactly one `unsafe` block, in
`crates/gunmetal-worker/src/sandbox/descriptors.rs`. Before it starts a
worker, the launcher lists `/proc/self/fd` (the listing's own descriptor
is skipped) and, for each number from 3 up, borrows it with
`BorrowedFd::borrow_raw` for one `rustix::io::fcntl_setfd(.., CLOEXEC)`
call. No new crate is added.

Why the borrow is sound:

- `borrow_raw` asks that the number name an open descriptor for as long
  as the borrow lives. The number comes from this process's own listing,
  and the borrow lives for one `fcntl(F_SETFD)` call; it is never stored,
  closed or read through.
- The launcher runs in the server, which has other threads, so a number
  can be closed between the listing and the call. Then the call fails
  with `EBADF` and nothing happens. If the number was reused in between,
  close-on-exec is set on the new descriptor, which is what the function
  does to every descriptor from 3 up anyway. `F_SETFD` reads and writes no
  memory. Neither case can cause undefined behaviour or pass a descriptor
  to the worker that it would not otherwise have had.
- This is never the only check. The worker still lists its descriptors
  after it starts, and refuses confinement (`StrayDescriptors`) before
  `no_new_privs`, Landlock and seccomp if any extra is open, so a failure
  here fails closed.

How the gate keeps it to one block:

- The workspace still sets `unsafe_code = "forbid"`, and no attribute can
  lift a `forbid`. `gunmetal-worker` alone repeats the workspace's lint
  tables with that one line changed to `"deny"`, and an `#[expect]` on the
  one statement allows it. A second `unsafe` block anywhere in the crate
  fails to build.
- `xtask lint-exceptions` fails if any other crate sets its own lints (the
  core keeps its stricter tables, record 6), if the worker's tables differ
  from the workspace's in anything but that line, and if the lint is named
  outside `descriptors.rs` or on more than one line there.
- `supply-chain/native-allowlist.toml` lists `gunmetal-worker` with this
  reason, so `xtask native-code` fails if `unsafe` appears in the crate
  without it, and fails if the entry outlives the block.

`gunmetal-core` stays free of `unsafe`: its own lint table forbids it, and
its workspace-rules tests check that.

## Consequences

- A server started with stray descriptors still starts workers that hold
  only their socket.
- The allow-list entry (`supply-chain/`) and this record (`docs/adr/`)
  are code-owner paths, so changing either needs the owner's approval
  (SEC-SUP-005). The module and the lint-exception row are not; a change
  to them that adds a second block still fails the gate as above.
- If `rustix` or the standard library gains a safe way to mark or close
  descriptors by number, or a `Command` option that passes only the
  descriptors it is given, the block goes, and with it the worker's own
  lint tables, the exception row and the allow-list entry.
