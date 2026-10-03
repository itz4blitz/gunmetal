//! The little of JSON the checks read: GitHub API responses, parsed whole
//! into values. A number is kept as written, checked only for the
//! characters a number can hold, because no check reads one.

use std::iter::Peekable;
use std::str::Chars;

/// The deepest nesting of arrays and objects a document may have.
pub const MAX_DEPTH: usize = 64;

/// A JSON value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    /// `null`.
    Null,
    /// `true` or `false`.
    Bool(bool),
    /// A number, as written.
    Number(String),
    /// A string, unescaped.
    String(String),
    /// An array.
    Array(Vec<Value>),
    /// An object's members, in document order.
    Object(Vec<(String, Value)>),
}

impl Value {
    /// The value of this object's first member named `key`.
    pub fn get(&self, key: &str) -> Option<&Value> {
        let Self::Object(members) = self else {
            return None;
        };
        members
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value)
    }

    /// This string's text.
    pub fn as_str(&self) -> Option<&str> {
        let Self::String(text) = self else {
            return None;
        };
        Some(text)
    }

    /// This array's items.
    pub fn as_array(&self) -> Option<&[Value]> {
        let Self::Array(items) = self else {
            return None;
        };
        Some(items)
    }
}

/// The value `text` holds, or `None` when it is not one JSON value.
pub fn parse(text: &str) -> Option<Value> {
    let mut parser = Parser {
        chars: text.chars().peekable(),
    };
    let value = parser.value(0)?;
    parser.skip_space();
    parser.chars.peek().is_none().then_some(value)
}

/// What is left of a document.
struct Parser<'a> {
    /// The characters not read yet.
    chars: Peekable<Chars<'a>>,
}

impl Parser<'_> {
    /// Skips JSON whitespace.
    fn skip_space(&mut self) {
        while self
            .chars
            .next_if(|&character| matches!(character, ' ' | '\t' | '\n' | '\r'))
            .is_some()
        {}
    }

    /// Reads `expected`, or nothing when it does not come next.
    fn eat(&mut self, expected: char) -> Option<()> {
        self.chars.next_if_eq(&expected).map(drop)
    }

    /// Reads one value inside `depth` enclosing arrays and objects.
    fn value(&mut self, depth: usize) -> Option<Value> {
        self.skip_space();
        match self.chars.next()? {
            'n' => self.word("ull").map(|()| Value::Null),
            't' => self.word("rue").map(|()| Value::Bool(true)),
            'f' => self.word("alse").map(|()| Value::Bool(false)),
            '"' => self.string().map(Value::String),
            '[' | '{' if depth >= MAX_DEPTH => None,
            '[' => self.array(depth + 1).map(Value::Array),
            '{' => self.object(depth + 1).map(Value::Object),
            first @ ('-' | '0'..='9') => Some(Value::Number(self.number(first))),
            _ => None,
        }
    }

    /// Reads the rest of a literal.
    fn word(&mut self, rest: &str) -> Option<()> {
        rest.chars().try_for_each(|character| self.eat(character))
    }

    /// Reads the rest of a number that starts with `first`.
    fn number(&mut self, first: char) -> String {
        let mut text = String::from(first);
        while let Some(character) = self.chars.next_if(|&character| {
            character.is_ascii_digit() || matches!(character, '.' | 'e' | 'E' | '+' | '-')
        }) {
            text.push(character);
        }
        text
    }

    /// Reads the rest of a string, after its opening quote.
    fn string(&mut self) -> Option<String> {
        let mut text = String::new();
        loop {
            match self.chars.next()? {
                '"' => return Some(text),
                '\\' => text.push(self.escape()?),
                character if character < ' ' => return None,
                character => text.push(character),
            }
        }
    }

    /// Reads the rest of an escape, after its backslash. A `\u` escape of
    /// half a surrogate pair reads as U+FFFD: no check needs the character.
    fn escape(&mut self) -> Option<char> {
        Some(match self.chars.next()? {
            '"' => '"',
            '\\' => '\\',
            '/' => '/',
            'b' => '\u{8}',
            'f' => '\u{c}',
            'n' => '\n',
            'r' => '\r',
            't' => '\t',
            'u' => {
                let mut code = 0;
                for _ in 0..4 {
                    code = code * 16 + self.chars.next()?.to_digit(16)?;
                }
                char::from_u32(code).unwrap_or(char::REPLACEMENT_CHARACTER)
            }
            _ => return None,
        })
    }

    /// Reads the rest of an array, after its `[`, inside `depth` arrays and
    /// objects.
    fn array(&mut self, depth: usize) -> Option<Vec<Value>> {
        let mut items = Vec::new();
        self.skip_space();
        if self.eat(']').is_some() {
            return Some(items);
        }
        loop {
            items.push(self.value(depth)?);
            self.skip_space();
            match self.chars.next()? {
                ',' => {}
                ']' => return Some(items),
                _ => return None,
            }
        }
    }

    /// Reads the rest of an object, after its `{`, inside `depth` arrays and
    /// objects.
    fn object(&mut self, depth: usize) -> Option<Vec<(String, Value)>> {
        let mut members = Vec::new();
        self.skip_space();
        if self.eat('}').is_some() {
            return Some(members);
        }
        loop {
            self.skip_space();
            self.eat('"')?;
            let name = self.string()?;
            self.skip_space();
            self.eat(':')?;
            members.push((name, self.value(depth)?));
            self.skip_space();
            match self.chars.next()? {
                ',' => {}
                '}' => return Some(members),
                _ => return None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{MAX_DEPTH, Value, parse};

    /// A string value.
    fn string(text: &str) -> Value {
        Value::String(text.to_owned())
    }

    #[test]
    fn parses_every_kind_of_value() {
        let text = " {\"none\": null, \"yes\" :true,\"no\":false,\n\t\"n\": -12.5e+3,\r\n\"s\": \"a b\",\"list\": [1,[ ] , {}],\"o\":{\"k\":\"v\"}} ";
        assert_eq!(
            parse(text),
            Some(Value::Object(vec![
                ("none".to_owned(), Value::Null),
                ("yes".to_owned(), Value::Bool(true)),
                ("no".to_owned(), Value::Bool(false)),
                ("n".to_owned(), Value::Number("-12.5e+3".to_owned())),
                ("s".to_owned(), string("a b")),
                (
                    "list".to_owned(),
                    Value::Array(vec![
                        Value::Number("1".to_owned()),
                        Value::Array(vec![]),
                        Value::Object(vec![]),
                    ])
                ),
                (
                    "o".to_owned(),
                    Value::Object(vec![("k".to_owned(), string("v"))])
                ),
            ]))
        );
        assert_eq!(
            parse("[0,-2]"),
            Some(Value::Array(vec![
                Value::Number("0".to_owned()),
                Value::Number("-2".to_owned()),
            ]))
        );
    }

    #[test]
    fn unescapes_strings() {
        assert_eq!(
            parse(r#""\"\\\/\b\f\n\r\t\u0041\u00e9\uABCD\ud83d end""#),
            Some(string("\"\\/\u{8}\u{c}\n\r\tA\u{e9}\u{abcd}\u{fffd} end"))
        );
        assert_eq!(parse("\"\u{e9}t\u{e9}\""), Some(string("\u{e9}t\u{e9}")));
    }

    #[test]
    fn refuses_anything_that_is_not_one_value() {
        for text in [
            "",
            " ",
            "nul",
            "nulL",
            "tru",
            "fals",
            "@",
            "[",
            "[1",
            "[1,]",
            "[1 2]",
            "[1}",
            "{",
            "{a: 1}",
            "{\"a",
            "{\"a\" 1}",
            "{\"a\":",
            "{\"a\":1",
            "{\"a\":1,}",
            "{\"a\":1 \"b\":2}",
            "{\"a\":1]",
            "\"unterminated",
            "\"bad \\x escape\"",
            "\"\\u12G4\"",
            "\"\\u12\"",
            "\"\\u12",
            "\"ends in a backslash\\",
            "\"raw\ttab\"",
            "\"raw\u{1f}control\"",
            "[1] 2",
            "null null",
        ] {
            assert_eq!(parse(text), None, "{text:?}");
        }
    }

    #[test]
    fn refuses_nesting_deeper_than_the_limit() {
        let arrays = |depth: usize| format!("{}{}", "[".repeat(depth), "]".repeat(depth));
        let objects =
            |depth: usize| format!("{}null{}", "{\"k\":".repeat(depth), "}".repeat(depth));
        let mut deepest = Value::Array(vec![]);
        for _ in 1..MAX_DEPTH {
            deepest = Value::Array(vec![deepest]);
        }
        assert_eq!(parse(&arrays(MAX_DEPTH)), Some(deepest));
        assert_eq!(parse(&arrays(MAX_DEPTH + 1)), None);
        assert!(parse(&objects(MAX_DEPTH)).is_some(), "{MAX_DEPTH} objects");
        assert_eq!(parse(&objects(MAX_DEPTH + 1)), None);
        assert_eq!(MAX_DEPTH, 64);
    }

    #[test]
    fn reads_members_strings_and_arrays() {
        let value = parse("{\"a\":\"first\",\"b\":[null],\"a\":\"second\"}");
        let value = value.as_ref();
        let member = |key: &str| value.and_then(|value| value.get(key));
        assert_eq!(member("a").and_then(Value::as_str), Some("first"));
        assert_eq!(
            member("b").and_then(Value::as_array),
            Some(&[Value::Null][..])
        );
        assert_eq!(member("b").and_then(Value::as_str), None);
        assert_eq!(member("a").and_then(Value::as_array), None);
        assert_eq!(member("missing"), None);
        assert_eq!(string("not an object").get("a"), None);
    }
}
