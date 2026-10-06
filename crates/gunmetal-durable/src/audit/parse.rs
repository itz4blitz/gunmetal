//! A JSON object scanner for the audit log's own canonical lines.

use std::collections::BTreeMap;

use crate::audit::error::AuditError;

/// One JSON value the log writes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Json {
    Null,
    Num(u64),
    Str(String),
    Obj(BTreeMap<String, Json>),
}

impl Json {
    pub(crate) fn str(&self) -> Option<&str> {
        match self {
            Self::Str(text) => Some(text),
            _ => None,
        }
    }

    pub(crate) fn num(&self) -> Option<u64> {
        match self {
            Self::Num(n) => Some(*n),
            _ => None,
        }
    }

    pub(crate) fn obj(&self) -> Option<&BTreeMap<String, Json>> {
        match self {
            Self::Obj(map) => Some(map),
            _ => None,
        }
    }
}

struct Cur<'a> {
    bytes: &'a [u8],
    i: usize,
}

impl Cur<'_> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.i).copied()
    }

    fn next(&mut self) -> Option<u8> {
        let b = self.peek()?;
        self.i = self.i.wrapping_add(1);
        Some(b)
    }

    fn eat(&mut self, expected: u8) -> Option<()> {
        (self.next()? == expected).then_some(())
    }
}

/// Parses one canonical object.
pub(crate) fn object(text: &str) -> Result<BTreeMap<String, Json>, AuditError> {
    let mut cur = Cur {
        bytes: text.as_bytes(),
        i: 0,
    };
    let value = parse_value(&mut cur).ok_or(AuditError::Corrupt { seq: 0 })?;
    if cur.peek().is_some() {
        return Err(AuditError::Corrupt { seq: 0 });
    }
    value.obj().cloned().ok_or(AuditError::Corrupt { seq: 0 })
}

fn parse_value(cur: &mut Cur<'_>) -> Option<Json> {
    match cur.peek()? {
        b'{' => parse_object(cur),
        b'"' => parse_string(cur).map(Json::Str),
        b'n' => parse_null(cur),
        b'0'..=b'9' => parse_num(cur),
        _ => None,
    }
}

fn parse_null(cur: &mut Cur<'_>) -> Option<Json> {
    cur.eat(b'n')?;
    cur.eat(b'u')?;
    cur.eat(b'l')?;
    cur.eat(b'l')?;
    Some(Json::Null)
}

fn parse_num(cur: &mut Cur<'_>) -> Option<Json> {
    let mut n = 0_u64;
    let mut saw = false;
    while let Some(b @ b'0'..=b'9') = cur.peek() {
        let _ = cur.next();
        saw = true;
        n = n
            .saturating_mul(10)
            .saturating_add(u64::from(b.wrapping_sub(b'0')));
    }
    saw.then_some(Json::Num(n))
}

fn parse_string(cur: &mut Cur<'_>) -> Option<String> {
    cur.eat(b'"')?;
    let mut out = String::new();
    loop {
        match cur.next()? {
            b'"' => return Some(out),
            b'\\' => match cur.next()? {
                b'"' => out.push('"'),
                b'\\' => out.push('\\'),
                b'u' => {
                    let mut code = 0_u32;
                    for _ in 0..4 {
                        let d = nibble(cur.next()?)?;
                        code = code.saturating_mul(16).saturating_add(u32::from(d));
                    }
                    out.push(char::from_u32(code)?);
                }
                _ => return None,
            },
            b => out.push(char::from(b)),
        }
    }
}

fn nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte.wrapping_sub(b'0')),
        b'a'..=b'f' => Some(byte.wrapping_sub(b'a').wrapping_add(10)),
        b'A'..=b'F' => Some(byte.wrapping_sub(b'A').wrapping_add(10)),
        _ => None,
    }
}

fn parse_object(cur: &mut Cur<'_>) -> Option<Json> {
    cur.eat(b'{')?;
    let mut map = BTreeMap::new();
    if cur.peek() == Some(b'}') {
        let _ = cur.next();
        return Some(Json::Obj(map));
    }
    loop {
        let key = parse_string(cur)?;
        cur.eat(b':')?;
        let value = parse_value(cur)?;
        map.insert(key, value);
        match cur.next()? {
            b'}' => return Some(Json::Obj(map)),
            b',' => {}
            _ => return None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Json, object};

    #[test]
    fn parses_a_canonical_object_and_refuses_junk() {
        let map = object(r#"{"seq":1,"event":"fail","nested":{"a":null}}"#).expect("object");
        assert_eq!(map.get("seq").and_then(Json::num), Some(1));
        assert_eq!(map.get("event").and_then(Json::str), Some("fail"));
        assert_eq!(
            map.get("nested")
                .and_then(Json::obj)
                .and_then(|n| n.get("a")),
            Some(&Json::Null)
        );
        assert!(object("[]").is_err());
        assert!(object("{").is_err());
        assert!(object(r#"{"a":1}trailing"#).is_err());
        assert!(object(r#"{"a":true}"#).is_err());
        assert!(object("null").is_err());
        assert!(object(r#"{"x":"\u0022"}"#).is_ok());
        assert!(object(r#"{"x":"\q"}"#).is_err());
        assert!(object("{}").is_ok());
    }
}
