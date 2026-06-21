use derive_more::Constructor;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug, PartialEq)]
pub enum TokenKind {
    Ident(String),
    Lit(String),
    Comment(String), // Includes both "//" and "#" comments
    MultiLineComment(String),
    Whitespace(String),
    Newline,

    // Keywords
    Base,
    Break,
    Case,
    Catch,
    Class,
    Clone,
    Const,
    Constructor,
    Continue,
    Default,
    Delete,
    Else,
    Enum,
    Extends,
    False,
    File,
    For,
    Foreach,
    Function,
    If,
    In,
    Instanceof,
    Line,
    Local,
    Null,
    Rawcall,
    Resume,
    Return,
    Static,
    Switch,
    This,
    Throw,
    True,
    Try,
    Typeof,
    While,
    Yield,

    // Symbols, separated for readability
    Plus,
    PlusEq,
    Inc,
    Minus,
    MinusEq,
    Dec,
    Mult,
    MultEq,
    Div,
    DivEq,
    Mod,
    ModEq,

    BitAnd,
    BitOr,
    BitXor,
    BitNot,

    And,
    Or,
    Not,

    BitLeft,
    BitRight,
    BitUnsRight,

    Lt,
    Le,
    Gt,
    Ge,
    EqEq,
    Neq,
    Spaceship,

    Eq,
    Ins,
    Comma,
    Question,

    ParenOpen,
    ParenClose,
    SquareOpen,
    SquareClose,
    BraceOpen,
    BraceClose,
    Dot,
    Ellipsis,
    Colon,
    Semicolon,
    Scope,
    At,
}

#[derive(Constructor, Debug, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub start_line: u32,
    pub start_column: u32,
    pub end_line: u32,
    pub end_column: u32,
}

/// An iterator which returns a stream of tokens from a source string.
///
/// Normally, an iterator should only return `None` when the iteration has finished. This is not the
/// case with the token stream, as after a `LexerError` pops up, the iteration terminates. There are
/// two reasons for that:
///
/// 1. We want the formatter to ONLY work with code that parses. Code that doesn't lex will not
///    parse. Thus, there is little point in returning any more tokens once an error has appeared.
/// 2. If we choose to terminate early, we don't have to account for info recovery after an error,
///    e.g. column counts.
pub struct Lexer {
    source: Vec<u8>,
    index: usize,
    line: u32,
    column: u32,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
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

    fn next_byte(&mut self, columns: bool) -> Option<u8> {
        if columns {
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

    // Here, `char` signals that the callee is lexing a character code literal, which additional
    // value boundaries on \u and \U apply. We also do not expect unicode characters in that
    // literal, so we can leverage on next_byte()'s auto column counting. Thus, the `char` bool also
    // functions as the `columns` bool to next_byte().
    fn advance_escape_sequence(&mut self, char: bool) -> Result<(), LexerErrorKind> {
        match self.next_byte(char) {
            Some(b'U' | b'u' | b'x') => {
                let start_index = self.index + 1;
                let max = match self.current_byte() {
                    Some(b'U') => 8,
                    Some(b'u') => 4,
                    Some(b'x') => 2,
                    _ => unreachable!(),
                };
                self.advance_hex_bytes(char, max);

                if start_index == self.index {
                    if self.current_byte().is_none() {
                        self.column -= 1;
                    }
                    return Err(LexerErrorKind::InvalidHexEscape);
                }

                // \x in a character code literal has no value boundaries, and it is the only
                // sequence with a max digit of 2. For the column count, see comment below.
                if char && max != 2 && self.value_from_hex(start_index)? > 127 {
                    self.column -= 1;
                    return Err(LexerErrorKind::CharOob);
                }

                // advance_hex_bytes() places the index one over the last hex digit, which will get
                // skipped over by next_byte() when we return to the main loop. We don't want that
                // to happen...
                self.index -= 1;
                if char {
                    self.column -= 1;
                }
                Ok(())
            }

            Some(b't' | b'a' | b'b' | b'n' | b'r' | b'v' | b'f' | b'\\' | b'"' | b'\'' | b'0') => {
                // ...this is also why we don't call next_byte() here
                Ok(())
            }

            Some(_) => Err(LexerErrorKind::InvalidEscape),
            None => Err(LexerErrorKind::UnexpectedEof),
        }
    }

    fn advance_hex_bytes(&mut self, columns: bool, max: u32) {
        for _ in 0..=max {
            match self.next_byte(columns) {
                Some(b'a'..=b'f' | b'A'..=b'F' | b'0'..=b'9') => {}
                _ => break,
            }
        }
    }

    fn value_from_hex(&self, start_index: usize) -> Result<u32, LexerErrorKind> {
        let src = str::from_utf8(
            self.source
                .get(start_index..self.index)
                .expect("range should be in bounds of source vector"),
        )
        .expect("range should only contain hex digit bytes");

        match u32::from_str_radix(src, 16) {
            Ok(value) => Ok(value),
            // This arm should be practically unreachable. I still return a LexerError here just to
            // be sure. (I cannot even think of a case where this arm matches.)
            Err(_) => Err(LexerErrorKind::InvalidHexEscape),
        }
    }

    fn advance_bytes_until_newline_or_eof(&mut self) {
        while let Some(byte) = self.next_byte(false) {
            if byte == b'\n' {
                break;
            }
        }
    }

    fn create_on_line(
        &self,
        kind: TokenKind,
        start_column: u32,
    ) -> Option<Result<Token, LexerError>> {
        Some(Ok(Token::new(
            kind,
            self.line,
            start_column,
            self.line,
            self.column - 1,
        )))
    }

    fn stop_and_error(&mut self, kind: LexerErrorKind) -> Option<Result<Token, LexerError>> {
        self.index = self.source.len();
        Some(Err(LexerError::new(kind, self.line, self.column)))
    }
}

impl Iterator for Lexer {
    type Item = Result<Token, LexerError>;

    fn next(&mut self) -> Option<Self::Item> {
        let start_line = self.line;
        let start_column = self.column;
        let start_index = self.index;
        match self.current_byte()? {
            // idents and keywords
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => {
                while let Some(b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_') =
                    self.next_byte(true)
                {}

                let value = self.string_from(start_index);

                // This keyword lookup is from Inko, and it is likely as efficient as it gets
                // without being too complex.
                let kind = match value.len() {
                    2 => match value.as_str() {
                        "if" => TokenKind::If,
                        "in" => TokenKind::In,
                        _ => TokenKind::Ident(value),
                    },
                    3 => match value.as_str() {
                        "for" => TokenKind::For,
                        "try" => TokenKind::Try,
                        _ => TokenKind::Ident(value),
                    },
                    4 => match value.as_str() {
                        "base" => TokenKind::Base,
                        "case" => TokenKind::Case,
                        "else" => TokenKind::Else,
                        "enum" => TokenKind::Enum,
                        "null" => TokenKind::Null,
                        "this" => TokenKind::This,
                        "true" => TokenKind::True,
                        _ => TokenKind::Ident(value),
                    },
                    5 => match value.as_str() {
                        "break" => TokenKind::Break,
                        "catch" => TokenKind::Catch,
                        "class" => TokenKind::Class,
                        "clone" => TokenKind::Clone,
                        "const" => TokenKind::Const,
                        "false" => TokenKind::False,
                        "local" => TokenKind::Local,
                        "throw" => TokenKind::Throw,
                        "while" => TokenKind::While,
                        "yield" => TokenKind::Yield,
                        _ => TokenKind::Ident(value),
                    },
                    6 => match value.as_str() {
                        "delete" => TokenKind::Delete,
                        "resume" => TokenKind::Resume,
                        "return" => TokenKind::Return,
                        "static" => TokenKind::Static,
                        "switch" => TokenKind::Switch,
                        "typeof" => TokenKind::Typeof,
                        _ => TokenKind::Ident(value),
                    },
                    7 => match value.as_str() {
                        "default" => TokenKind::Default,
                        "extends" => TokenKind::Extends,
                        "foreach" => TokenKind::Foreach,
                        "rawcall" => TokenKind::Rawcall,
                        _ => TokenKind::Ident(value),
                    },
                    8 => match value.as_str() {
                        "__FILE__" => TokenKind::File,
                        "__LINE__" => TokenKind::Line,
                        "continue" => TokenKind::Continue,
                        "function" => TokenKind::Function,
                        _ => TokenKind::Ident(value),
                    },
                    10 | 11 => match value.as_str() {
                        "instanceof" => TokenKind::Instanceof,
                        "constructor" => TokenKind::Constructor,
                        _ => TokenKind::Ident(value),
                    },
                    _ => TokenKind::Ident(value),
                };

                self.create_on_line(kind, start_column)
            }

            b'/' => match self.next_byte(false) {
                // "/* ... */" multi-line comment
                Some(b'*') => {
                    let ended_properly: bool;
                    let mut last_line_start_index = 0;
                    loop {
                        match self.next_byte(false) {
                            Some(b'*') => {
                                // If we don't peek here, in a case like "**/" the middle "*" will
                                // be skipped over and the comment won't end when it should end.
                                if self.peek_byte() == Some(b'/') {
                                    self.next_byte(false);
                                    self.next_byte(false);
                                    ended_properly = true;
                                    break;
                                }
                            }

                            Some(b'\n') => {
                                self.line += 1;
                                last_line_start_index = self.index + 1;
                            }

                            Some(_) => {}

                            None => {
                                // A newline is the very last byte of file whilst a multi-line
                                // comment block has started
                                //
                                // This takes care of the case that last_line_start_index can be out
                                // of range, so that string_from() doesn't panic.
                                if last_line_start_index >= self.source.len() {
                                    self.column = 1;
                                    return self
                                        .stop_and_error(LexerErrorKind::UnclosedMultiLineComment);
                                } else {
                                    ended_properly = false;
                                    break;
                                }
                            }
                        }
                    }

                    let value = self.string_from(start_index);

                    if last_line_start_index == 0 {
                        self.column += value.graphemes(true).count() as u32;
                    } else {
                        self.column = self
                            .string_from(last_line_start_index)
                            .graphemes(true)
                            .count() as u32
                            + 1;
                    }

                    if !ended_properly {
                        self.column -= 1;
                        return self.stop_and_error(LexerErrorKind::UnclosedMultiLineComment);
                    }

                    Some(Ok(Token::new(
                        TokenKind::MultiLineComment(value),
                        start_line,
                        start_column,
                        self.line,
                        self.column - 1,
                    )))
                }

                // "//" comment
                Some(b'/') => {
                    self.advance_bytes_until_newline_or_eof();
                    let value = self.string_from(start_index);
                    self.column += value.graphemes(true).count() as u32;
                    self.create_on_line(TokenKind::Comment(value), start_column)
                }

                // "/="
                Some(b'=') => {
                    self.next_byte(false);
                    self.column += 2;
                    self.create_on_line(TokenKind::DivEq, start_column)
                }

                // "/"
                _ => {
                    self.column += 1;
                    self.create_on_line(TokenKind::Div, start_column)
                }
            },

            // "#" comment
            b'#' => {
                self.advance_bytes_until_newline_or_eof();
                let value = self.string_from(start_index);
                self.column += value.graphemes(true).count() as u32;
                self.create_on_line(TokenKind::Comment(value), start_column)
            }

            b'@' => match self.next_byte(false) {
                // <@"..."> verbatim string literal
                //
                // This part of the logic is extremely similar to "/* ... */" multi-line comment's,
                // so refer to the comments there regarding implementation.
                //
                // Because of that, apart from the slight difference in eating-up-bytes logic, the
                // two parts basically have the same code. It might be possible to cut down on
                // redundant code, but I haven't figured out a way of doing it yet.
                Some(b'"') => {
                    let ended_properly: bool;
                    let mut last_line_start_index = 0;
                    loop {
                        match self.next_byte(false) {
                            Some(b'"') => {
                                // We don't need to peek here because we want to skip over the
                                // second <"> in <"">.
                                if self.next_byte(false) != Some(b'"') {
                                    ended_properly = true;
                                    break;
                                }
                            }

                            Some(b'\n') => {
                                self.line += 1;
                                last_line_start_index = self.index + 1;
                            }

                            Some(_) => {}

                            None => {
                                if last_line_start_index >= self.source.len() {
                                    self.column = 1;
                                    return self
                                        .stop_and_error(LexerErrorKind::UnclosedVerbatimString);
                                } else {
                                    ended_properly = false;
                                    break;
                                }
                            }
                        }
                    }

                    let value = self.string_from(start_index);

                    if last_line_start_index == 0 {
                        self.column += value.graphemes(true).count() as u32;
                    } else {
                        self.column = self
                            .string_from(last_line_start_index)
                            .graphemes(true)
                            .count() as u32
                            + 1;
                    }

                    if !ended_properly {
                        self.column -= 1;
                        return self.stop_and_error(LexerErrorKind::UnclosedVerbatimString);
                    }

                    Some(Ok(Token::new(
                        TokenKind::Lit(value),
                        start_line,
                        start_column,
                        self.line,
                        self.column - 1,
                    )))
                }

                // "@", signaling a lambda expression
                _ => {
                    self.column += 1;
                    self.create_on_line(TokenKind::At, start_column)
                }
            },

            // numerical literal
            b'0'..=b'9' => {
                if self.current_byte()? == b'0' {
                    match self.next_byte(true) {
                        Some(b'0'..=b'7') => {
                            loop {
                                match self.next_byte(true) {
                                    Some(b'0'..=b'7') => {}
                                    Some(b'8' | b'9') => {
                                        return self.stop_and_error(LexerErrorKind::InvalidOctal);
                                    }
                                    _ => break,
                                }
                            }

                            let value = self.string_from(start_index);
                            return self.create_on_line(TokenKind::Lit(value), start_column);
                        }

                        Some(b'x' | b'X') => {
                            while let Some(b'A'..=b'F' | b'a'..=b'f' | b'0'..=b'9') =
                                self.next_byte(true)
                            {}

                            let value = self.string_from(start_index);
                            return self.create_on_line(TokenKind::Lit(value), start_column);
                        }

                        Some(b'8' | b'9') => {
                            while let Some(b'0'..=b'9') = self.next_byte(true) {}

                            let value = self.string_from(start_index);
                            return self.create_on_line(TokenKind::Lit(value), start_column);
                        }

                        Some(b'.' | b'e' | b'E') => {}

                        _ => return self.create_on_line(TokenKind::Lit("0".into()), start_column),
                    }
                }

                // The messiness of these matches lies in the different column handling depending on
                // if `Some(_)` (e.g. 9.5eg, error should point at the "g") or
                // `None` (e.g. 3.1e<stop>, error should point at the "e") is matched.
                loop {
                    match self.current_byte() {
                        Some(b'e' | b'E') => match self.next_byte(true) {
                            Some(b'0'..=b'9') => {}

                            Some(b'+' | b'-') => match self.next_byte(true) {
                                Some(b'0'..=b'9') => {}

                                Some(_) => {
                                    return self
                                        .stop_and_error(LexerErrorKind::MissingFloatExponent);
                                }

                                None => {
                                    self.column -= 1;
                                    return self
                                        .stop_and_error(LexerErrorKind::MissingFloatExponent);
                                }
                            },

                            Some(_) => {
                                return self.stop_and_error(LexerErrorKind::MissingFloatExponent);
                            }

                            None => {
                                self.column -= 1;
                                return self.stop_and_error(LexerErrorKind::MissingFloatExponent);
                            }
                        },

                        Some(b'.' | b'0'..=b'9') => {}
                        _ => break,
                    }

                    self.next_byte(true);
                }

                let value = self.string_from(start_index);
                self.create_on_line(TokenKind::Lit(value), start_column)
            }

            // "'...'" character code literal
            b'\'' => {
                match self.next_byte(true) {
                    Some(b'\\') => match self.advance_escape_sequence(true) {
                        Ok(_) => {}
                        Err(kind) => {
                            if kind == LexerErrorKind::UnexpectedEof {
                                self.column -= 1;
                            }
                            return self.stop_and_error(kind);
                        }
                    },

                    Some(b'\n') | None => {
                        self.column -= 1;
                        return self.stop_and_error(LexerErrorKind::UnclosedChar);
                    }

                    Some(b'\'') => return self.stop_and_error(LexerErrorKind::EmptyChar),
                    Some(0..128) => {}
                    Some(_) => return self.stop_and_error(LexerErrorKind::CharOob),
                };

                match self.next_byte(true) {
                    Some(b'\'') => {
                        self.next_byte(true);
                        let value = self.string_from(start_index);
                        self.create_on_line(TokenKind::Lit(value), start_column)
                    }

                    Some(b'\n') | None => {
                        self.column -= 1;
                        self.stop_and_error(LexerErrorKind::UnclosedChar)
                    }

                    _ => self.stop_and_error(LexerErrorKind::CharTooLong),
                }
            }

            // <"..."> string literal
            b'"' => {
                loop {
                    match self.next_byte(false) {
                        Some(b'\\') => match self.advance_escape_sequence(false) {
                            Ok(_) => {}
                            Err(kind) => {
                                self.column +=
                                    self.string_from(start_index).graphemes(true).count() as u32;

                                if kind == LexerErrorKind::UnexpectedEof {
                                    self.column -= 1;
                                }

                                return self.stop_and_error(kind);
                            }
                        },

                        Some(b'\n') | None => {
                            self.column +=
                                self.string_from(start_index).graphemes(true).count() as u32 - 1;
                            return self.stop_and_error(LexerErrorKind::UnclosedString);
                        }

                        Some(b'"') => break,
                        _ => {}
                    }
                }

                self.next_byte(false);
                let value = self.string_from(start_index);
                self.column += value.graphemes(true).count() as u32;
                self.create_on_line(TokenKind::Lit(value), start_column)
            }

            // "+", "+=" or "++"
            b'+' => {
                let kind = match self.next_byte(true) {
                    Some(b'=') => TokenKind::PlusEq,
                    Some(b'+') => TokenKind::Inc,
                    _ => TokenKind::Plus,
                };

                if kind != TokenKind::Plus {
                    self.next_byte(true);
                }

                self.create_on_line(kind, start_column)
            }

            // "-", "-=" or "--"
            b'-' => {
                let kind = match self.next_byte(true) {
                    Some(b'=') => TokenKind::MinusEq,
                    Some(b'-') => TokenKind::Dec,
                    _ => TokenKind::Minus,
                };

                if kind != TokenKind::Minus {
                    self.next_byte(true);
                }

                self.create_on_line(kind, start_column)
            }

            // "*" or "*="
            b'*' => {
                let kind = match self.next_byte(true) {
                    Some(b'=') => TokenKind::MultEq,
                    _ => TokenKind::Mult,
                };

                if kind != TokenKind::Mult {
                    self.next_byte(true);
                }

                self.create_on_line(kind, start_column)
            }

            // "%" or "%="
            b'%' => {
                let kind = match self.next_byte(true) {
                    Some(b'=') => TokenKind::ModEq,
                    _ => TokenKind::Mod,
                };

                if kind != TokenKind::Mod {
                    self.next_byte(true);
                }

                self.create_on_line(kind, start_column)
            }

            // "!" or "!="
            b'!' => {
                let kind = match self.next_byte(true) {
                    Some(b'=') => TokenKind::Neq,
                    _ => TokenKind::Not,
                };

                if kind != TokenKind::Not {
                    self.next_byte(true);
                }

                self.create_on_line(kind, start_column)
            }

            // "=" or "=="
            b'=' => {
                let kind = match self.next_byte(true) {
                    Some(b'=') => TokenKind::EqEq,
                    _ => TokenKind::Eq,
                };

                if kind != TokenKind::Eq {
                    self.next_byte(true);
                }

                self.create_on_line(kind, start_column)
            }

            // "&" or "&&"
            b'&' => {
                let kind = match self.next_byte(true) {
                    Some(b'&') => TokenKind::And,
                    _ => TokenKind::BitAnd,
                };

                if kind != TokenKind::BitAnd {
                    self.next_byte(true);
                }

                self.create_on_line(kind, start_column)
            }

            // "|" or "||"
            b'|' => {
                let kind = match self.next_byte(true) {
                    Some(b'|') => TokenKind::Or,
                    _ => TokenKind::BitOr,
                };

                if kind != TokenKind::BitOr {
                    self.next_byte(true);
                }

                self.create_on_line(kind, start_column)
            }

            // ":" or "::"
            b':' => {
                let kind = match self.next_byte(true) {
                    Some(b':') => TokenKind::Scope,
                    _ => TokenKind::Colon,
                };

                if kind != TokenKind::Colon {
                    self.next_byte(true);
                }

                self.create_on_line(kind, start_column)
            }

            // "<", "<<", "<-", "<=", or "<=>"
            b'<' => {
                let kind = match self.next_byte(true) {
                    Some(b'<') => TokenKind::BitLeft,
                    Some(b'-') => TokenKind::Ins,
                    Some(b'=') => match self.next_byte(true) {
                        Some(b'>') => TokenKind::Spaceship,
                        _ => TokenKind::Le,
                    },
                    _ => TokenKind::Lt,
                };

                if kind != TokenKind::Lt && kind != TokenKind::Le {
                    self.next_byte(true);
                }

                self.create_on_line(kind, start_column)
            }

            // ">", ">>", ">>>" or ">="
            b'>' => {
                let kind = match self.next_byte(true) {
                    Some(b'=') => TokenKind::Ge,
                    Some(b'>') => match self.next_byte(true) {
                        Some(b'>') => TokenKind::BitUnsRight,
                        _ => TokenKind::BitRight,
                    },
                    _ => TokenKind::Gt,
                };

                if kind != TokenKind::Gt && kind != TokenKind::BitRight {
                    self.next_byte(true);
                }

                self.create_on_line(kind, start_column)
            }

            // "." or "..."
            b'.' => {
                let kind = match self.next_byte(true) {
                    Some(b'.') => match self.next_byte(true) {
                        Some(b'.') => Some(TokenKind::Ellipsis),
                        _ => None,
                    },
                    _ => Some(TokenKind::Dot),
                };

                let Some(kind) = kind else {
                    self.column -= 1;
                    return self.stop_and_error(LexerErrorKind::DoubleDot);
                };

                if kind != TokenKind::Dot {
                    self.next_byte(true);
                }

                self.create_on_line(kind, start_column)
            }

            b'^' => {
                self.next_byte(true);
                self.create_on_line(TokenKind::BitXor, start_column)
            }

            b'~' => {
                self.next_byte(true);
                self.create_on_line(TokenKind::BitNot, start_column)
            }

            b',' => {
                self.next_byte(true);
                self.create_on_line(TokenKind::Comma, start_column)
            }

            b'?' => {
                self.next_byte(true);
                self.create_on_line(TokenKind::Question, start_column)
            }

            b'(' => {
                self.next_byte(true);
                self.create_on_line(TokenKind::ParenOpen, start_column)
            }

            b')' => {
                self.next_byte(true);
                self.create_on_line(TokenKind::ParenClose, start_column)
            }

            b'[' => {
                self.next_byte(true);
                self.create_on_line(TokenKind::SquareOpen, start_column)
            }

            b']' => {
                self.next_byte(true);
                self.create_on_line(TokenKind::SquareClose, start_column)
            }

            b'{' => {
                self.next_byte(true);
                self.create_on_line(TokenKind::BraceOpen, start_column)
            }

            b'}' => {
                self.next_byte(true);
                self.create_on_line(TokenKind::BraceClose, start_column)
            }

            b';' => {
                self.next_byte(true);
                self.create_on_line(TokenKind::Semicolon, start_column)
            }

            // whitespaces
            b' ' | b'\t' => {
                while let Some(b' ' | b'\t') = self.next_byte(true) {}
                let value = self.string_from(start_index);
                self.create_on_line(TokenKind::Whitespace(value), start_column)
            }

            b'\n' => {
                self.column += 1;
                let token = self.create_on_line(TokenKind::Newline, start_column);
                self.advance_line();
                token
            }

            _ => self.stop_and_error(LexerErrorKind::UnexpectedSymbol),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum LexerErrorKind {
    /// A symbol outside of the ASCII range (0 to 127 inclusive) was encountered in a character code
    /// literal.
    CharOob,
    /// More than one symbol were encountered in a character code literal.
    CharTooLong,
    /// An empty character code literal was encountered, i.e. `''`.
    EmptyChar,
    /// A character code literal was unclosed.
    UnclosedChar,
    /// An invalid escape sequence was encountered.
    InvalidEscape,
    /// An invalid hexadecimal escape sequence was encountered. This error is only returned from the
    /// sequences `\x`, `\u` and `\U`.
    InvalidHexEscape,
    /// An invalid octal number was encountered.
    ///
    /// Squirrel lexes octal numbers weirdly. If the second digit contains non-octal digits, i.e. 8
    /// and 9, the entire number is understood as decimal. Otherwise, it is octal and octal lexing
    /// rules apply.
    ///
    /// # Examples
    ///
    /// - `091` is decimal.
    /// - `072` is octal.
    /// - `028` is invalid octal.
    /// - `0006` is octal.
    InvalidOctal,
    /// An exponent for floating point numbers written in scientific notation was missing.
    MissingFloatExponent,
    /// A multi-line comment was unclosed.
    UnclosedMultiLineComment,
    /// A verbatim string was unclosed.
    UnclosedVerbatimString,
    /// A string was unclosed.
    UnclosedString,
    /// An unexpected symbol was encountered.
    UnexpectedSymbol,
    /// A `..` (not a single dot, nor an ellipsis) was encountered.
    DoubleDot,
    /// An unexpected end of file was encountered. So far, this error will only appear if an end of
    /// file directly follows a `\`, i.e. an escape sequence was started but file ended immediately
    /// afterwards.
    UnexpectedEof,
}

#[derive(Constructor, Debug, PartialEq)]
pub struct LexerError {
    pub kind: LexerErrorKind,
    pub line: u32,
    pub column: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use LexerErrorKind::*;
    use TokenKind::*;

    fn token(kind: TokenKind, start: (u32, u32), end: (u32, u32)) -> Token {
        Token::new(kind, start.0, start.1, end.0, end.1)
    }

    fn error(kind: LexerErrorKind, line: u32, column: u32) -> LexerError {
        LexerError::new(kind, line, column)
    }

    macro_rules! assert_stream {
        (
            $source: expr,
            $(
                $token: expr
            ),+
        ) => {{
            let vec_source = Lexer::new($source).collect::<Result<Vec<Token>, LexerError>>().unwrap();
            assert_eq!(vec_source, vec![$($token,)+]);
        }};
    }

    macro_rules! assert_error {
        (
            $source: expr,
            $kind: expr,
            $line: expr,
            $column: expr
        ) => {{
            let source = Lexer::new($source)
                .collect::<Result<Vec<Token>, LexerError>>()
                .unwrap_err();
            assert_eq!(source, error($kind, $line, $column));
        }};
    }

    #[test]
    fn empty() {
        let mut lexer = Lexer::new("");
        assert_eq!(lexer.next(), None);
        assert_eq!(lexer.next(), None);
    }

    #[test]
    #[rustfmt::skip]
    fn idents() {
        // unused variable
        assert_stream!("_", token(Ident("_".into()), (1, 1), (1, 1)));
        assert_stream!("f", token(Ident("f".into()), (1, 1), (1, 1)));
        assert_stream!("F", token(Ident("F".into()), (1, 1), (1, 1)));
        assert_stream!("f1", token(Ident("f1".into()), (1, 1), (1, 2)));
        assert_stream!("_1", token(Ident("_1".into()), (1, 1), (1, 2)));
        assert_stream!("__", token(Ident("__".into()), (1, 1), (1, 2)));
        // general variable
        assert_stream!("foo", token(Ident("foo".into()), (1, 1), (1, 3)));
        assert_stream!("__fo", token(Ident("__fo".into()), (1, 1), (1, 4)));
        assert_stream!("__2fo", token(Ident("__2fo".into()), (1, 1), (1, 5)));
        // PascalCase
        assert_stream!("FooBar", token(Ident("FooBar".into()), (1, 1), (1, 6)));
        assert_stream!("fOo2BaR", token(Ident("fOo2BaR".into()), (1, 1), (1, 7)));
        // camelCase
        assert_stream!("fooBarBa", token(Ident("fooBarBa".into()), (1, 1), (1, 8)));
        // SCREAMING_SNAKE_CASE
        assert_stream!("HALF_LIFE", token(Ident("HALF_LIFE".into()), (1, 1), (1, 9)));
        // snake_case
        assert_stream!("portal_two", token(Ident("portal_two".into()), (1, 1), (1, 10)));
        // a general script function beginning with "_"
        assert_stream!("__DumpScope", token(Ident("__DumpScope".into()), (1, 1), (1, 11)));
        assert_stream!("__0foobarbaz", token(Ident("__0foobarbaz".into()), (1, 1), (1, 12)));
        assert_stream!("___0123456789", token(Ident("___0123456789".into()), (1, 1), (1, 13)));
    }

    #[test]
    #[rustfmt::skip]
    fn keywords() {
        assert_stream!("base", token(Base, (1, 1), (1, 4)));
        assert_stream!("break", token(Break, (1, 1), (1, 5)));
        assert_stream!("case", token(Case, (1, 1), (1, 4)));
        assert_stream!("catch", token(Catch, (1, 1), (1, 5)));
        assert_stream!("class", token(Class, (1, 1), (1, 5)));
        assert_stream!("clone", token(Clone, (1, 1), (1, 5)));
        assert_stream!("const", token(Const, (1, 1), (1, 5)));
        assert_stream!("constructor", token(Constructor, (1, 1), (1, 11)));
        assert_stream!("continue", token(Continue, (1, 1), (1, 8)));
        assert_stream!("default", token(Default, (1, 1), (1, 7)));
        assert_stream!("delete", token(Delete, (1, 1), (1, 6)));
        assert_stream!("else", token(Else, (1, 1), (1, 4)));
        assert_stream!("enum", token(Enum, (1, 1), (1, 4)));
        assert_stream!("extends", token(Extends, (1, 1), (1, 7)));
        assert_stream!("false", token(False, (1, 1), (1, 5)));
        assert_stream!("__FILE__", token(File, (1, 1), (1, 8)));
        assert_stream!("for", token(For, (1, 1), (1, 3)));
        assert_stream!("foreach", token(Foreach, (1, 1), (1, 7)));
        assert_stream!("function", token(Function, (1, 1), (1, 8)));
        assert_stream!("if", token(If, (1, 1), (1, 2)));
        assert_stream!("in", token(In, (1, 1), (1, 2)));
        assert_stream!("instanceof", token(Instanceof, (1, 1), (1, 10)));
        assert_stream!("__LINE__", token(Line, (1, 1), (1, 8)));
        assert_stream!("local", token(Local, (1, 1), (1, 5)));
        assert_stream!("null", token(Null, (1, 1), (1, 4)));
        assert_stream!("rawcall", token(Rawcall, (1, 1), (1, 7)));
        assert_stream!("resume", token(Resume, (1, 1), (1, 6)));
        assert_stream!("return", token(Return, (1, 1), (1, 6)));
        assert_stream!("static", token(Static, (1, 1), (1, 6)));
        assert_stream!("switch", token(Switch, (1, 1), (1, 6)));
        assert_stream!("this", token(This, (1, 1), (1, 4)));
        assert_stream!("throw", token(Throw, (1, 1), (1, 5)));
        assert_stream!("true", token(True, (1, 1), (1, 4)));
        assert_stream!("try", token(Try, (1, 1), (1, 3)));
        assert_stream!("typeof", token(Typeof, (1, 1), (1, 6)));
        assert_stream!("while", token(While, (1, 1), (1, 5)));
        assert_stream!("yield", token(Yield, (1, 1), (1, 5)));
    }

    #[test]
    fn symbols() {
        assert_stream!("+", token(Plus, (1, 1), (1, 1)));
        assert_stream!("+=", token(PlusEq, (1, 1), (1, 2)));
        assert_stream!("++", token(Inc, (1, 1), (1, 2)));
        assert_stream!("-", token(Minus, (1, 1), (1, 1)));
        assert_stream!("-=", token(MinusEq, (1, 1), (1, 2)));
        assert_stream!("--", token(Dec, (1, 1), (1, 2)));
        assert_stream!("*", token(Mult, (1, 1), (1, 1)));
        assert_stream!("*=", token(MultEq, (1, 1), (1, 2)));
        assert_stream!("/", token(Div, (1, 1), (1, 1)));
        assert_stream!("/=", token(DivEq, (1, 1), (1, 2)));
        assert_stream!("%", token(Mod, (1, 1), (1, 1)));
        assert_stream!("%=", token(ModEq, (1, 1), (1, 2)));

        assert_stream!("&", token(BitAnd, (1, 1), (1, 1)));
        assert_stream!("|", token(BitOr, (1, 1), (1, 1)));
        assert_stream!("^", token(BitXor, (1, 1), (1, 1)));
        assert_stream!("~", token(BitNot, (1, 1), (1, 1)));

        assert_stream!("&&", token(And, (1, 1), (1, 2)));
        assert_stream!("||", token(Or, (1, 1), (1, 2)));
        assert_stream!("!", token(Not, (1, 1), (1, 1)));

        assert_stream!("<<", token(BitLeft, (1, 1), (1, 2)));
        assert_stream!(">>", token(BitRight, (1, 1), (1, 2)));
        assert_stream!(">>>", token(BitUnsRight, (1, 1), (1, 3)));

        assert_stream!("<", token(Lt, (1, 1), (1, 1)));
        assert_stream!("<=", token(Le, (1, 1), (1, 2)));
        assert_stream!(">", token(Gt, (1, 1), (1, 1)));
        assert_stream!(">=", token(Ge, (1, 1), (1, 2)));
        assert_stream!("==", token(EqEq, (1, 1), (1, 2)));
        assert_stream!("!=", token(Neq, (1, 1), (1, 2)));
        assert_stream!("<=>", token(Spaceship, (1, 1), (1, 3)));

        assert_stream!("=", token(Eq, (1, 1), (1, 1)));
        assert_stream!("<-", token(Ins, (1, 1), (1, 2)));
        assert_stream!(",", token(Comma, (1, 1), (1, 1)));
        assert_stream!("?", token(Question, (1, 1), (1, 1)));

        assert_stream!("(", token(ParenOpen, (1, 1), (1, 1)));
        assert_stream!(")", token(ParenClose, (1, 1), (1, 1)));
        assert_stream!("[", token(SquareOpen, (1, 1), (1, 1)));
        assert_stream!("]", token(SquareClose, (1, 1), (1, 1)));
        assert_stream!("{", token(BraceOpen, (1, 1), (1, 1)));
        assert_stream!("}", token(BraceClose, (1, 1), (1, 1)));
        assert_stream!(".", token(Dot, (1, 1), (1, 1)));
        assert_stream!("...", token(Ellipsis, (1, 1), (1, 3)));
        assert_stream!(":", token(Colon, (1, 1), (1, 1)));
        assert_stream!(";", token(Semicolon, (1, 1), (1, 1)));
        assert_stream!("::", token(Scope, (1, 1), (1, 2)));
        assert_stream!("@", token(At, (1, 1), (1, 1)));
    }

    #[test]
    fn string_empty() {
        assert_stream!("\"\"", token(Lit("\"\"".into()), (1, 1), (1, 2)));
    }

    #[test]
    fn string() {
        // general netprop
        assert_stream!(
            "\"m_iszMvMPopfileName\"",
            token(Lit("\"m_iszMvMPopfileName\"".into()), (1, 1), (1, 21))
        );
        // unicode characters
        assert_stream!(
            "\"viele Möglichkeiten\"",
            token(Lit("\"viele Möglichkeiten\"".into()), (1, 1), (1, 21))
        );
    }

    #[test]
    fn char_code() {
        assert_stream!("'_'", token(Lit("'_'".into()), (1, 1), (1, 3)));
        assert_stream!("'a'", token(Lit("'a'".into()), (1, 1), (1, 3)));
        assert_stream!("'Z'", token(Lit("'Z'".into()), (1, 1), (1, 3)));
    }

    #[test]
    fn escape_sequences() {
        // string literals
        assert_stream!(r#""\t""#, token(Lit(r#""\t""#.into()), (1, 1), (1, 4)));
        assert_stream!(r#""\a""#, token(Lit(r#""\a""#.into()), (1, 1), (1, 4)));
        assert_stream!(r#""\b""#, token(Lit(r#""\b""#.into()), (1, 1), (1, 4)));
        assert_stream!(r#""\n""#, token(Lit(r#""\n""#.into()), (1, 1), (1, 4)));
        assert_stream!(r#""\r""#, token(Lit(r#""\r""#.into()), (1, 1), (1, 4)));
        assert_stream!(r#""\v""#, token(Lit(r#""\v""#.into()), (1, 1), (1, 4)));
        assert_stream!(r#""\f""#, token(Lit(r#""\f""#.into()), (1, 1), (1, 4)));
        assert_stream!(r#""\\""#, token(Lit(r#""\\""#.into()), (1, 1), (1, 4)));
        assert_stream!(r#""\"""#, token(Lit(r#""\"""#.into()), (1, 1), (1, 4)));
        assert_stream!(r#""\'""#, token(Lit(r#""\'""#.into()), (1, 1), (1, 4)));
        assert_stream!(r#""\0""#, token(Lit(r#""\0""#.into()), (1, 1), (1, 4)));
        assert_stream!(r#""\xf""#, token(Lit(r#""\xf""#.into()), (1, 1), (1, 5)));
        assert_stream!(r#""\xFF""#, token(Lit(r#""\xFF""#.into()), (1, 1), (1, 6)));
        assert_stream!(r#""\uf""#, token(Lit(r#""\uf""#.into()), (1, 1), (1, 5)));
        assert_stream!(
            r#""\uFFFF""#,
            token(Lit(r#""\uFFFF""#.into()), (1, 1), (1, 8))
        );
        assert_stream!(r#""\Uf""#, token(Lit(r#""\Uf""#.into()), (1, 1), (1, 5)));
        assert_stream!(
            r#""\UFFFFFFFF""#,
            token(Lit(r#""\UFFFFFFFF""#.into()), (1, 1), (1, 12))
        );

        // character code literals
        assert_stream!("'\\t'", token(Lit("'\\t'".into()), (1, 1), (1, 4)));
        assert_stream!("'\\a'", token(Lit("'\\a'".into()), (1, 1), (1, 4)));
        assert_stream!("'\\b'", token(Lit("'\\b'".into()), (1, 1), (1, 4)));
        assert_stream!("'\\n'", token(Lit("'\\n'".into()), (1, 1), (1, 4)));
        assert_stream!("'\\r'", token(Lit("'\\r'".into()), (1, 1), (1, 4)));
        assert_stream!("'\\v'", token(Lit("'\\v'".into()), (1, 1), (1, 4)));
        assert_stream!("'\\f'", token(Lit("'\\f'".into()), (1, 1), (1, 4)));
        assert_stream!("'\\\\'", token(Lit("'\\\\'".into()), (1, 1), (1, 4)));
        assert_stream!("'\\\"'", token(Lit("'\\\"'".into()), (1, 1), (1, 4)));
        assert_stream!("'\\''", token(Lit("'\\''".into()), (1, 1), (1, 4)));
        assert_stream!("'\\0'", token(Lit("'\\0'".into()), (1, 1), (1, 4)));
        assert_stream!("'\\xf'", token(Lit("'\\xf'".into()), (1, 1), (1, 5)));
        assert_stream!("'\\xFF'", token(Lit("'\\xFF'".into()), (1, 1), (1, 6)));
        assert_stream!("'\\uf'", token(Lit("'\\uf'".into()), (1, 1), (1, 5)));
        assert_stream!("'\\u007F'", token(Lit("'\\u007F'".into()), (1, 1), (1, 8)));
        assert_stream!("'\\Uf'", token(Lit("'\\Uf'".into()), (1, 1), (1, 5)));
        assert_stream!(
            "'\\U0000007F'",
            token(Lit("'\\U0000007F'".into()), (1, 1), (1, 12))
        );
    }

    #[test]
    fn numbers() {
        // octals
        assert_stream!("0", token(Lit("0".into()), (1, 1), (1, 1)));
        assert_stream!("000", token(Lit("000".into()), (1, 1), (1, 3)));
        assert_stream!("07127", token(Lit("07127".into()), (1, 1), (1, 5)));
        assert_stream!("003400005", token(Lit("003400005".into()), (1, 1), (1, 9)));

        // decimals
        assert_stream!("2", token(Lit("2".into()), (1, 1), (1, 1)));
        assert_stream!("420", token(Lit("420".into()), (1, 1), (1, 3)));
        assert_stream!("1337", token(Lit("1337".into()), (1, 1), (1, 4)));
        assert_stream!("56789", token(Lit("56789".into()), (1, 1), (1, 5)));

        // hexadecimals
        assert_stream!("0x", token(Lit("0x".into()), (1, 1), (1, 2)));
        assert_stream!("0X", token(Lit("0X".into()), (1, 1), (1, 2)));
        assert_stream!("0x012aBc", token(Lit("0x012aBc".into()), (1, 1), (1, 8)));
        assert_stream!("0X034CdE", token(Lit("0X034CdE".into()), (1, 1), (1, 8)));
        assert_stream!("0x567AbCd", token(Lit("0x567AbCd".into()), (1, 1), (1, 9)));
        assert_stream!("0X890cDeF", token(Lit("0X890cDeF".into()), (1, 1), (1, 9)));

        // floats
        assert_stream!("0.", token(Lit("0.".into()), (1, 1), (1, 2)));
        assert_stream!("0.0", token(Lit("0.0".into()), (1, 1), (1, 3)));
        assert_stream!("0.015", token(Lit("0.015".into()), (1, 1), (1, 5)));
        assert_stream!("2.71", token(Lit("2.71".into()), (1, 1), (1, 4)));
        assert_stream!("3e8", token(Lit("3e8".into()), (1, 1), (1, 3)));
        assert_stream!("6.02e+23", token(Lit("6.02e+23".into()), (1, 1), (1, 8)));
        assert_stream!("1.6e-19", token(Lit("1.6e-19".into()), (1, 1), (1, 7)));
        assert_stream!("44.1E3", token(Lit("44.1E3".into()), (1, 1), (1, 6)));
        assert_stream!("192E+3", token(Lit("192E+3".into()), (1, 1), (1, 6)));
        assert_stream!("1.38E-23", token(Lit("1.38E-23".into()), (1, 1), (1, 8)));

        // insane floats that somehow lexes
        assert_stream!("5.35....1", token(Lit("5.35....1".into()), (1, 1), (1, 9)));
        assert_stream!("0...e2", token(Lit("0...e2".into()), (1, 1), (1, 6)));
        assert_stream!(
            "1e5.125..e8...e-1..E+12.0e+10",
            token(Lit("1e5.125..e8...e-1..E+12.0e+10".into()), (1, 1), (1, 29))
        );
        assert_stream!(
            "4.5e+4a-2",
            token(Lit("4.5e+4".into()), (1, 1), (1, 6)),
            token(Ident("a".into()), (1, 7), (1, 7)),
            token(Minus, (1, 8), (1, 8)),
            token(Lit("2".into()), (1, 9), (1, 9))
        );
    }

    #[test]
    fn comments_empty() {
        assert_stream!("//", token(Comment("//".into()), (1, 1), (1, 2)));
        assert_stream!("#", token(Comment("#".into()), (1, 1), (1, 1)));
    }

    #[test]
    fn comments() {
        assert_stream!(
            "// viele Möglichkeiten",
            token(Comment("// viele Möglichkeiten".into()), (1, 1), (1, 22))
        );

        assert_stream!(
            "// viele Möglichkeiten\n",
            token(Comment("// viele Möglichkeiten".into()), (1, 1), (1, 22)),
            token(Newline, (1, 23), (1, 23))
        );

        assert_stream!(
            "# viele Möglichkeiten",
            token(Comment("# viele Möglichkeiten".into()), (1, 1), (1, 21))
        );

        assert_stream!(
            "# viele Möglichkeiten\n",
            token(Comment("# viele Möglichkeiten".into()), (1, 1), (1, 21)),
            token(Newline, (1, 22), (1, 22))
        );
    }

    #[test]
    fn multi_line_comment_empty() {
        assert_stream!(
            "/**/",
            token(MultiLineComment("/**/".into()), (1, 1), (1, 4))
        );
    }

    #[test]
    fn multi_line_comment() {
        assert_stream!(
            "/* viele Möglichkeiten * / /* **/",
            token(
                MultiLineComment("/* viele Möglichkeiten * / /* **/".into()),
                (1, 1),
                (1, 33)
            )
        );

        assert_stream!(
            "/* /* */*/",
            token(MultiLineComment("/* /* */".into()), (1, 1), (1, 8)),
            token(Mult, (1, 9), (1, 9)),
            token(Div, (1, 10), (1, 10))
        );

        #[rustfmt::skip]
        assert_stream!(
            r#"/** ganz
                *  viele
                *  Möglichkeiten */"#,
            token(
                MultiLineComment(
            r#"/** ganz
                *  viele
                *  Möglichkeiten */"#.into()
                ),
                (1, 1),
                (3, 35)
            )
        );
    }

    #[test]
    fn verbatim_string_empty() {
        assert_stream!("@\"\"", token(Lit("@\"\"".into()), (1, 1), (1, 3)));
    }

    #[test]
    fn verbatim_string() {
        assert_stream!(
            "@\"viele Möglichkeiten\"",
            token(Lit("@\"viele Möglichkeiten\"".into()), (1, 1), (1, 22))
        );
        assert_stream!(
            r#"@"ganz
viele
Möglichkeiten""#,
            token(
                Lit("@\"ganz\nviele\nMöglichkeiten\"".into()),
                (1, 1),
                (3, 14)
            )
        );
        assert_stream!(
            r#"@"no ""escapes"", \R\E\A\L\L\Y!""#,
            token(
                Lit(r#"@"no ""escapes"", \R\E\A\L\L\Y!""#.into()),
                (1, 1),
                (1, 32)
            )
        );
        assert_stream!(
            r#"@"no ""escapes""""#,
            token(Lit(r#"@"no ""escapes""""#.into()), (1, 1), (1, 17))
        );
    }

    #[test]
    fn whitespace() {
        assert_stream!(" ", token(Whitespace(" ".into()), (1, 1), (1, 1)));
        assert_stream!("\t", token(Whitespace("\t".into()), (1, 1), (1, 1)));
        assert_stream!(
            " a  b   c\td",
            token(Whitespace(" ".into()), (1, 1), (1, 1)),
            token(Ident("a".into()), (1, 2), (1, 2)),
            token(Whitespace("  ".into()), (1, 3), (1, 4)),
            token(Ident("b".into()), (1, 5), (1, 5)),
            token(Whitespace("   ".into()), (1, 6), (1, 8)),
            token(Ident("c".into()), (1, 9), (1, 9)),
            token(Whitespace("\t".into()), (1, 10), (1, 10)),
            token(Ident("d".into()), (1, 11), (1, 11))
        );
    }

    #[test]
    fn newline() {
        assert_stream!("\n", token(Newline, (1, 1), (1, 1)));
        assert_stream!(
            "a\nbc\n",
            token(Ident("a".into()), (1, 1), (1, 1)),
            token(Newline, (1, 2), (1, 2)),
            token(Ident("bc".into()), (2, 1), (2, 2)),
            token(Newline, (2, 3), (2, 3))
        );
    }

    #[test]
    fn error_unexpected_symbol() {
        assert_error!("ändern", UnexpectedSymbol, 1, 1);
        assert_error!("hä?", UnexpectedSymbol, 1, 2);
    }

    #[test]
    fn error_double_dot() {
        assert_error!("a..b", DoubleDot, 1, 3);
    }

    #[test]
    fn error_unclosed_string() {
        assert_error!("\"", UnclosedString, 1, 1);
        assert_error!("\"hä?", UnclosedString, 1, 4);
        assert_error!("\"\n", UnclosedString, 1, 1);
        assert_error!("\"hä?\n", UnclosedString, 1, 4);
    }

    #[test]
    fn error_char_oob() {
        assert_error!("'ä'", CharOob, 1, 2);
        assert_error!("'\\u0080'", CharOob, 1, 7);
        assert_error!("'\\uFFFF'", CharOob, 1, 7);
        assert_error!("'\\U00000080'", CharOob, 1, 11);
        assert_error!("'\\UFFFFFFFF'", CharOob, 1, 11);
    }

    #[test]
    fn error_char_too_long() {
        assert_error!("'xd'", CharTooLong, 1, 3);
        assert_error!("'\\xffg'", CharTooLong, 1, 6);
        assert_error!("'\\u007fg'", CharTooLong, 1, 8);
        assert_error!("'\\U0000007fg'", CharTooLong, 1, 12);
    }

    #[test]
    fn error_empty_char() {
        assert_error!("''", EmptyChar, 1, 2);
    }

    #[test]
    fn error_unclosed_char() {
        assert_error!("'", UnclosedChar, 1, 1);
        assert_error!("'\n", UnclosedChar, 1, 1);
        assert_error!("'a", UnclosedChar, 1, 2);
        assert_error!("'a\n", UnclosedChar, 1, 2);
        assert_error!("'\\xff", UnclosedChar, 1, 5);
        assert_error!("'\\xff\n", UnclosedChar, 1, 5);
        assert_error!("'\\u007f", UnclosedChar, 1, 7);
        assert_error!("'\\u007f\n", UnclosedChar, 1, 7);
        assert_error!("'\\U0000007f", UnclosedChar, 1, 11);
        assert_error!("'\\U0000007f\n", UnclosedChar, 1, 11);
    }

    #[test]
    fn error_invalid_escape() {
        // string literals
        assert_error!("\"\\c\"", InvalidEscape, 1, 3);
        assert_error!("\"\\X\"", InvalidEscape, 1, 3);

        // character code literals
        assert_error!("'\\c'", InvalidEscape, 1, 3);
        assert_error!("'\\X'", InvalidEscape, 1, 3);
    }

    // See docs for LexerErrorKind::UnexpectedEof for more
    #[test]
    fn error_unexpected_eof() {
        assert_error!("\"\\", UnexpectedEof, 1, 2);
        assert_error!("'\\", UnexpectedEof, 1, 2);
    }

    // We technically should account for the cases when bogus bytes are present, but I think this is
    // overkill and won't actually happen in a real setting. See expect() calls and comment on
    // value_from_hex().
    #[test]
    fn error_invalid_hex_escape() {
        // string literals
        assert_error!("\"\\x", InvalidHexEscape, 1, 3);
        assert_error!("\"\\u", InvalidHexEscape, 1, 3);
        assert_error!("\"\\U", InvalidHexEscape, 1, 3);
        assert_error!("\"\\x\"", InvalidHexEscape, 1, 4);
        assert_error!("\"\\u\"", InvalidHexEscape, 1, 4);
        assert_error!("\"\\U\"", InvalidHexEscape, 1, 4);
        assert_error!("\"\\xz\"", InvalidHexEscape, 1, 4);
        assert_error!("\"\\uz\"", InvalidHexEscape, 1, 4);
        assert_error!("\"\\Uz\"", InvalidHexEscape, 1, 4);

        // character code literals
        assert_error!("'\\x", InvalidHexEscape, 1, 3);
        assert_error!("'\\u", InvalidHexEscape, 1, 3);
        assert_error!("'\\U", InvalidHexEscape, 1, 3);
        assert_error!("'\\x'", InvalidHexEscape, 1, 4);
        assert_error!("'\\u'", InvalidHexEscape, 1, 4);
        assert_error!("'\\U'", InvalidHexEscape, 1, 4);
        assert_error!("'\\xz'", InvalidHexEscape, 1, 4);
        assert_error!("'\\uz'", InvalidHexEscape, 1, 4);
        assert_error!("'\\Uz'", InvalidHexEscape, 1, 4);
    }

    #[test]
    fn error_invalid_octal() {
        // The error should point at the offending digit. Squirrel's own diagnostics
        // point at one column before the offending digit, which doesn't make a lot of sense.
        assert_error!("0080", InvalidOctal, 1, 3);
        assert_error!("04079", InvalidOctal, 1, 5);
    }

    #[test]
    fn error_missing_float_exp() {
        assert_error!("0e", MissingFloatExponent, 1, 2);
        assert_error!("0E", MissingFloatExponent, 1, 2);
        assert_error!("1.2e+", MissingFloatExponent, 1, 5);
        assert_error!("1.2e-", MissingFloatExponent, 1, 5);
        assert_error!("7e8..9.0e.", MissingFloatExponent, 1, 10);
        assert_error!("7e8..9.0e+a", MissingFloatExponent, 1, 11);
        assert_error!("7e8..9.0e-Z", MissingFloatExponent, 1, 11);
    }

    #[test]
    fn error_unclosed_multi_line_comment() {
        assert_error!("/*", UnclosedMultiLineComment, 1, 2);
        assert_error!("/* *", UnclosedMultiLineComment, 1, 4);
        assert_error!("/* * /", UnclosedMultiLineComment, 1, 6);
        assert_error!("/*\n", UnclosedMultiLineComment, 2, 1);
        assert_error!("/*\n* * /", UnclosedMultiLineComment, 2, 5);
    }

    #[test]
    fn error_unclosed_verbatim_string() {
        assert_error!("@\"", UnclosedVerbatimString, 1, 2);
        assert_error!("@\"\"\"", UnclosedVerbatimString, 1, 4);
        assert_error!("@\"viele Möglichkeiten", UnclosedVerbatimString, 1, 21);
        assert_error!("@\"\n", UnclosedVerbatimString, 2, 1);
        assert_error!("@\"\n\"\"", UnclosedVerbatimString, 2, 2);
        assert_error!("@\"\nviele Möglichkeiten", UnclosedVerbatimString, 2, 19);
    }
}
