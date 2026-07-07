use LexerErrorKind::*;
use TokenKind::*;

use unicode_segmentation::UnicodeSegmentation;

/// Represents a cursor position in source code.
///
/// Column numbers are counted by grapheme clusters following [UAX #29] rules.
/// Therefore, the character "é", despite consisting of two Unicode scalar values,
/// counts as one cluster and spans one column. If we had counted by
/// the number of `char`s that make up this character instead, we would have got
/// a column count of 2, which is incorrect. More about this in the
/// [`char` primitive type docs][doc].
///
/// [UAX #29]: https://www.unicode.org/reports/tr29/
/// [doc]: https://doc.rust-lang.org/stable/std/primitive.char.html#representation
#[derive(Debug, PartialEq)]
pub struct Position {
    /// The line number of this position.
    pub line: usize,
    /// The column number of this position.
    pub column: usize,
}

impl Position {
    fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

#[derive(Debug, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub start: Position,
    pub end: Position,
}

impl Token {
    pub fn new(kind: TokenKind, start: Position, end: Position) -> Self {
        Self { kind, start, end }
    }
}

/// Describes the kind of [`Token`] and possibly the value that it contains.
///
/// For example, an `Ident` token contains a `String` value (its name),
/// but a `Base` token has no value.
#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub enum TokenKind {
    /// An identifier.
    Ident(String),
}

#[derive(Debug, PartialEq)]
pub struct LexerError {
    kind: LexerErrorKind,
    start: Position,
    end: Position,
}

impl LexerError {
    fn new(kind: LexerErrorKind, start: Position, end: Position) -> Self {
        Self { kind, start, end }
    }
}

/// Describes the kind of [`LexerError`] and provides additional information if any.
#[derive(Debug, PartialEq)]
pub enum LexerErrorKind {
    /// One or more unexpected symbols were encountered.
    UnexpectedSymbol(String),
}

/// Creates an iterator that produces [`Token`]s wrapped behind
/// [`Result`]s from an input source string.
pub fn tokenize(source: &str) -> impl Iterator<Item = Result<Token, LexerError>> {
    let mut lexer = Lexer::new(source);
    std::iter::from_fn(move || lexer.next_token())
}

struct Lexer {
    source: Vec<u8>,
    index: usize,
    line: usize,
    column: usize,
}

impl Lexer {
    fn new(source: &str) -> Self {
        Self {
            source: source.as_bytes().to_owned(),
            index: 0,
            line: 1,
            column: 1,
        }
    }

    fn current_byte(&self) -> Option<u8> {
        self.source.get(self.index).copied()
    }

    fn peek_byte(&self) -> Option<u8> {
        self.source.get(self.index + 1).copied()
    }

    fn next_byte(&mut self, count_columns: bool) -> Option<u8> {
        if count_columns {
            self.column += 1;
        }
        self.index += 1;
        self.current_byte()
    }

    fn advance_line(&mut self) {
        self.index += 1;
        self.line += 1;
        self.column = 1;
    }

    fn string_from(&self, start_index: usize) -> String {
        // You should be banned from scripting if you somehow wrote bogus bytes
        String::from_utf8_lossy(
            self.source
                .get(start_index..self.index)
                .expect("range should be in bounds of source vector"),
        )
        .into_owned()
    }

    fn advance_bytes_until_newline_or_eof(&mut self) {
        while let Some(byte) = self.next_byte(false) {
            if byte == b'\n' {
                break;
            }
        }
    }

    fn error_on_line(
        &self,
        kind: LexerErrorKind,
        start_column: usize,
    ) -> Option<Result<Token, LexerError>> {
        Some(Err(LexerError::new(
            kind,
            Position::new(self.line, start_column),
            Position::new(self.line, self.column - 1),
        )))
    }

    fn next_token(&mut self) -> Option<Result<Token, LexerError>> {
        match self.current_byte()? {
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => self.idents_and_keywords(),

            // Unexpected byte. Advance until the next lexable byte and
            // report the offender to the user.
            _ => self.unexpected(),
        }
    }

    fn idents_and_keywords(&mut self) -> Option<Result<Token, LexerError>> {
        todo!()
    }

    fn unexpected(&mut self) -> Option<Result<Token, LexerError>> {
        let start_index = self.index;
        let start_column = self.column;

        while let Some(byte) = self.next_byte(false) {
            // TODO: figure out the boundary for lexable bytes
            if byte < 128 {
                break;
            }
        }

        let offender = self.string_from(start_index);
        let columns = offender.graphemes(true).count();
        self.column += columns;

        self.error_on_line(UnexpectedSymbol(offender), start_column)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_stream_eq {
        (
            $source: expr,
            $(
                $token: expr
            ),+
        ) => {{
            let vec_source: Vec<_> = tokenize($source).collect();
            assert_eq!(vec_source, vec![$($token,)+]);
        }};
    }

    fn token(
        kind: TokenKind,
        start: (usize, usize),
        end: (usize, usize),
    ) -> Result<Token, LexerError> {
        Ok(Token::new(
            kind,
            Position::new(start.0, start.1),
            Position::new(end.0, end.1),
        ))
    }

    fn error(
        kind: LexerErrorKind,
        start: (usize, usize),
        end: (usize, usize),
    ) -> Result<Token, LexerError> {
        Err(LexerError::new(
            kind,
            Position::new(start.0, start.1),
            Position::new(end.0, end.1),
        ))
    }

    #[test]
    fn empty() {
        let mut tokens = tokenize("");
        assert_eq!(tokens.next(), None);
        assert_eq!(tokens.next(), None);
    }

    #[test]
    fn unexpected_symbol() {
        assert_stream_eq!("ä", error(UnexpectedSymbol("ä".into()), (1, 1), (1, 1)));
        assert_stream_eq!("äöü", error(UnexpectedSymbol("äöü".into()), (1, 1), (1, 3)));
    }
}
