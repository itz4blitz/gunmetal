//! Reads the columns of a table from the `CREATE TABLE` text SQLite keeps
//! in `sqlite_schema`, for the data-class check (SEC-TM-050).
//!
//! The one SQLite door in `gunmetal-fs` refuses any pragma given a value,
//! `table_info` among them, so the store cannot ask SQLite for a table's
//! columns. SQLite does keep the text that created each table, with
//! `ALTER TABLE` edits applied to it, so the store reads the columns from
//! that text instead.
//!
//! Only the shape SQLite stores for an ordinary table is understood:
//! `CREATE TABLE`, the table's name, then the parenthesised list of column
//! definitions and table constraints. (For a table created from a `SELECT`,
//! SQLite stores a statement of that shape that it writes itself.) Anything
//! else, such as a virtual table, is not understood, and the store treats
//! the schema as one it cannot check.

/// One token of the text.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Token<'a> {
    /// `(`
    Open,
    /// `)`
    Close,
    /// `,`
    Comma,
    /// A bare word: letters, digits, `_`, `$` and any non-ASCII character.
    Word(&'a str),
    /// A quoted name or string, with its quotes removed.
    Quoted(String),
    /// Any other character.
    Other,
}

/// The words that start a table constraint rather than a column.
const CONSTRAINTS: [&str; 5] = ["CHECK", "CONSTRAINT", "FOREIGN", "PRIMARY", "UNIQUE"];

/// The columns `sql` defines, in order, or `None` when `sql` is not a
/// `CREATE TABLE` statement with a column list.
pub(crate) fn column_names(sql: &str) -> Option<Vec<String>> {
    let tokens = tokenize(sql)?;
    let rest = match tokens.as_slice() {
        [
            Token::Word(create),
            Token::Word(table),
            name,
            Token::Open,
            rest @ ..,
        ] if create.eq_ignore_ascii_case("CREATE")
            && table.eq_ignore_ascii_case("TABLE")
            && matches!(name, Token::Word(_) | Token::Quoted(_)) =>
        {
            rest
        }
        _ => return None,
    };
    let mut names = Vec::new();
    let mut depth = 0_usize;
    let mut first = true;
    for token in rest {
        if first {
            first = false;
            match token {
                Token::Word(word) if is_constraint(word) => {}
                Token::Word(word) => names.push((*word).to_owned()),
                Token::Quoted(name) => names.push(name.clone()),
                _ => return None,
            }
            continue;
        }
        match (token, depth) {
            (Token::Close, 0) => return Some(names),
            (Token::Comma, 0) => first = true,
            (Token::Open, _) => depth += 1,
            (Token::Close, _) => depth -= 1,
            _ => {}
        }
    }
    None
}

fn is_constraint(word: &str) -> bool {
    CONSTRAINTS
        .iter()
        .any(|constraint| constraint.eq_ignore_ascii_case(word))
}

/// Splits `sql` into tokens, leaving out white space and comments, or
/// returns `None` when a quote or a block comment is not closed.
fn tokenize(sql: &str) -> Option<Vec<Token<'_>>> {
    let mut tokens = Vec::new();
    let mut rest = sql;
    while let Some(next) = rest.chars().next() {
        let after = &rest[next.len_utf8()..];
        rest = match next {
            '(' | ')' | ',' => {
                tokens.push(match next {
                    '(' => Token::Open,
                    ')' => Token::Close,
                    _ => Token::Comma,
                });
                after
            }
            '-' if after.starts_with('-') => after.find('\n').map_or("", |end| &after[end..]),
            // The comment's text starts after the `*`, so `/*/` is not closed.
            '/' if after.starts_with('*') => &after[after[1..].find("*/")? + 3..],
            '"' | '\'' | '`' => {
                let (name, after) = unquote(after, next)?;
                tokens.push(Token::Quoted(name));
                after
            }
            '[' => {
                let end = after.find(']')?;
                tokens.push(Token::Quoted(after[..end].to_owned()));
                &after[end + 1..]
            }
            _ if is_word(next) => {
                // Measured from after the first character, so the loop
                // moves on whatever the rest holds.
                let end = after.find(|c| !is_word(c)).unwrap_or(after.len());
                tokens.push(Token::Word(&rest[..next.len_utf8() + end]));
                &after[end..]
            }
            _ if next.is_ascii_whitespace() => after,
            _ => {
                tokens.push(Token::Other);
                after
            }
        };
    }
    Some(tokens)
}

/// Whether `c` may be part of a bare word.
fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '$' || !c.is_ascii()
}

/// Reads a quoted text that started just before `text`, up to the closing
/// `quote`, where a doubled `quote` stands for one. Returns the text and
/// what follows the closing quote.
fn unquote(text: &str, quote: char) -> Option<(String, &str)> {
    let mut name = String::new();
    let mut rest = text;
    loop {
        let end = rest.find(quote)?;
        name.push_str(&rest[..end]);
        rest = &rest[end + 1..];
        if rest.starts_with(quote) {
            name.push(quote);
            rest = &rest[1..];
        } else {
            return Some((name, rest));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(sql: &str) -> Option<Vec<String>> {
        column_names(sql)
    }

    fn owned(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    #[test]
    fn reads_the_columns_of_a_plain_table() {
        assert_eq!(
            names("CREATE TABLE tracks (id INTEGER PRIMARY KEY, title TEXT NOT NULL) STRICT"),
            Some(owned(&["id", "title"]))
        );
    }

    #[test]
    fn reads_columns_with_no_type_and_no_space_before_the_list() {
        assert_eq!(names("create table t(a,b)"), Some(owned(&["a", "b"])));
    }

    #[test]
    fn reads_quoted_names_with_their_quotes_doubled_inside() {
        assert_eq!(
            names(r#"CREATE TABLE "my ""t""" ("a b", 'c''d', `e``f`, [g "h"], "", x)"#),
            Some(owned(&["a b", "c'd", "e`f", "g \"h\"", "", "x"]))
        );
    }

    #[test]
    fn skips_table_constraints_in_any_case() {
        assert_eq!(
            names(
                "CREATE TABLE t (a INT, b INT, Constraint pk Primary Key (a, b), \
                 unique (b), check (a > 0), FOREIGN KEY (b) REFERENCES u (id), c INT)"
            ),
            Some(owned(&["a", "b", "c"]))
        );
    }

    #[test]
    fn ignores_commas_and_parentheses_nested_in_a_definition() {
        assert_eq!(
            names(
                "CREATE TABLE t (a DECIMAL(10, 2) DEFAULT (abs(-1)), \
                 b TEXT CHECK (b IN ('x,y', ')')), c AS (a * (2 + 1)))"
            ),
            Some(owned(&["a", "b", "c"]))
        );
    }

    #[test]
    fn ignores_comments_and_white_space_of_every_kind() {
        assert_eq!(
            names(
                "CREATE TABLE t ( -- the first, (column)\n\ta INT /* a, ( */,\r\n\
                 /**/b-- last\n)"
            ),
            Some(owned(&["a", "b"]))
        );
    }

    #[test]
    fn reads_words_with_digits_dollars_underscores_and_non_ascii() {
        assert_eq!(
            names("CREATE TABLE t (a1, $b, _c, état, d-e)"),
            Some(owned(&["a1", "$b", "_c", "état", "d"]))
        );
    }

    #[test]
    fn ignores_what_follows_the_column_list() {
        assert_eq!(
            names("CREATE TABLE t (a) WITHOUT ROWID, STRICT -- (b)"),
            Some(owned(&["a"]))
        );
        assert_eq!(names("CREATE TABLE t (a) ) , ("), Some(owned(&["a"])));
    }

    #[test]
    fn refuses_what_is_not_a_table_with_a_column_list() {
        for sql in [
            "",
            "CREATE TABLE",
            "CREATE TABLE t",
            "CREATE TABLE t AS SELECT a FROM u",
            "CREATE VIRTUAL TABLE t USING fts5(a)",
            "CREATE INDEX i ON t (a)",
            "CREATE VIEW v (a) AS SELECT 1",
            "CREATE TABLE (a)",
            "CREATE TABLE , (a)",
            "CREATE TABLE t , (a)",
            "TABLE CREATE t (a)",
            "CREATE TABLE t (a",
            "CREATE TABLE t (a, b (",
            "CREATE TABLE t ()",
            "CREATE TABLE t (a,)",
            "CREATE TABLE t (, a)",
            "CREATE TABLE t ((a))",
            "CREATE TABLE t (+a)",
            "CREATE TABLE t (a, b) STRICT 'unclosed",
            "CREATE TABLE t (\"a)",
            "CREATE TABLE t ('a)",
            "CREATE TABLE t (`a)",
            "CREATE TABLE t ([a)",
            "CREATE TABLE t (a /* b)",
            "CREATE TABLE t (a) /*",
        ] {
            assert_eq!(names(sql), None);
        }
    }

    #[test]
    fn reads_a_table_whose_only_entries_are_constraints() {
        assert_eq!(names("CREATE TABLE t (CHECK (1))"), Some(owned(&[])));
    }

    #[test]
    fn tokenizes_every_kind_of_token() {
        assert_eq!(
            tokenize("( ) , w1 'q' + ;"),
            Some(vec![
                Token::Open,
                Token::Close,
                Token::Comma,
                Token::Word("w1"),
                Token::Quoted("q".to_owned()),
                Token::Other,
                Token::Other,
            ])
        );
    }

    #[test]
    fn a_lone_dash_or_slash_is_another_character() {
        assert_eq!(
            tokenize("-a/b-"),
            Some(vec![
                Token::Other,
                Token::Word("a"),
                Token::Other,
                Token::Word("b"),
                Token::Other,
            ])
        );
        assert_eq!(tokenize("-- to the end"), Some(vec![]));
        assert_eq!(
            tokenize("a/**/b/* * / */"),
            Some(vec![Token::Word("a"), Token::Word("b")])
        );
        assert_eq!(tokenize("/*/"), None);
        assert_eq!(
            tokenize("[a]b'c'd"),
            Some(vec![
                Token::Quoted("a".to_owned()),
                Token::Word("b"),
                Token::Quoted("c".to_owned()),
                Token::Word("d"),
            ])
        );
    }
}
