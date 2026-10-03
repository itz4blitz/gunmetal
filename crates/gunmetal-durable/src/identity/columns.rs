//! The column names of a table, read from the `CREATE TABLE` text SQLite
//! keeps in `sqlite_schema`.
//!
//! The data-class check at open compares the columns the live file has with
//! the classes the schema parts declare (SEC-TM-050). SQLite reports a
//! table's columns through `pragma_table_info(name)`, but the connection
//! opener (WP-126) refuses every pragma given a value once it has locked a
//! connection down, so the store reads the statement text instead. The
//! reader understands what a schema part writes: column definitions and
//! table constraints separated by commas, quoted names, string literals and
//! comments. Anything else, including text that ends inside a quote, a
//! comment or the column list, gets `None`, and the store refuses the file
//! rather than guess.

/// One piece of a statement.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    /// A bare name, keyword or number.
    Word(String),
    /// A name in double quotes, backquotes or square brackets.
    Quoted(String),
    /// A string literal; its text does not matter here.
    Literal,
    /// Any other character, such as a parenthesis or a comma.
    Mark(char),
}

/// The words that start a table constraint rather than a column definition.
const CONSTRAINTS: [&str; 5] = ["CONSTRAINT", "PRIMARY", "UNIQUE", "CHECK", "FOREIGN"];

/// The names of the columns `sql`, a `CREATE TABLE` statement, defines, in
/// order, or `None` when the text is not one this reader understands.
pub(crate) fn columns(sql: &str) -> Option<Vec<String>> {
    let tokens = tokenize(sql)?;
    let start = tokens.iter().position(|token| *token == Token::Mark('('))?;
    let mut names = Vec::new();
    let mut definition: Vec<&Token> = Vec::new();
    // Inside the list, which ends when this falls back to zero. It is at
    // least one until then, so neither step can overflow.
    let mut depth = 1_usize;
    for token in tokens.iter().skip(start + 1) {
        match token {
            Token::Mark('(') => depth += 1,
            Token::Mark(')') => depth -= 1,
            _ => {}
        }
        if depth == 0 || (depth == 1 && *token == Token::Mark(',')) {
            if let Definition::Column(name) = definition_of(&definition)? {
                names.push(name);
            }
            definition.clear();
            if depth == 0 {
                return Some(names);
            }
        } else {
            definition.push(token);
        }
    }
    None
}

/// What one comma-separated piece of the list defines.
enum Definition {
    /// A column, with its name.
    Column(String),
    /// A table constraint.
    Constraint,
}

/// What `definition` defines, or `None` when it is empty or starts with
/// neither a name nor a constraint keyword.
fn definition_of(definition: &[&Token]) -> Option<Definition> {
    match definition.first()? {
        Token::Word(word) if CONSTRAINTS.iter().any(|kw| kw.eq_ignore_ascii_case(word)) => {
            Some(Definition::Constraint)
        }
        Token::Word(name) | Token::Quoted(name) => Some(Definition::Column(name.clone())),
        Token::Literal | Token::Mark(_) => None,
    }
}

/// Splits `sql` into tokens, dropping white space and comments, or `None`
/// when it ends inside a quote or a block comment.
fn tokenize(sql: &str) -> Option<Vec<Token>> {
    let mut chars = sql.chars().peekable();
    let mut tokens = Vec::new();
    while let Some(c) = chars.next() {
        match c {
            '-' if chars.peek() == Some(&'-') => {
                // A line comment runs to the end of the line or the text.
                for skipped in chars.by_ref() {
                    if skipped == '\n' {
                        break;
                    }
                }
            }
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                let mut previous = '/';
                loop {
                    let next = chars.next()?;
                    if previous == '*' && next == '/' {
                        break;
                    }
                    previous = next;
                }
            }
            '\'' => {
                quoted(&mut chars, '\'')?;
                tokens.push(Token::Literal);
            }
            '"' | '`' => tokens.push(Token::Quoted(quoted(&mut chars, c)?)),
            '[' => tokens.push(Token::Quoted(bracketed(&mut chars)?)),
            c if is_word(c) => {
                let mut word = String::from(c);
                while let Some(&next) = chars.peek().filter(|&&next| is_word(next)) {
                    word.push(next);
                    chars.next();
                }
                tokens.push(Token::Word(word));
            }
            c if c.is_whitespace() => {}
            c => tokens.push(Token::Mark(c)),
        }
    }
    Some(tokens)
}

/// Whether `c` can be part of a bare name.
fn is_word(c: char) -> bool {
    c == '_' || c == '$' || c.is_alphanumeric()
}

/// Reads up to the closing `close`, where a doubled `close` stands for one,
/// and returns the text between.
fn quoted(chars: &mut std::iter::Peekable<std::str::Chars<'_>>, close: char) -> Option<String> {
    let mut text = String::new();
    loop {
        let c = chars.next()?;
        if c != close {
            text.push(c);
        } else if chars.next_if_eq(&close).is_some() {
            text.push(close);
        } else {
            return Some(text);
        }
    }
}

/// Reads up to the closing `]`, which cannot be escaped.
fn bracketed(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> Option<String> {
    let mut text = String::new();
    loop {
        match chars.next()? {
            ']' => return Some(text),
            c => text.push(c),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|name| (*name).to_owned()).collect()
    }

    #[test]
    fn reads_the_columns_of_a_plain_table() {
        assert_eq!(
            columns("CREATE TABLE t (a INTEGER PRIMARY KEY, b TEXT NOT NULL, c)"),
            Some(names(&["a", "b", "c"]))
        );
    }

    #[test]
    fn skips_table_constraints_in_any_case() {
        assert_eq!(
            columns(
                "CREATE TABLE t (a, b, PRIMARY KEY (a, b), unique (b), Check (a > 0), \
                 constraint k FOREIGN KEY (a) REFERENCES u (x), foreign key (b) REFERENCES u)"
            ),
            Some(names(&["a", "b"]))
        );
    }

    #[test]
    fn ignores_commas_and_parentheses_inside_a_definition() {
        assert_eq!(
            columns(
                "CREATE TABLE t (a DECIMAL(10, 2) CHECK (a IN (1, 2)), \
                 b TEXT DEFAULT 'x, (y)', c INTEGER CHECK ((c))) WITHOUT ROWID"
            ),
            Some(names(&["a", "b", "c"]))
        );
    }

    #[test]
    fn reads_quoted_names_with_their_escapes() {
        assert_eq!(
            columns(r#"CREATE TABLE "t" ("a b", `c``d`, [e "f"], "g""h", 'i')"#),
            None
        );
        assert_eq!(
            columns(r#"CREATE TABLE "t" ("a b", `c``d`, [e "f"], "g""h", ab$_9é)"#),
            Some(names(&["a b", "c`d", "e \"f\"", "g\"h", "ab$_9é"]))
        );
    }

    #[test]
    fn skips_comments() {
        assert_eq!(
            columns(
                "CREATE TABLE t ( -- the key, (unbalanced\n a, /* b, ( */ c /**/,\
                 d /* * / */ -- trailing"
            ),
            None
        );
        assert_eq!(
            columns(
                "CREATE TABLE t ( -- the key, (unbalanced\n a, /* b, ( */ c /**/,\
                 d /* * / */) -- trailing"
            ),
            Some(names(&["a", "c", "d"]))
        );
        assert_eq!(
            columns("CREATE TABLE t (a /*/ b, */, c - 1, e/f)"),
            Some(names(&["a", "c", "e"]))
        );
    }

    #[test]
    fn stops_at_the_parenthesis_that_closes_the_list() {
        assert_eq!(
            columns("CREATE TABLE t (a) STRICT, (b)"),
            Some(names(&["a"]))
        );
        assert_eq!(columns("CREATE TABLE t(a)"), Some(names(&["a"])));
        assert_eq!(columns("CREATE TABLE \"(\" (a)"), Some(names(&["a"])));
    }

    #[test]
    fn refuses_text_it_does_not_understand() {
        for sql in [
            "",
            "CREATE TABLE t AS SELECT 1",
            "CREATE TABLE t (a",
            "CREATE TABLE t (a, (b))",
            "CREATE TABLE t (a,, b)",
            "CREATE TABLE t (, a)",
            "CREATE TABLE t (a,)",
            "CREATE TABLE t ()",
            "CREATE TABLE t (a 'unterminated)",
            "CREATE TABLE t (\"a)",
            "CREATE TABLE t (`a)",
            "CREATE TABLE t ([a)",
            "CREATE TABLE t (a /* unterminated)",
            "CREATE TABLE t (a) /*",
            "CREATE TABLE t (a) 'x",
        ] {
            assert_eq!(columns(sql), None);
        }
    }

    #[test]
    fn tokenizes_marks_words_and_literals() {
        assert_eq!(
            tokenize("a.b 'it''s' , \"q\"\"x\" [r]-1"),
            Some(vec![
                Token::Word("a".to_owned()),
                Token::Mark('.'),
                Token::Word("b".to_owned()),
                Token::Literal,
                Token::Mark(','),
                Token::Quoted("q\"x".to_owned()),
                Token::Quoted("r".to_owned()),
                Token::Mark('-'),
                Token::Word("1".to_owned()),
            ])
        );
        assert_eq!(
            tokenize("a -- b\nc"),
            Some(vec![
                Token::Word("a".to_owned()),
                Token::Word("c".to_owned()),
            ])
        );
        assert_eq!(tokenize("/"), Some(vec![Token::Mark('/')]));
        assert_eq!(tokenize("/*"), None);
        assert_eq!(tokenize("/**/"), Some(vec![]));
    }

    /// A column name the generator may write bare: a letter, then letters,
    /// digits and underscores, and not a constraint keyword.
    fn bare_name() -> impl Strategy<Value = String> {
        "[a-z][a-z0-9_]{0,8}".prop_filter("not a constraint keyword", |name| {
            !CONSTRAINTS.iter().any(|kw| kw.eq_ignore_ascii_case(name))
        })
    }

    /// The text after a column's name: a type, constraints with nested
    /// parentheses and commas, or nothing.
    fn tail() -> impl Strategy<Value = &'static str> {
        prop::sample::select(vec![
            "",
            " INTEGER",
            " TEXT NOT NULL DEFAULT 'a,b)'",
            " NUMERIC(4, 2)",
            " CHECK (x IN (1, (2)))",
            " /* , ) */ BLOB",
            " -- , )\n",
        ])
    }

    proptest! {
        #[test]
        fn reads_back_every_column_it_is_given(
            columns_in in prop::collection::vec((bare_name(), tail(), any::<bool>()), 1..6),
            constraint in any::<bool>(),
        ) {
            let mut definitions: Vec<String> = columns_in
                .iter()
                .map(|(name, tail, quote)| {
                    if *quote {
                        format!("\"{}\"{tail}", name.replace('"', "\"\""))
                    } else {
                        format!("{name}{tail}")
                    }
                })
                .collect();
            if constraint {
                definitions.push("PRIMARY KEY (a, b)".to_owned());
            }
            let sql = format!("CREATE TABLE t ({})", definitions.join(", "));
            let expected: Vec<String> = columns_in.into_iter().map(|(name, _, _)| name).collect();
            prop_assert_eq!(columns(&sql), Some(expected));
        }

        #[test]
        fn returns_for_any_text(sql in "(?s).{0,64}") {
            let _ = columns(&sql);
        }
    }
}
