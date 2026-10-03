//! The untrusted wrapper as every crate outside the core sees it
//! (SEC-TM-031).
//!
//! Outside the core, an [`Untrusted`] value can be made and handed to a
//! validating constructor, and nothing else. The unit tests in
//! `src/untrusted.rs` prove that it has no trait conversion to a path, a
//! process argument, an address or text. The tests here prove the rest from
//! where other crates stand: that no method or field hands the wrapped value
//! out, and that every validating constructor of the core takes it, so a
//! server or client that receives a value can keep it wrapped until a
//! validator turns it into a domain type or refuses it.

use std::net::{IpAddr, Ipv4Addr};

use gunmetal_core::base64::{self, Alphabet, B64Error};
use gunmetal_core::link::{self, HOME, Link, LinkError};
use gunmetal_core::net::{IpNet, NetError};
use gunmetal_core::text::{self, Encoding, Lines, Text};
use gunmetal_core::time::{self, TimeError};
use gunmetal_core::untrusted::Untrusted;
use gunmetal_core::values::{
    Field, GainDb, Isrc, Mbid, NumberOf, PartialDate, PeakRatio, ValueError,
};

/// What a probe below answers. A public method of the same name on
/// `Untrusted` answers with something else, and the call stops compiling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Probed;

/// Names the value namespace's `Untrusted` here. While the wrapper's field is
/// private, its tuple constructor is invisible outside the core and the
/// import above brings only the type, so this function may take the name. A
/// public field makes the constructor public, the import brings it too, and
/// the two names collide (E0255), so the test stops compiling.
#[expect(
    non_snake_case,
    reason = "it must share the wrapper's name to collide with a public constructor"
)]
fn Untrusted() -> Probed {
    Probed
}

/// Hands the names an accessor for the wrapped value would plausibly have to
/// `$callback`, so that the probe trait and the calls below share one list.
macro_rules! with_accessor_names {
    ($callback:ident!($($args:tt)*)) => {
        $callback! {
            [
                as_bytes, as_inner, as_mut, as_ref, as_str, expose, get, get_mut,
                get_ref, inner, into_inner, raw, take, unwrap, value,
            ]
            $($args)*
        }
    };
}

/// Defines one probe method per name. A method call tries a type's own
/// methods before trait methods, first on `self`, then on `&self`, then on
/// `&mut self`. A probe takes `&mut self`, the last of the three, so an
/// accessor of the same name with any of those receivers takes the call
/// wherever it is visible. Today every such method is private to the core
/// or does not exist, so the probe answers.
macro_rules! define_probes {
    ([$($name:ident,)*]) => {
        #[expect(
            clippy::wrong_self_convention,
            reason = "every probe takes `&mut self`, so `into_inner` does too"
        )]
        trait Probe {
            $(fn $name(&mut self) -> Probed;)*
        }
        impl<T> Probe for Untrusted<T> {
            $(fn $name(&mut self) -> Probed {
                Probed
            })*
        }
    };
}

with_accessor_names!(define_probes!());

/// Calls every probe on `$wrapped`, by its concrete type, so an accessor
/// defined only for that type is found too.
macro_rules! probe_every_name {
    ([$($name:ident,)*] $wrapped:expr) => {{
        let mut wrapped = $wrapped;
        [$(wrapped.$name()),*]
    }};
}

/// No public method or field hands the wrapped value out, whatever the
/// wrapper holds: each call below reaches the probe, not an accessor of the
/// core, and the constructor that a public field would bring is not
/// visible. An accessor with a name outside the list would pass; review of
/// `src/untrusted.rs`, which stays a few lines long, covers that.
///
/// Verifies: SEC-TM-031
#[test]
fn hands_the_wrapped_value_to_nothing_outside_the_core() {
    let every_probe = [Probed; 15];
    assert_eq!(
        with_accessor_names!(probe_every_name!(Untrusted::new("text"))),
        every_probe
    );
    assert_eq!(
        with_accessor_names!(probe_every_name!(Untrusted::new(b"octets".as_slice()))),
        every_probe
    );
    assert_eq!(
        with_accessor_names!(probe_every_name!(Untrusted::new(String::from("text")))),
        every_probe
    );
    assert_eq!(
        with_accessor_names!(probe_every_name!(Untrusted::new(b"octets".to_vec()))),
        every_probe
    );
    assert_eq!(Untrusted(), Probed);
}

// Each validating constructor of the core, called from outside it with an
// untrusted value: a well-formed value becomes the domain type, and a hostile
// one is refused with the exact error.

/// Verifies: SEC-TM-031
#[test]
fn an_mbid_leaves_the_wrapper_only_through_its_validator() {
    assert_eq!(
        Mbid::parse(Untrusted::new("F81D4FAE-7DEC-11D0-A765-00A0C91E6BF6"))
            .map(|mbid| mbid.to_string()),
        Ok("f81d4fae-7dec-11d0-a765-00a0c91e6bf6".to_owned())
    );
    assert_eq!(
        Mbid::parse(Untrusted::new("../../../../../../../../etc/passwd")),
        Err(ValueError::Malformed { field: Field::Mbid })
    );
}

/// Verifies: SEC-TM-031
#[test]
fn an_isrc_leaves_the_wrapper_only_through_its_validator() {
    assert_eq!(
        Isrc::parse(Untrusted::new("us-s1z-99-00001")).map(|isrc| isrc.as_str().to_owned()),
        Ok("USS1Z9900001".to_owned())
    );
    assert_eq!(
        Isrc::parse(Untrusted::new("US'; DROP --")),
        Err(ValueError::Malformed { field: Field::Isrc })
    );
}

/// Verifies: SEC-TM-031
#[test]
fn a_track_number_leaves_the_wrapper_only_through_its_validator() {
    assert_eq!(
        NumberOf::parse(Untrusted::new("03 of 12")).map(|number| (number.number(), number.total())),
        Ok((3, Some(12)))
    );
    assert_eq!(
        NumberOf::parse(Untrusted::new("13/12")),
        Err(ValueError::AboveTotal {
            number: 13,
            total: 12
        })
    );
}

/// Verifies: SEC-TM-031
#[test]
fn a_date_leaves_the_wrapper_only_through_its_validator() {
    assert_eq!(
        PartialDate::parse(Untrusted::new("2019-05-17T07:00:00Z")).map(|date| (
            date.year(),
            date.month(),
            date.day()
        )),
        Ok((2019, Some(5), Some(17)))
    );
    assert_eq!(
        PartialDate::parse(Untrusted::new("2019-02-29")),
        Err(ValueError::OutOfRange {
            field: Field::Day,
            value: 29
        })
    );
}

/// Verifies: SEC-TM-031
#[test]
fn a_gain_leaves_the_wrapper_only_through_its_validator() {
    assert_eq!(
        GainDb::parse(Untrusted::new("-6,5 dB")).map(GainDb::db),
        Ok(-6.5)
    );
    assert_eq!(
        GainDb::parse(Untrusted::new("nan")),
        Err(ValueError::Malformed { field: Field::Gain })
    );
}

/// Verifies: SEC-TM-031
#[test]
fn a_peak_leaves_the_wrapper_only_through_its_validator() {
    assert_eq!(
        PeakRatio::parse(Untrusted::new("0,5")).map(PeakRatio::ratio),
        Ok(0.5)
    );
    assert_eq!(
        PeakRatio::parse(Untrusted::new("16.01")),
        Err(ValueError::Unusable { field: Field::Peak })
    );
}

/// Verifies: SEC-TM-031
#[test]
fn a_timestamp_leaves_the_wrapper_only_through_its_validator() {
    assert_eq!(
        time::parse_rfc3339(Untrusted::new("2009-02-14T00:31:30.123+01:00"))
            .map(time::Timestamp::millis),
        Ok(1_234_567_890_123)
    );
    assert_eq!(
        time::parse_rfc3339(Untrusted::new("2016-12-31T23:59:60Z")),
        Err(TimeError::InvalidTime {
            hour: 23,
            minute: 59,
            second: 60
        })
    );
}

/// Verifies: SEC-TM-031
#[test]
fn a_network_leaves_the_wrapper_only_through_its_validator() {
    assert_eq!(
        IpNet::parse(Untrusted::new("192.168.1.0/24")).map(|net| (net.addr(), net.prefix())),
        Ok((IpAddr::V4(Ipv4Addr::new(192, 168, 1, 0)), 24))
    );
    assert_eq!(
        IpNet::parse(Untrusted::new("192.168.1.1/24")),
        Err(NetError::HostBitsSet)
    );
}

/// Verifies: SEC-TM-031
#[test]
fn base64_leaves_the_wrapper_only_through_its_decoder() {
    assert_eq!(
        base64::decode(Untrusted::new(b"Zm9vYmFy"), Alphabet::Standard, 6),
        Ok(b"foobar".to_vec())
    );
    assert_eq!(
        base64::decode(Untrusted::new(b"Zm9vYmFy"), Alphabet::UrlSafe, 5),
        Err(B64Error::TooLong { needed: 6, max: 5 })
    );
}

/// Verifies: SEC-TM-031
#[test]
fn media_text_leaves_the_wrapper_only_through_its_decoder() {
    assert_eq!(
        text::decode(Untrusted::new(b"Caf\xE9\x85\x1B"), Encoding::Latin1, 64),
        Text {
            value: "Caf\u{E9}".to_owned(),
            truncated: false,
            replaced: false,
        }
    );
}

/// Verifies: SEC-TM-031
#[test]
fn client_text_leaves_the_wrapper_only_through_its_normaliser() {
    assert_eq!(
        text::normalise(Untrusted::new("a\u{202E}b\n".as_bytes()), Lines::Single, 64),
        Text {
            value: "ab".to_owned(),
            truncated: false,
            replaced: false,
        }
    );
}

/// Verifies: SEC-TM-031
#[test]
fn a_link_leaves_the_wrapper_only_through_its_validator() {
    let accepted = Link::parse(Untrusted::new("HTTPS://Example.COM/x"));
    assert_eq!(
        accepted.as_ref().map(|link| (link.href(), link.host())),
        Ok(("https://example.com/x", "example.com"))
    );
    assert_eq!(
        Link::parse(Untrusted::new("javascript:alert(1)")),
        Err(LinkError::NotHttps)
    );
}

/// Verifies: SEC-TM-031
#[test]
fn a_return_target_leaves_the_wrapper_only_through_its_validator() {
    let routes = ["/", "/library"];
    assert_eq!(
        link::return_target(Untrusted::new("/library"), &routes).as_str(),
        "/library"
    );
    assert_eq!(
        link::return_target(Untrusted::new("//evil.example"), &routes).as_str(),
        HOME
    );
}
