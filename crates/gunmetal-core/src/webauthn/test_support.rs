//! Test support for the `WebAuthn` readers: a small-stack runner and an
//! encoder for CBOR heads, written for the tests and never calling the
//! reader under test.

/// The stack size SEC-MED-001 names, in octets.
const STACK: usize = 262_144;

/// Runs `work` on a fresh thread with a 256 KiB stack and returns what it
/// returned, so a parse that recurses too deeply fails its test instead of
/// passing on the test runner's larger stack.
pub(super) fn on_small_stack<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(STACK)
        .spawn(work)
        .expect("the test thread starts")
        .join()
        .expect("the code under test returned instead of panicking")
}

/// How many octets `bytes` holds, as the `u64` the readers report.
pub(super) fn len(bytes: &[u8]) -> u64 {
    u64::try_from(bytes.len()).unwrap_or(u64::MAX)
}

/// Appends the shortest definite head RFC 8949 allows for major type
/// `major` with argument `n`.
pub(super) fn write_head(out: &mut Vec<u8>, major: u8, n: u64) {
    let lead = major.checked_mul(0x20).unwrap_or(0);
    match (u8::try_from(n), u16::try_from(n), u32::try_from(n)) {
        (Ok(small @ 0..=23), _, _) => out.push(lead | small),
        (Ok(byte), _, _) => out.extend_from_slice(&[lead | 0x18, byte]),
        (_, Ok(wide), _) => {
            out.push(lead | 0x19);
            out.extend_from_slice(&wide.to_be_bytes());
        }
        (_, _, Ok(wide)) => {
            out.push(lead | 0x1A);
            out.extend_from_slice(&wide.to_be_bytes());
        }
        _ => {
            out.push(lead | 0x1B);
            out.extend_from_slice(&n.to_be_bytes());
        }
    }
}

/// The head alone.
pub(super) fn head(major: u8, n: u64) -> Vec<u8> {
    let mut out = Vec::new();
    write_head(&mut out, major, n);
    out
}

/// `unit` written `count` times over.
pub(super) fn repeated(unit: &[u8], count: usize) -> Vec<u8> {
    let mut out = Vec::new();
    for _ in 0..count {
        out.extend_from_slice(unit);
    }
    out
}

/// An unsigned integer.
pub(super) fn unsigned(n: u64) -> Vec<u8> {
    head(0, n)
}

/// A negative integer whose value is `-1 - n`.
pub(super) fn negative_arg(n: u64) -> Vec<u8> {
    head(1, n)
}

/// A byte string.
pub(super) fn bytes(value: &[u8]) -> Vec<u8> {
    let mut out = head(2, len(value));
    out.extend_from_slice(value);
    out
}

/// A text string.
pub(super) fn text(value: &str) -> Vec<u8> {
    let mut out = head(3, len(value.as_bytes()));
    out.extend_from_slice(value.as_bytes());
    out
}

/// A map of already encoded keys and values, in the order given.
pub(super) fn map(pairs: &[(Vec<u8>, Vec<u8>)]) -> Vec<u8> {
    let mut out = head(5, u64::try_from(pairs.len()).unwrap_or(u64::MAX));
    for (key, value) in pairs {
        out.extend_from_slice(key);
        out.extend_from_slice(value);
    }
    out
}

#[test]
fn writes_the_shortest_head_at_every_width() {
    assert_eq!(head(0, 0), [0x00]);
    assert_eq!(head(0, 23), [0x17]);
    assert_eq!(head(1, 24), [0x38, 0x18]);
    assert_eq!(head(2, 255), [0x58, 0xFF]);
    assert_eq!(head(3, 256), [0x79, 0x01, 0x00]);
    assert_eq!(head(4, 65_535), [0x99, 0xFF, 0xFF]);
    assert_eq!(head(5, 65_536), [0xBA, 0x00, 0x01, 0x00, 0x00]);
    assert_eq!(head(0, u64::from(u32::MAX)), [0x1A, 0xFF, 0xFF, 0xFF, 0xFF]);
    assert_eq!(
        head(1, 4_294_967_296),
        [0x3B, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00]
    );
    assert_eq!(text("ab"), [0x62, b'a', b'b']);
    assert_eq!(bytes(&[7]), [0x41, 7]);
    assert_eq!(map(&[(unsigned(1), negative_arg(0))]), [0xA1, 0x01, 0x20]);
    assert_eq!(len(&[1, 2, 3]), 3);
    assert_eq!(repeated(&[1, 2], 3), [1, 2, 1, 2, 1, 2]);
    assert_eq!(repeated(&[1], 0), []);
    assert_eq!(on_small_stack(|| 5), 5);
}
