//! The one piece of this crate that is compiled only for the browser: the
//! wrapper around each exported function.
//!
//! A mirror value crosses to JavaScript through `tsify::Ts`, and that
//! conversion runs only inside a JavaScript host. So each export is a plain
//! function of mirror types and plain values, tested on the host like all
//! other code, and one line of `export!` beside it, which writes the
//! function the browser calls (record 12, the addition of 2026-10-05):
//!
//! ```text
//! crate::export::export! {
//!     /// The browser's `normaliseText`.
//!     "normaliseText": fn normalise_text_export = normalise_text(input: &str, cap: u32; lines: Lines) -> NormalisedText
//! }
//! ```
//!
//! The wrapper takes the plain parameters as they are and each parameter
//! after the `;` as a `tsify::Ts` of that mirror type. It calls the function
//! once, with every parameter in the order written, and answers with a
//! `tsify::Ts` of the mirror type or a JavaScript error. A line of `export!`
//! can name things and do nothing else, so a wrapper holds no branch, no
//! arithmetic and no value of its own.
//!
//! Host coverage and mutation testing cannot see the wrappers: the host
//! build does not compile them, and the mutation tool does not read what a
//! macro writes. They are the one written exception to those two rules.
//! `xtask facade-wrappers` holds this crate's own files to that exception:
//! it fails when this file differs from the copy it holds, and when another
//! source of the crate holds a condition other than its tests' gate, brings
//! in another file as code or names `wasm_bindgen`. What a macro of another
//! crate writes is not in those files, and the check does not see it; its
//! documentation lists what else it does not see. The `wasm32` build in
//! `.github/workflows/wasm.yml` compiles every wrapper.

/// Writes the browser's wrapper around one function of this crate.
macro_rules! export {
    (
        $(#[doc = $doc:literal])*
        $javascript:literal: fn $export:ident = $function:ident(
            $($plain:ident: $kind:ty),*
            $(; $($crossing:ident: $crossed:ident),+)?
        ) -> $mirror:ident
    ) => {
        $(#[doc = $doc])*
        #[cfg(target_arch = "wasm32")]
        #[wasm_bindgen::prelude::wasm_bindgen(js_name = $javascript)]
        pub fn $export(
            $($plain: $kind,)*
            $($($crossing: tsify::Ts<$crossed>,)+)?
        ) -> Result<tsify::Ts<$mirror>, wasm_bindgen::JsError> {
            Ok(tsify::Ts::from_rust(&$function(
                $($plain,)*
                $($($crossing.to_rust()?,)+)?
            ))?)
        }
    };
}

pub(crate) use export;
