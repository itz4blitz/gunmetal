//! A component package: one file, never an archive.
//!
//! [`open`] reads one WebAssembly component and the manifest embedded in its
//! `gunmetal-manifest` custom section. An archive is refused at its magic
//! and is not read further (SEC-EXT-034). This module does not link a
//! WebAssembly runtime.

use gunmetal_core::crypto::sha256;

/// An archive a package must not be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveKind {
    /// A Zip local header, end of central directory, or spanned marker.
    Zip,
    /// A gzip member.
    Gzip,
    /// A RAR archive.
    Rar,
    /// A 7-Zip archive.
    SevenZip,
    /// A ustar tape archive.
    Tar,
    /// An xz stream.
    Xz,
    /// A bzip2 stream.
    Bzip2,
    /// A zstd frame.
    Zstd,
}

/// Why a package, signature, length, or hash was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageError {
    /// The bytes are an archive. They were not unpacked.
    Archive(ArchiveKind),
    /// The bytes are not a WebAssembly module.
    NotWasm,
    /// The bytes are WebAssembly, but not a component.
    NotComponent,
    /// A header, integer, or payload ended early.
    Truncated,
    /// A LEB128 integer is non-minimal, longer than five bytes, or does not
    /// fit in a `u32`.
    BadLeb,
    /// A custom section's name length is greater than that section's payload.
    SectionTooLarge,
    /// No `gunmetal-manifest` custom section.
    ManifestMissing,
    /// More than one `gunmetal-manifest` custom section.
    ManifestDuplicate,
    /// The manifest bytes are not UTF-8.
    ManifestNotUtf8,
    /// No detached signature, or it is empty.
    SignatureMissing,
    /// The byte length is not the length that was signed.
    LengthMismatch {
        /// The length of the bytes.
        got: usize,
        /// The length that was signed.
        expected: usize,
    },
    /// The SHA-256 digest is not the digest that was signed.
    HashMismatch {
        /// The digest of the bytes.
        got: [u8; 32],
        /// The digest that was signed.
        expected: [u8; 32],
    },
}

/// A component whose manifest was read from its custom section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Component<'a> {
    /// The `gunmetal-manifest` custom section, as text.
    pub manifest: &'a str,
}

const WASM_MAGIC: [u8; 4] = [0x00, 0x61, 0x73, 0x6d];
const COMPONENT_VERSION: [u8; 4] = [0x0d, 0x00, 0x01, 0x00];
const MANIFEST_NAME: &[u8] = b"gunmetal-manifest";
const CUSTOM: u8 = 0;

/// Opens one component and returns its embedded manifest.
///
/// Archive magic is refused before the WebAssembly header is read. Zip,
/// gzip, RAR, 7-Zip, xz, bzip2, and zstd are recognised by their prefix.
/// A ustar archive is recognised when the input is longer than 262 bytes
/// and bytes 257..262 are `ustar`. Nothing after a matched magic is read.
///
/// # Errors
///
/// Returns [`PackageError::Archive`] for an archive,
/// [`PackageError::NotWasm`] when the bytes are not a WebAssembly module,
/// [`PackageError::NotComponent`] when the version is not the component
/// version, [`PackageError::Truncated`] when a size or payload ends early,
/// [`PackageError::BadLeb`] for a malformed LEB128 integer,
/// [`PackageError::SectionTooLarge`] when a custom section's name does not
/// fit in that section, [`PackageError::ManifestMissing`] when the manifest
/// section is absent, [`PackageError::ManifestDuplicate`] when it appears
/// twice, and [`PackageError::ManifestNotUtf8`] when its bytes are not
/// UTF-8.
#[must_use = "a refused package must not be installed"]
pub fn open(bytes: &[u8]) -> Result<Component<'_>, PackageError> {
    if let Some(kind) = archive_kind(bytes) {
        return Err(PackageError::Archive(kind));
    }
    let (magic, rest) = bytes.split_at_checked(4).ok_or(PackageError::NotWasm)?;
    if magic != WASM_MAGIC {
        return Err(PackageError::NotWasm);
    }
    let (version, body) = rest.split_at_checked(4).ok_or(PackageError::NotWasm)?;
    if version != COMPONENT_VERSION {
        return Err(PackageError::NotComponent);
    }
    sections(body)
}

/// Returns the detached signature, or refuses a missing one.
///
/// # Errors
///
/// Returns [`PackageError::SignatureMissing`] when `signature` is [`None`]
/// or empty.
#[must_use = "a missing signature must not be treated as present"]
pub const fn require_signature(signature: Option<&[u8]>) -> Result<&[u8], PackageError> {
    if let Some(bytes) = signature {
        if !bytes.is_empty() {
            return Ok(bytes);
        }
    }
    Err(PackageError::SignatureMissing)
}

/// Checks the length, then the SHA-256 digest, of `bytes`.
///
/// On success the digest is the one [`sha256`] computed. The length is
/// checked first, so a wrong length is never reported as a hash mismatch.
///
/// # Errors
///
/// Returns [`PackageError::LengthMismatch`] when `bytes.len()` is not
/// `expected_len`, and [`PackageError::HashMismatch`] when the digest is
/// not `expected_hash`.
#[must_use = "a package whose length or hash does not match must not be installed"]
pub fn length_and_hash(
    bytes: &[u8],
    expected_len: usize,
    expected_hash: [u8; 32],
) -> Result<[u8; 32], PackageError> {
    if bytes.len() != expected_len {
        return Err(PackageError::LengthMismatch {
            got: bytes.len(),
            expected: expected_len,
        });
    }
    let digest = sha256(bytes);
    if digest != expected_hash {
        return Err(PackageError::HashMismatch {
            got: digest,
            expected: expected_hash,
        });
    }
    Ok(digest)
}

/// The archive kind of `bytes`, or [`None`] when they are not an archive.
///
/// A matched prefix is not read past. The ustar check looks only at bytes
/// 257..262, and only when no prefix matched.
fn archive_kind(bytes: &[u8]) -> Option<ArchiveKind> {
    if has_prefix(bytes, b"PK\x03\x04")
        || has_prefix(bytes, b"PK\x05\x06")
        || has_prefix(bytes, b"PK\x07\x08")
    {
        return Some(ArchiveKind::Zip);
    }
    if has_prefix(bytes, &[0x1f, 0x8b]) {
        return Some(ArchiveKind::Gzip);
    }
    if has_prefix(bytes, b"Rar!") {
        return Some(ArchiveKind::Rar);
    }
    if has_prefix(bytes, &[0x37, 0x7a, 0xbc, 0xaf, 0x27, 0x1c]) {
        return Some(ArchiveKind::SevenZip);
    }
    if has_prefix(bytes, &[0xfd, 0x37, 0x7a, 0x58, 0x5a, 0x00]) {
        return Some(ArchiveKind::Xz);
    }
    if has_prefix(bytes, b"BZh") {
        return Some(ArchiveKind::Bzip2);
    }
    if has_prefix(bytes, &[0x28, 0xb5, 0x2f, 0xfd]) {
        return Some(ArchiveKind::Zstd);
    }
    if bytes.len() > 262 && bytes.get(257..262) == Some(b"ustar".as_slice()) {
        return Some(ArchiveKind::Tar);
    }
    None
}

/// Whether `bytes` begins with `magic`, without reading past it.
fn has_prefix(bytes: &[u8], magic: &[u8]) -> bool {
    bytes.get(..magic.len()) == Some(magic)
}

/// The manifest custom section, after every other section has been skipped.
fn sections(mut body: &[u8]) -> Result<Component<'_>, PackageError> {
    let mut manifest = None;
    while let Some((&id, rest)) = body.split_first() {
        body = rest;
        let (size, rest) = read_leb_u32(body)?;
        body = rest;
        let (payload, rest) = split_counted(body, size).ok_or(PackageError::Truncated)?;
        body = rest;
        if id == CUSTOM {
            if let Some(raw) = custom_name(payload)? {
                if manifest.is_some() {
                    return Err(PackageError::ManifestDuplicate);
                }
                let Ok(text) = core::str::from_utf8(raw) else {
                    return Err(PackageError::ManifestNotUtf8);
                };
                manifest = Some(text);
            }
        }
    }
    let manifest = manifest.ok_or(PackageError::ManifestMissing)?;
    Ok(Component { manifest })
}

/// The manifest bytes when `payload` names `gunmetal-manifest`.
///
/// Other names are [`None`]. A name length that does not fit in `payload`
/// is [`PackageError::SectionTooLarge`]: the section itself was present.
fn custom_name(payload: &[u8]) -> Result<Option<&[u8]>, PackageError> {
    let (name_len, rest) = read_leb_u32(payload)?;
    let (name, manifest) = split_counted(rest, name_len).ok_or(PackageError::SectionTooLarge)?;
    if name == MANIFEST_NAME {
        Ok(Some(manifest))
    } else {
        Ok(None)
    }
}

/// Splits `count` bytes from `input`, or [`None`] when they are not there.
///
/// A `u32` that does not fit in `usize` is treated as longer than any slice
/// this process can hold.
fn split_counted(input: &[u8], count: u32) -> Option<(&[u8], &[u8])> {
    input.split_at_checked(usize::try_from(count).unwrap_or(usize::MAX))
}

/// A minimal unsigned LEB128 `u32`, and the bytes after it.
///
/// A missing byte is [`PackageError::Truncated`]. A non-minimal encoding,
/// a sixth byte, or a value that does not fit in `u32` is
/// [`PackageError::BadLeb`].
fn read_leb_u32(input: &[u8]) -> Result<(u32, &[u8]), PackageError> {
    let mut value = 0_u32;
    let mut shift = 0_u32;
    let mut rest = input;
    loop {
        let (&byte, next) = rest.split_first().ok_or(PackageError::Truncated)?;
        rest = next;
        // The fifth byte holds at most four value bits. A continuation, or
        // any higher bit, is longer than five bytes or does not fit in u32.
        if shift == 28 && byte > 0x0f {
            return Err(PackageError::BadLeb);
        }
        let bits = u32::from(byte & 0x7f);
        value |= bits << shift;
        if byte & 0x80 == 0 {
            if shift > 0 && bits == 0 {
                return Err(PackageError::BadLeb);
            }
            return Ok((value, rest));
        }
        shift += 7;
    }
}

#[cfg(test)]
mod tests {
    use super::{ArchiveKind, Component, PackageError, length_and_hash, open, require_signature};
    use proptest::prelude::*;

    /// Hand-built component: magic, component version `0d 00 01 00`, one
    /// custom section. `gunmetal-manifest` is 17 bytes, so the name length
    /// is `0x11` and the payload (length, name, `id=1`) is 22 bytes.
    const MINIMAL: &[u8] = &[
        0x00, 0x61, 0x73, 0x6d, 0x0d, 0x00, 0x01, 0x00, 0x00, 0x16, 0x11, b'g', b'u', b'n', b'm',
        b'e', b't', b'a', b'l', b'-', b'm', b'a', b'n', b'i', b'f', b'e', b's', b't', b'i', b'd',
        b'=', b'1',
    ];

    /// SHA-256 of [`MINIMAL`], from `hashlib.sha256` before this assertion
    /// was written. Not computed by the code under test.
    const MINIMAL_SHA256: [u8; 32] = [
        0x0e, 0xcb, 0x7d, 0xc9, 0x1b, 0x86, 0xb3, 0xd1, 0xc0, 0x27, 0x2f, 0x8c, 0xcc, 0xf1, 0x6d,
        0x8e, 0xb2, 0x0f, 0xf3, 0x7f, 0x88, 0xb1, 0xf8, 0x66, 0x85, 0x27, 0x15, 0xba, 0xb2, 0x0b,
        0x44, 0xb6,
    ];

    /// SHA-256 of `b"plugin"`, from `hashlib.sha256`.
    const PLUGIN_SHA256: [u8; 32] = [
        0x5e, 0x68, 0x9e, 0x2b, 0x01, 0x67, 0x2b, 0xf3, 0x39, 0x96, 0xe7, 0x5d, 0x5e, 0x37, 0x2f,
        0xf6, 0x0c, 0x53, 0x6c, 0xe1, 0x59, 0x9a, 0x14, 0x58, 0xe8, 0x67, 0xcd, 0x8f, 0x4b, 0xef,
        0x51, 0x60,
    ];

    /// SHA-256 of the empty input, from `hashlib.sha256`.
    const EMPTY_SHA256: [u8; 32] = [
        0xe3, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14, 0x9a, 0xfb, 0xf4, 0xc8, 0x99, 0x6f, 0xb9,
        0x24, 0x27, 0xae, 0x41, 0xe4, 0x64, 0x9b, 0x93, 0x4c, 0xa4, 0x95, 0x99, 0x1b, 0x78, 0x52,
        0xb8, 0x55,
    ];

    const HEADER: [u8; 8] = [0x00, 0x61, 0x73, 0x6d, 0x0d, 0x00, 0x01, 0x00];

    fn with_header(rest: &[u8]) -> Vec<u8> {
        let mut bytes = HEADER.to_vec();
        bytes.extend_from_slice(rest);
        bytes
    }

    fn push_leb(out: &mut Vec<u8>, mut value: u32) {
        loop {
            let low = u8::try_from(value & 0x7f).unwrap_or(0);
            value >>= 7;
            if value == 0 {
                out.push(low);
                return;
            }
            out.push(low | 0x80);
        }
    }

    fn section(id: u8, payload: &[u8]) -> Vec<u8> {
        let mut bytes = vec![id];
        push_leb(&mut bytes, u32::try_from(payload.len()).unwrap_or(u32::MAX));
        bytes.extend_from_slice(payload);
        bytes
    }

    fn manifest_payload(manifest: &[u8]) -> Vec<u8> {
        let mut payload = Vec::new();
        push_leb(&mut payload, 17);
        payload.extend_from_slice(b"gunmetal-manifest");
        payload.extend_from_slice(manifest);
        payload
    }

    fn manifest_section(manifest: &[u8]) -> Vec<u8> {
        section(0, &manifest_payload(manifest))
    }

    fn component(manifest: &[u8]) -> Vec<u8> {
        with_header(&manifest_section(manifest))
    }

    fn component_named(name: &[u8], manifest: &[u8]) -> Vec<u8> {
        let mut payload = Vec::new();
        push_leb(&mut payload, u32::try_from(name.len()).unwrap_or(u32::MAX));
        payload.extend_from_slice(name);
        payload.extend_from_slice(manifest);
        with_header(&section(0, &payload))
    }

    /// The archive magics, written here as the specification states them.
    fn reference_archive(bytes: &[u8]) -> Option<ArchiveKind> {
        if bytes.get(..4) == Some(b"PK\x03\x04".as_slice())
            || bytes.get(..4) == Some(b"PK\x05\x06".as_slice())
            || bytes.get(..4) == Some(b"PK\x07\x08".as_slice())
        {
            return Some(ArchiveKind::Zip);
        }
        if bytes.get(..2) == Some(&[0x1f, 0x8b][..]) {
            return Some(ArchiveKind::Gzip);
        }
        if bytes.get(..4) == Some(b"Rar!".as_slice()) {
            return Some(ArchiveKind::Rar);
        }
        if bytes.get(..6) == Some(&[0x37, 0x7a, 0xbc, 0xaf, 0x27, 0x1c][..]) {
            return Some(ArchiveKind::SevenZip);
        }
        if bytes.get(..6) == Some(&[0xfd, 0x37, 0x7a, 0x58, 0x5a, 0x00][..]) {
            return Some(ArchiveKind::Xz);
        }
        if bytes.get(..3) == Some(b"BZh".as_slice()) {
            return Some(ArchiveKind::Bzip2);
        }
        if bytes.get(..4) == Some(&[0x28, 0xb5, 0x2f, 0xfd][..]) {
            return Some(ArchiveKind::Zstd);
        }
        if bytes.len() > 262 && bytes.get(257..262) == Some(b"ustar".as_slice()) {
            return Some(ArchiveKind::Tar);
        }
        None
    }

    /// Verifies: SEC-EXT-034
    #[test]
    fn a_hand_built_component_yields_its_manifest() {
        assert_eq!(MINIMAL.len(), 32);
        assert_eq!(MINIMAL[9], 22);
        assert_eq!(MINIMAL[10], 17);
        assert_eq!(&MINIMAL[11..28], b"gunmetal-manifest");
        assert_eq!(&MINIMAL[28..], b"id=1");
        assert_eq!(open(MINIMAL), Ok(Component { manifest: "id=1" }));
        assert_eq!(
            open(&component(b"id=1\n")),
            Ok(Component { manifest: "id=1\n" })
        );
        assert_eq!(open(&component(b"")), Ok(Component { manifest: "" }));
        assert_eq!(
            open(&component("id=café".as_bytes())),
            Ok(Component {
                manifest: "id=café"
            })
        );
    }

    /// Verifies: SEC-EXT-034
    #[test]
    fn an_archive_is_refused_at_its_magic() {
        let cases: &[(&[u8], ArchiveKind)] = &[
            (b"PK\x03\x04wasm", ArchiveKind::Zip),
            (b"PK\x03\x04", ArchiveKind::Zip),
            (b"PK\x05\x06", ArchiveKind::Zip),
            (b"PK\x07\x08rest", ArchiveKind::Zip),
            (&[0x1f, 0x8b], ArchiveKind::Gzip),
            (&[0x1f, 0x8b, 0x08], ArchiveKind::Gzip),
            (b"Rar!", ArchiveKind::Rar),
            (b"Rar!\x1a\x07\x00", ArchiveKind::Rar),
            (&[0x37, 0x7a, 0xbc, 0xaf, 0x27, 0x1c], ArchiveKind::SevenZip),
            (&[0xfd, 0x37, 0x7a, 0x58, 0x5a, 0x00], ArchiveKind::Xz),
            (b"BZh", ArchiveKind::Bzip2),
            (b"BZh9", ArchiveKind::Bzip2),
            (&[0x28, 0xb5, 0x2f, 0xfd], ArchiveKind::Zstd),
            (&[0x28, 0xb5, 0x2f, 0xfd, 0x00], ArchiveKind::Zstd),
        ];
        for (bytes, kind) in cases {
            assert_eq!(open(bytes), Err(PackageError::Archive(*kind)), "{bytes:?}");
            assert_eq!(reference_archive(bytes), Some(*kind), "{bytes:?}");
        }
        let mut packed = b"PK\x03\x04".to_vec();
        packed.extend_from_slice(MINIMAL);
        assert_eq!(open(&packed), Err(PackageError::Archive(ArchiveKind::Zip)));

        let mut tar = vec![0_u8; 263];
        tar[257..262].copy_from_slice(b"ustar");
        assert_eq!(open(&tar), Err(PackageError::Archive(ArchiveKind::Tar)));
        assert_eq!(reference_archive(&tar), Some(ArchiveKind::Tar));
        tar[262] = b'X';
        assert_eq!(open(&tar), Err(PackageError::Archive(ArchiveKind::Tar)));
        assert_eq!(reference_archive(&tar), Some(ArchiveKind::Tar));

        let mut zip = vec![0_u8; 263];
        zip[..4].copy_from_slice(b"PK\x03\x04");
        zip[257..262].copy_from_slice(b"ustar");
        assert_eq!(open(&zip), Err(PackageError::Archive(ArchiveKind::Zip)));

        let mut disguised = vec![0_u8; 263];
        disguised[..8].copy_from_slice(&HEADER);
        disguised[257..262].copy_from_slice(b"ustar");
        assert_eq!(
            open(&disguised),
            Err(PackageError::Archive(ArchiveKind::Tar))
        );
    }

    /// Verifies: SEC-EXT-034
    #[test]
    fn a_short_or_other_file_is_not_wasm() {
        let cases: &[&[u8]] = &[
            b"",
            &[0, 0, 0, 0, 0, 0, 0],
            b"\0asm",
            b"\0asm\x0d\x00\x01",
            b"PK",
            b"PK\x03",
            b"PK\x03\x05xxxx",
            b"PK\x01\x02xxxx",
            &[0x1f],
            &[0x1f, 0x8c, 0, 0, 0, 0, 0, 0],
            b"Rar?",
            b"rar!xxxx",
            b"BZ",
            b"BZixxxx",
            &[0x37, 0x7a, 0xbc, 0xaf, 0x27, 0x00],
            &[0xfd, 0x37, 0x7a, 0x58, 0x5a, 0x01],
            &[0x28, 0xb5, 0x2f, 0xfe, 0, 0, 0, 0],
            b"wasm\x0d\x00\x01\x00",
            b"\0asM\x0d\x00\x01\x00",
        ];
        for bytes in cases {
            assert_eq!(open(bytes), Err(PackageError::NotWasm), "{bytes:?}");
        }
        let mut almost = vec![0_u8; 262];
        almost[257..262].copy_from_slice(b"ustar");
        assert_eq!(open(&almost), Err(PackageError::NotWasm));
        let mut shifted = vec![0_u8; 263];
        shifted[256..261].copy_from_slice(b"ustar");
        assert_eq!(open(&shifted), Err(PackageError::NotWasm));
        shifted.fill(0);
        shifted[258..263].copy_from_slice(b"ustar");
        assert_eq!(open(&shifted), Err(PackageError::NotWasm));
        shifted.fill(0);
        shifted[257..262].copy_from_slice(b"ustaq");
        assert_eq!(open(&shifted), Err(PackageError::NotWasm));
    }

    /// Verifies: SEC-EXT-034
    #[test]
    fn a_core_module_is_not_a_component() {
        let versions: &[[u8; 4]] = &[
            [0x01, 0x00, 0x00, 0x00],
            [0x02, 0x00, 0x00, 0x00],
            [0x0d, 0x00, 0x00, 0x00],
            [0x0d, 0x00, 0x01, 0x01],
            [0x0d, 0x00, 0x02, 0x00],
            [0x00, 0x01, 0x00, 0x0d],
            [0x00, 0x00, 0x00, 0x00],
        ];
        for version in versions {
            let mut bytes = b"\0asm".to_vec();
            bytes.extend_from_slice(version);
            bytes.extend_from_slice(&MINIMAL[8..]);
            assert_eq!(open(&bytes), Err(PackageError::NotComponent), "{version:?}");
        }
    }

    /// Verifies: SEC-EXT-034
    #[test]
    fn a_component_without_the_manifest_section_is_refused() {
        assert_eq!(open(&HEADER), Err(PackageError::ManifestMissing));
        assert_eq!(
            open(&component_named(b"other", b"id=1")),
            Err(PackageError::ManifestMissing)
        );
        assert_eq!(
            open(&component_named(b"gunmetal-manifes", b"id=1")),
            Err(PackageError::ManifestMissing)
        );
        assert_eq!(
            open(&component_named(b"gunmetal-manifest-x", b"id=1")),
            Err(PackageError::ManifestMissing)
        );
        assert_eq!(
            open(&component_named(b"gunmetal-manifesu", b"id=1")),
            Err(PackageError::ManifestMissing)
        );
        assert_eq!(
            open(&with_header(&section(1, &manifest_payload(b"id=1")))),
            Err(PackageError::ManifestMissing)
        );
        assert_eq!(
            open(&with_header(&section(0, &[0x00]))),
            Err(PackageError::ManifestMissing)
        );
    }

    /// Verifies: SEC-EXT-034
    #[test]
    fn a_second_manifest_section_is_a_duplicate() {
        let mut bytes = component(b"id=1");
        bytes.extend_from_slice(&manifest_section(b"id=2"));
        assert_eq!(open(&bytes), Err(PackageError::ManifestDuplicate));
        let mut bad_second = component(b"id=1");
        bad_second.extend_from_slice(&manifest_section(&[0xff]));
        assert_eq!(open(&bad_second), Err(PackageError::ManifestDuplicate));
        let mut other = component(b"id=1");
        other.extend_from_slice(&section(0, b"\x05other"));
        assert_eq!(open(&other), Ok(Component { manifest: "id=1" }));
    }

    /// Verifies: SEC-EXT-034
    #[test]
    fn a_manifest_that_is_not_utf8_is_refused() {
        assert_eq!(
            open(&component(&[0xff])),
            Err(PackageError::ManifestNotUtf8)
        );
        assert_eq!(
            open(&component(&[0xc3])),
            Err(PackageError::ManifestNotUtf8)
        );
        let mut later = component(&[0xff]);
        later.extend_from_slice(&manifest_section(b"id=1"));
        assert_eq!(open(&later), Err(PackageError::ManifestNotUtf8));
        let mut ignored = with_header(&section(0, &[0x01, 0xff]));
        ignored.extend_from_slice(&manifest_section(b"id=1"));
        assert_eq!(open(&ignored), Ok(Component { manifest: "id=1" }));
    }

    /// Verifies: SEC-EXT-034
    #[test]
    fn a_truncated_or_non_minimal_section_is_refused() {
        assert_eq!(open(&with_header(&[0x00])), Err(PackageError::Truncated));
        assert_eq!(
            open(&with_header(&[0x00, 0x80])),
            Err(PackageError::Truncated)
        );
        assert_eq!(
            open(&with_header(&[0x00, 0x0a, 1, 2, 3])),
            Err(PackageError::Truncated)
        );
        assert_eq!(
            open(&with_header(&[0x00, 0x05])),
            Err(PackageError::Truncated)
        );
        assert_eq!(
            open(&with_header(&[0x00, 0x00])),
            Err(PackageError::Truncated)
        );

        let mut past = with_header(&[0x00, 0x01, 0x80]);
        past.push(0x01);
        past.extend(core::iter::repeat_n(b'x', 128));
        assert_eq!(open(&past), Err(PackageError::Truncated));

        let mut overlong = with_header(&[0x00, 0x96, 0x00]);
        overlong.extend_from_slice(&MINIMAL[10..]);
        assert_eq!(open(&overlong), Err(PackageError::BadLeb));
        let mut six = with_header(&[0x00, 0x96, 0x80, 0x80, 0x80, 0x80, 0x00]);
        six.extend_from_slice(&MINIMAL[10..]);
        assert_eq!(open(&six), Err(PackageError::BadLeb));
        assert_eq!(
            open(&with_header(&[0x00, 0x80, 0x80, 0x80, 0x80, 0x80])),
            Err(PackageError::BadLeb)
        );
        assert_eq!(
            open(&with_header(&[0x00, 0x80, 0x80, 0x80, 0x80, 0x10])),
            Err(PackageError::BadLeb)
        );
        assert_eq!(
            open(&with_header(&[0x00, 0x80, 0x00])),
            Err(PackageError::BadLeb)
        );
        assert_eq!(
            open(&with_header(&[0x00, 0xff, 0xff, 0xff, 0xff, 0x0f])),
            Err(PackageError::Truncated)
        );
        assert_eq!(
            open(&with_header(&[0x00, 0x80, 0x80, 0x7f])),
            Err(PackageError::Truncated)
        );
        assert_eq!(
            open(&with_header(&[0x00, 0x80, 0x80, 0x80, 0x7f])),
            Err(PackageError::Truncated)
        );

        assert_eq!(
            open(&with_header(&[0x00, 0x01, 0x11])),
            Err(PackageError::SectionTooLarge)
        );
        assert_eq!(
            open(&with_header(&[0x00, 0x05, 0x11, 1, 2, 3, 4])),
            Err(PackageError::SectionTooLarge)
        );
        assert_eq!(
            open(&with_header(&[0x00, 0x05, 0xff, 0xff, 0xff, 0xff, 0x0f])),
            Err(PackageError::SectionTooLarge)
        );
    }

    /// Verifies: SEC-EXT-034
    #[test]
    fn sections_other_than_the_manifest_are_skipped() {
        let mut bytes = with_header(&section(1, b"PK\x03\x04"));
        bytes.extend_from_slice(&section(255, &[0xff, 0xfe]));
        bytes.extend_from_slice(&section(0, b"\x05other"));
        bytes.extend_from_slice(&manifest_section(b"id=1"));
        bytes.extend_from_slice(&section(2, &[]));
        assert_eq!(open(&bytes), Ok(Component { manifest: "id=1" }));

        let skipped = vec![0_u8; 257];
        let mut long = with_header(&section(3, &skipped));
        long.extend_from_slice(&manifest_section(b"id=1"));
        assert!(long.len() > 262);
        assert_eq!(open(&long), Ok(Component { manifest: "id=1" }));

        let text = "a".repeat(110);
        assert_eq!(
            open(&component(text.as_bytes())),
            Ok(Component {
                manifest: text.as_str(),
            })
        );
        let mut hostile_name = vec![0x80, 0x02];
        hostile_name.extend(core::iter::repeat_n(0xff, 256));
        let mut hostile = with_header(&section(0, &hostile_name));
        hostile.extend_from_slice(&manifest_section(b"id=1"));
        assert_eq!(open(&hostile), Ok(Component { manifest: "id=1" }));

        let mut trailing = MINIMAL.to_vec();
        trailing.push(0);
        assert_eq!(open(&trailing), Err(PackageError::Truncated));
    }

    /// Verifies: SEC-EXT-034
    #[test]
    fn a_missing_or_empty_signature_is_refused() {
        assert_eq!(require_signature(None), Err(PackageError::SignatureMissing));
        assert_eq!(
            require_signature(Some(b"")),
            Err(PackageError::SignatureMissing)
        );
        let signature = b"detached-signature";
        let accepted = require_signature(Some(signature));
        assert_eq!(accepted, Ok(signature.as_slice()));
        assert_eq!(accepted.map(<[u8]>::as_ptr), Ok(signature.as_ptr()));
        assert_eq!(require_signature(Some(&[0xff])), Ok(&[0xff][..]));
    }

    /// Verifies: SEC-EXT-034
    #[test]
    fn length_and_hash_names_both_sides_of_a_mismatch() {
        assert_eq!(
            length_and_hash(MINIMAL, MINIMAL.len(), MINIMAL_SHA256),
            Ok(MINIMAL_SHA256)
        );
        assert_eq!(
            length_and_hash(b"plugin", b"plugin".len(), PLUGIN_SHA256),
            Ok(PLUGIN_SHA256)
        );
        assert_eq!(length_and_hash(b"", 0, EMPTY_SHA256), Ok(EMPTY_SHA256));
        assert_eq!(
            length_and_hash(MINIMAL, 1, MINIMAL_SHA256),
            Err(PackageError::LengthMismatch {
                got: MINIMAL.len(),
                expected: 1,
            })
        );
        assert_eq!(
            length_and_hash(b"ab", 3, [0; 32]),
            Err(PackageError::LengthMismatch {
                got: 2,
                expected: 3,
            })
        );
        assert_eq!(
            length_and_hash(b"", 1, EMPTY_SHA256),
            Err(PackageError::LengthMismatch {
                got: 0,
                expected: 1,
            })
        );
        assert_eq!(
            length_and_hash(MINIMAL, MINIMAL.len(), [0x11; 32]),
            Err(PackageError::HashMismatch {
                got: MINIMAL_SHA256,
                expected: [0x11; 32],
            })
        );
        assert_eq!(
            length_and_hash(b"plugin", 6, [0; 32]),
            Err(PackageError::HashMismatch {
                got: PLUGIN_SHA256,
                expected: [0; 32],
            })
        );
    }

    /// Verifies: SEC-EXT-034
    #[test]
    fn open_on_the_listed_short_slices_returns_an_error() {
        let cases: &[&[u8]] = &[b"", &[0, 0, 0, 0, 0, 0, 0], b"\0asm"];
        for bytes in cases {
            assert_eq!(open(bytes), Err(PackageError::NotWasm), "{bytes:?}");
        }
    }

    proptest! {
        /// Verifies: SEC-EXT-034
        #[test]
        fn open_on_a_short_slice_returns_an_error(
            bytes in proptest::collection::vec(any::<u8>(), 0..8)
        ) {
            let expected = match reference_archive(&bytes) {
                Some(kind) => Err(PackageError::Archive(kind)),
                None => Err(PackageError::NotWasm),
            };
            prop_assert_eq!(open(&bytes), expected);
        }
    }
}
