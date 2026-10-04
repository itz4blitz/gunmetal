//! Closed vocabularies with stable numeric codes.
//!
//! Every enumeration the catalogue stores or syncs has a code that never
//! changes and is never reused, so a record written by one release reads
//! the same in the next, and a code an older release does not know reads
//! as `None` rather than as some other value (LAT-001).

/// Declares an enumeration with a stable code per variant.
///
/// Each entry is a documented variant and its code, so one line holds
/// everything about a value. Mutation testing does not see code a macro
/// generates, and coverage does not count its match arms one by one, so
/// each enumeration's tests pin every code against a literal table instead.
macro_rules! coded {
    (
        $(#[$meta:meta])*
        $name:ident: $repr:ty {
            $(
                $(#[$doc:meta])*
                $variant:ident = $code:literal,
            )*
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum $name {
            $( $(#[$doc])* $variant, )*
        }

        impl $name {
            /// Every value, in order of its code.
            pub const ALL: &'static [Self] = &[$(Self::$variant),*];

            /// The stable code, which never changes and is never reused.
            #[must_use]
            pub const fn code(self) -> $repr {
                match self {
                    $(Self::$variant => $code,)*
                }
            }

            /// The value whose code is `code`, or `None` for a code this
            /// release does not know.
            #[must_use]
            pub const fn from_code(code: $repr) -> Option<Self> {
                match code {
                    $($code => Some(Self::$variant),)*
                    _ => None,
                }
            }
        }
    };
}

pub(crate) use coded;
