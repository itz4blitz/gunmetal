//! The harness for the streaming decompression helper in
//! [`gunmetal_core::inflate`] (SEC-MED-009).

use gunmetal_core::inflate::{self, BoundedBuf, Framing, InflateError, Target};
use gunmetal_core::parse::{Budget, Limits, ParseFault};

/// The inflated size every call declares, which caps its output: small
/// enough that a few dozen octets of input reach it, so fuzzing meets the
/// cap often.
pub const CAP: u64 = 65_536;

/// What the helper reported for one input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// [`Framing::from_content_comp_algo`] of the input's first eight
    /// octets read as a little-endian Matroska `ContentCompAlgo`, with
    /// zeros for the octets a shorter input lacks.
    pub algo: Result<Framing, InflateError>,
    /// [`inflate::inflate`] of the input read as zlib.
    pub zlib: Inflated,
    /// [`inflate::inflate`] of the input read as raw deflate.
    pub deflate: Inflated,
}

/// One call's result and the octets it left in its buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inflated {
    /// The input octets the stream took, or the error.
    pub result: Result<u64, InflateError>,
    /// The octets in the buffer when the call returned.
    pub output: Vec<u8>,
}

/// Feeds `data` to [`inflate::inflate`] as zlib, inflating to a picture,
/// and as raw deflate, inflating to a header, each time into a buffer that
/// declares [`CAP`] octets, with a step budget of one step per input octet
/// plus [`CAP`], the bound the helper documents (SEC-MED-007). It also
/// reads the first eight octets as a Matroska `ContentCompAlgo` and asks
/// the helper which framing that names.
///
/// # Panics
///
/// Panics when the helper breaks an invariant that holds for every input:
/// it caps a buffer above the context's limit, runs out of that budget,
/// holds or reserves more than its cap, reports more input taken than there
/// is, reports an error at an offset past the end of the input, or takes
/// any `ContentCompAlgo` but 0 (SEC-MED-009).
#[must_use]
pub fn run(data: &[u8]) -> Outcome {
    let mut head = [0; 8];
    for (slot, &octet) in head.iter_mut().zip(data) {
        *slot = octet;
    }
    let value = u64::from_le_bytes(head);
    let algo = Framing::from_content_comp_algo(value);
    assert_eq!(algo.is_ok(), value == 0);
    Outcome {
        algo,
        zlib: run_one(data, Framing::Zlib, Target::Picture),
        deflate: run_one(data, Framing::Deflate, Target::Header),
    }
}

/// One call of the helper on `data`, framed as `framing`, inflating to
/// `target`, with its invariants checked.
fn run_one(data: &[u8], framing: Framing, target: Target) -> Inflated {
    let octets = u64::try_from(data.len()).unwrap_or(u64::MAX);
    let limits = Limits::DEFAULT;
    let mut out = BoundedBuf::new(&limits, target, Some(CAP));
    let cap = out.cap();
    assert!(cap <= limits.get(target.limit()));
    let mut budget = Budget::for_input(octets, 1, CAP);
    let result = inflate::inflate(data, framing, &mut budget, &mut out);
    let taken = match result {
        Ok(read) => read,
        Err(error) => error.offset(),
    };
    assert!(taken <= octets);
    assert!(!matches!(
        result,
        Err(InflateError::Fault(ParseFault::BudgetExceeded { .. }))
    ));
    let made = u64::try_from(out.as_slice().len()).unwrap_or(u64::MAX);
    let held = u64::try_from(out.capacity()).unwrap_or(u64::MAX);
    assert!(made <= cap);
    assert!(held <= cap);
    Inflated {
        result,
        output: out.into_vec(),
    }
}
