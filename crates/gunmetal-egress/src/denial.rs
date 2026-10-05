//! Why the gate refuses a request.

use core::net::IpAddr;

use gunmetal_core::net::AddrClass;

/// Why a request was refused before it left the server.
///
/// Each refusal is one security event and one line of the activity record
/// (SEC-PRV-008).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Denial {
    /// Offline mode is on, which blocks every outbound request.
    Offline,
    /// The server is not claimed yet, and the purpose is not one that may
    /// be used before the claim.
    BeforeClaim,
    /// The owner has not granted the purpose.
    PurposeNotGranted,
    /// The purpose is granted, but not this scheme, host and port.
    DestinationNotGranted,
    /// Requests go through the admin's proxy, which resolves names itself,
    /// and the destination is an address or a LAN destination, whose
    /// addresses could then not be checked.
    NotThroughProxy,
    /// The name resolved to no address.
    NoAddress,
    /// The name resolved to an address the destination may not reach.
    AddressRefused {
        /// The address, in its canonical form.
        address: IpAddr,
        /// What kind of address it is.
        class: AddrClass,
    },
    /// The request follows no redirects.
    RedirectsRefused,
    /// The request was already redirected as often as it may be.
    TooManyRedirects,
    /// The redirect leads to another host.
    CrossHostRedirect,
}
