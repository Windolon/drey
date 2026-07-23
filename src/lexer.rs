use LexerErrorKind::*;
use TokenKind::*;

use smol_str::SmolStr;
use std::str::CharIndices;

#[derive(Debug, PartialEq)]
pub struct Position {
    pub byte_pos: usize,
    /// The line number of this position.
    pub line: usize,
    /// The column number of this position.
    pub column: usize,
}

impl Position {
    fn new(byte_pos: usize, line: usize, column: usize) -> Self {
        Self {
            byte_pos,
            line,
            column,
        }
    }
}

pub type LResult = Result<Token, LexerError>;

#[derive(Debug, PartialEq)]
pub struct Token {
    /// The kind of this token.
    pub kind: TokenKind,
    pub start: Position,
    pub end: Position,
}

impl Token {
    /// Creates a new `Token` with the specified parameters.
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
    Ident(SmolStr),
    /// A line comment beginning with either `//` or `#`.
    LineComment(SmolStr),

    /// The `base` keyword.
    Base,
    /// The `break` keyword.
    Break,
    /// The `case` keyword.
    Case,
    /// The `catch` keyword.
    Catch,
    /// The `class` keyword.
    Class,
    /// The `clone` keyword.
    Clone,
    /// The `const` keyword.
    Const,
    /// The `constructor` keyword.
    Constructor,
    /// The `continue` keyword.
    Continue,
    /// The `default` keyword.
    Default,
    /// The `delete` keyword.
    Delete,
    /// The `do` keyword.
    Do,
    /// The `else` keyword.
    Else,
    /// The `enum` keyword.
    Enum,
    /// The `extends` keyword.
    Extends,
    /// The `false` keyword.
    False,
    /// The `__FILE__` keyword.
    File,
    /// The `for` keyword.
    For,
    /// The `foreach` keyword.
    Foreach,
    /// The `function` keyword.
    Function,
    /// The `if` keyword.
    If,
    /// The `in` keyword.
    In,
    /// The `instanceof` keyword.
    Instanceof,
    /// The `__LINE__` keyword.
    Line,
    /// The `local` keyword.
    Local,
    /// The `null` keyword.
    Null,
    /// The `rawcall` keyword.
    Rawcall,
    /// The `resume` keyword.
    Resume,
    /// The `return` keyword.
    Return,
    /// The `static` keyword.
    Static,
    /// The `switch` keyword.
    Switch,
    /// The `this` keyword.
    This,
    /// The `throw` keyword.
    Throw,
    /// The `true` keyword.
    True,
    /// The `try` keyword.
    Try,
    /// The `typeof` keyword.
    Typeof,
    /// The `while` keyword.
    While,
    /// The `yield` keyword.
    Yield,

    /// `+`
    Plus,
    /// `+=`
    PlusEq,
    /// `++`
    PlusPlus,
    /// `-`
    Minus,
    /// `-=`
    MinusEq,
    /// `--`
    MinusMinus,
    /// `*`
    Mult,
    /// `*=`
    MultEq,
    /// `/`
    Div,
    /// `/=`
    DivEq,
    /// `%`
    Mod,
    /// `%=`
    ModEq,

    /// `&`
    BitAnd,
    /// `|`
    BitOr,
    /// `^`
    BitXor,
    /// `~`
    BitNot,

    /// `&&`
    And,
    /// `||`
    Or,
    /// `!`
    Not,

    /// `<<`
    ShiftLeft,
    /// `>>`
    ShiftRight,
    /// `>>>`
    UShiftRight,

    /// `<`
    Lt,
    /// `<=`
    Le,
    /// `>`
    Gt,
    /// `>=`
    Ge,
    /// `==`
    EqEq,
    /// `!=`
    Ne,
    /// `<=>`
    Spaceship,

    /// `=`
    Eq,
    /// `<-`
    Newslot,
    /// `,`
    Comma,
    /// `?`
    Question,

    /// `(`
    ParenOpen,
    /// `)`
    ParenClose,
    /// `[`
    SquareOpen,
    /// `]`
    SquareClose,
    /// `{`
    BraceOpen,
    /// `}`
    BraceClose,
    /// `</`
    AttrOpen,
    /// `/>`
    AttrClose,
    /// `.`
    Dot,
    /// `...`
    DotDotDot,
    /// `:`
    Colon,
    /// `;`
    Semicolon,
    /// `::`
    Scope,
    /// `@`
    At,

    /// End of file.
    Eof,
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
    /// An unexpected character was encountered.
    UnexpectedChar(char),
    /// An invalid token was encountered.
    InvalidToken(SmolStr),
}

pub struct Lexer<'src> {
    src: &'src str,
    char_indices: CharIndices<'src>,
    byte_pos: usize,
    line: usize,
    column: usize,
}

impl<'src> Lexer<'src> {
    pub fn new(src: &'src str) -> Self {
        Self {
            src,
            char_indices: src.char_indices(),
            byte_pos: 0,
            line: 1,
            column: 0,
        }
    }

    fn next_char(&mut self) -> Option<char> {
        let (i, c) = self.char_indices.next()?;
        self.column += 1;
        self.byte_pos = i;
        Some(c)
    }

    fn peek_first(&self) -> Option<char> {
        let (_, c) = self.char_indices.clone().next()?;
        Some(c)
    }

    fn peek_second(&self) -> Option<char> {
        let mut iter = self.char_indices.clone();
        // rustc_lexer claims that calling consecutive next()s
        // optimises better than nth(), so let's follow that.
        iter.next();
        let (_, c) = iter.next()?;
        Some(c)
    }

    /// Returns the byte position of the next char from `char_indices`
    /// without advancing the iterator. If iteration has completed, returns
    /// the length in bytes of `src`.
    fn peek_byte_pos(&self) -> usize {
        self.char_indices
            .clone()
            .next()
            .unwrap_or((self.src.len(), '_'))
            .0
    }

    fn str_from_end(&self, start: usize) -> SmolStr {
        // It is an internal error if the indices ever go out of bounds,
        // so we index into the &str directly instead of
        // wrapping the action under get().
        SmolStr::new(&self.src[start..=self.byte_pos])
    }

    fn str_from_end_safe(&self, start: usize) -> SmolStr {
        // In situations like "chloë", "ë" spans byte positions 4 to 5.
        // If we call str_from_end() while the "cursor" sits on the "ë",
        // the str will be malformed since it is made from bytes 0 to 4.
        //
        // Use str_from_end() if you are sure situations like the above
        // will never happen (and do not want the safety check overhead).
        //
        // TODO: make str_from_end() unsafe?
        let end = self.peek_byte_pos();
        SmolStr::new(&self.src[start..end])
    }

    fn token_on_line(&self, kind: TokenKind, byte_pos: usize, column: usize) -> LResult {
        Ok(Token::new(
            kind,
            Position::new(byte_pos, self.line, column),
            Position::new(self.byte_pos, self.line, self.column),
        ))
    }

    fn error_on_line(&self, kind: LexerErrorKind, byte_pos: usize, column: usize) -> LResult {
        Err(LexerError::new(
            kind,
            Position::new(byte_pos, self.line, column),
            Position::new(self.byte_pos, self.line, self.column),
        ))
    }

    fn token_on_line_safe(&self, kind: TokenKind, byte_pos: usize, column: usize) -> LResult {
        let end = self.peek_byte_pos() - 1;
        Ok(Token::new(
            kind,
            Position::new(byte_pos, self.line, column),
            Position::new(end, self.line, self.column),
        ))
    }

    fn one_char_token(&self, kind: TokenKind) -> LResult {
        self.token_on_line(kind, self.byte_pos, self.column)
    }

    fn two_char_token(&mut self, kind: TokenKind) -> LResult {
        let byte_pos = self.byte_pos;
        let column = self.column;
        self.next_char();
        self.token_on_line(kind, byte_pos, column)
    }

    fn three_char_token(&mut self, kind: TokenKind) -> LResult {
        let byte_pos = self.byte_pos;
        let column = self.column;
        self.next_char();
        self.next_char();
        self.token_on_line(kind, byte_pos, column)
    }

    pub fn collect_any(&mut self) -> Vec<LResult> {
        let mut vec = Vec::new();
        let mut pushed_eof = false;
        while !pushed_eof {
            let result = self.next_token();
            if let Ok(token) = &result {
                pushed_eof = token.kind == Eof;
            }
            vec.push(result);
        }
        vec
    }

    pub fn next_token(&mut self) -> LResult {
        // We want the Eof token to point at the very last char
        // in the source string. If that last char is a newline,
        // then the byte position points at it, but
        // line & column info point at the start of the next line.
        let Some(this_char) = self.next_char() else {
            if self.column == 0 {
                self.column = 1;
            }

            let end = if self.src.is_empty() {
                0
            } else {
                self.src.len() - 1
            };

            return Ok(Token::new(
                Eof,
                Position::new(self.byte_pos, self.line, self.column),
                Position::new(end, self.line, self.column),
            ));
        };

        match this_char {
            c if c.is_ascii_alphabetic() || c == '_' => self.idents_and_keywords(),

            // Line comment, block comment, /=, /> and /.
            '/' => match self.peek_first() {
                Some('/') => self.line_comment(),
                Some('*') => todo!(),
                Some('=') => self.two_char_token(DivEq),
                Some('>') => self.two_char_token(AttrClose),
                _ => self.one_char_token(Div),
            },

            // Line comment beginning with a #.
            '#' => self.line_comment(),

            // Verbatim strings and @.
            '@' => match self.peek_first() {
                Some('"') => todo!(),
                _ => self.one_char_token(At),
            },

            // +=, ++ and +.
            '+' => match self.peek_first() {
                Some('=') => self.two_char_token(PlusEq),
                Some('+') => self.two_char_token(PlusPlus),
                _ => self.one_char_token(Plus),
            },

            // -=, -- and -.
            '-' => match self.peek_first() {
                Some('=') => self.two_char_token(MinusEq),
                Some('-') => self.two_char_token(MinusMinus),
                _ => self.one_char_token(Minus),
            },

            // *= and *.
            '*' => match self.peek_first() {
                Some('=') => self.two_char_token(MultEq),
                _ => self.one_char_token(Mult),
            },

            // %= and %.
            '%' => match self.peek_first() {
                Some('=') => self.two_char_token(ModEq),
                _ => self.one_char_token(Mod),
            },

            // != and !.
            '!' => match self.peek_first() {
                Some('=') => self.two_char_token(Ne),
                _ => self.one_char_token(Not),
            },

            // == and =.
            '=' => match self.peek_first() {
                Some('=') => self.two_char_token(EqEq),
                _ => self.one_char_token(Eq),
            },

            // && and &.
            '&' => match self.peek_first() {
                Some('&') => self.two_char_token(And),
                _ => self.one_char_token(BitAnd),
            },

            // || and |.
            '|' => match self.peek_first() {
                Some('|') => self.two_char_token(Or),
                _ => self.one_char_token(BitOr),
            },

            // :: and :.
            ':' => match self.peek_first() {
                Some(':') => self.two_char_token(Scope),
                _ => self.one_char_token(Colon),
            },

            // <=>, <=, <<, <-, </ and <.
            '<' => match self.peek_first() {
                Some('=') => match self.peek_second() {
                    Some('>') => self.three_char_token(Spaceship),
                    _ => self.two_char_token(Le),
                },
                Some('<') => self.two_char_token(ShiftLeft),
                Some('-') => self.two_char_token(Newslot),
                Some('/') => self.two_char_token(AttrOpen),
                _ => self.one_char_token(Lt),
            },

            // >>>, >>, >= and >.
            '>' => match self.peek_first() {
                Some('>') => match self.peek_second() {
                    Some('>') => self.three_char_token(UShiftRight),
                    _ => self.two_char_token(ShiftRight),
                },
                Some('=') => self.two_char_token(Ge),
                _ => self.one_char_token(Gt),
            },

            // "..." and ".".
            '.' => match self.peek_first() {
                Some('.') => match self.peek_second() {
                    Some('.') => self.three_char_token(DotDotDot),
                    _ => {
                        // ".." is an invalid token and results in a lexer error.
                        // https://github.com/albertodemichelis/squirrel/blob/f9267f2f2/squirrel/sqlexer.cpp#L226
                        let byte_pos = self.byte_pos;
                        let column = self.column;
                        self.next_char();
                        // Don't even need to slice src here.
                        self.error_on_line(InvalidToken("..".into()), byte_pos, column)
                    }
                },
                _ => self.one_char_token(Dot),
            },

            // Whitespaces.
            // Exact definition for whitespaces in Squirrel can be found at
            // https://github.com/albertodemichelis/squirrel/blob/f9267f2f2/squirrel/sqlexer.cpp#L133
            ' ' | '\t' | '\r' => self.whitespace(),

            // Newline.
            '\n' => self.newline(),

            // Tokens consisting of only one symbol.
            '^' => self.one_char_token(BitXor),
            '~' => self.one_char_token(BitNot),
            ',' => self.one_char_token(Comma),
            '?' => self.one_char_token(Question),
            '(' => self.one_char_token(ParenOpen),
            ')' => self.one_char_token(ParenClose),
            '[' => self.one_char_token(SquareOpen),
            ']' => self.one_char_token(SquareClose),
            '{' => self.one_char_token(BraceOpen),
            '}' => self.one_char_token(BraceClose),
            ';' => self.one_char_token(Semicolon),

            // Unexpected char. Report this offending char to the user.
            _ => {
                let start_byte_pos = self.byte_pos;
                let end_byte_pos = self.peek_byte_pos() - 1;
                Err(LexerError::new(
                    UnexpectedChar(this_char),
                    Position::new(start_byte_pos, self.line, self.column),
                    Position::new(end_byte_pos, self.line, self.column),
                ))
            }
        }
    }

    fn idents_and_keywords(&mut self) -> LResult {
        let byte_pos = self.byte_pos;
        let column = self.column;

        while let Some(c) = self.peek_first() {
            if !c.is_ascii_alphanumeric() && c != '_' {
                break;
            }
            self.next_char();
        }

        let value = self.str_from_end(byte_pos);
        // This keyword lookup is from Inko, and it is likely as efficient as it gets
        // without being too complex.
        let kind = match value.len() {
            2 => match value.as_str() {
                "do" => Do,
                "if" => If,
                "in" => In,
                _ => Ident(value),
            },
            3 => match value.as_str() {
                "for" => For,
                "try" => Try,
                _ => Ident(value),
            },
            4 => match value.as_str() {
                "base" => Base,
                "case" => Case,
                "else" => Else,
                "enum" => Enum,
                "null" => Null,
                "this" => This,
                "true" => True,
                _ => Ident(value),
            },
            5 => match value.as_str() {
                "break" => Break,
                "catch" => Catch,
                "class" => Class,
                "clone" => Clone,
                "const" => Const,
                "false" => False,
                "local" => Local,
                "throw" => Throw,
                "while" => While,
                "yield" => Yield,
                _ => Ident(value),
            },
            6 => match value.as_str() {
                "delete" => Delete,
                "resume" => Resume,
                "return" => Return,
                "static" => Static,
                "switch" => Switch,
                "typeof" => Typeof,
                _ => Ident(value),
            },
            7 => match value.as_str() {
                "default" => Default,
                "extends" => Extends,
                "foreach" => Foreach,
                "rawcall" => Rawcall,
                _ => Ident(value),
            },
            8 => match value.as_str() {
                "__FILE__" => File,
                "__LINE__" => Line,
                "continue" => Continue,
                "function" => Function,
                _ => Ident(value),
            },
            10 | 11 => match value.as_str() {
                "instanceof" => Instanceof,
                "constructor" => Constructor,
                _ => Ident(value),
            },
            _ => Ident(value),
        };

        self.token_on_line(kind, byte_pos, column)
    }

    fn line_comment(&mut self) -> LResult {
        let byte_pos = self.byte_pos;
        let column = self.column;

        while let Some(c) = self.peek_first() {
            if c == '\n' {
                break;
            }
            self.next_char();
        }

        let text = self.str_from_end_safe(byte_pos);
        self.token_on_line_safe(LineComment(text), byte_pos, column)
    }

    fn whitespace(&mut self) -> LResult {
        todo!()
    }

    fn newline(&mut self) -> LResult {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_stream_eq {
        (
            $src: expr,
            $(
                $result: expr
            ),+
        ) => {{
            let vec_source: Vec<_> = Lexer::new($src).collect_any();
            assert_eq!(vec_source, vec![$($result,)+]);
        }};
    }

    fn token(kind: TokenKind, start: (usize, usize, usize), end: (usize, usize, usize)) -> LResult {
        Ok(Token::new(
            kind,
            Position::new(start.0, start.1, start.2),
            Position::new(end.0, end.1, end.2),
        ))
    }

    fn eof(start: usize, line: usize, column: usize, end: usize) -> LResult {
        token(Eof, (start, line, column), (end, line, column))
    }

    fn error(
        kind: LexerErrorKind,
        start: (usize, usize, usize),
        end: (usize, usize, usize),
    ) -> LResult {
        Err(LexerError::new(
            kind,
            Position::new(start.0, start.1, start.2),
            Position::new(end.0, end.1, end.2),
        ))
    }

    #[test]
    fn empty() {
        let mut lexer = Lexer::new("");
        assert_eq!(lexer.next_token(), eof(0, 1, 1, 0));
        assert_eq!(lexer.next_token(), eof(0, 1, 1, 0));
    }

    #[test]
    #[rustfmt::skip]
    fn idents() {
        // Unused variable
        assert_stream_eq!("_", token(Ident("_".into()), (0, 1, 1), (0, 1, 1)), eof(0, 1, 1, 0));
        assert_stream_eq!("f", token(Ident("f".into()), (0, 1, 1), (0, 1, 1)), eof(0, 1, 1, 0));
        assert_stream_eq!("F", token(Ident("F".into()), (0, 1, 1), (0, 1, 1)), eof(0, 1, 1, 0));
        assert_stream_eq!("f1", token(Ident("f1".into()), (0, 1, 1), (1, 1, 2)), eof(1, 1, 2, 1));
        assert_stream_eq!("_1", token(Ident("_1".into()), (0, 1, 1), (1, 1, 2)), eof(1, 1, 2, 1));
        assert_stream_eq!("__", token(Ident("__".into()), (0, 1, 1), (1, 1, 2)), eof(1, 1, 2, 1));
        // General variable
        assert_stream_eq!("foo", token(Ident("foo".into()), (0, 1, 1), (2, 1, 3)), eof(2, 1, 3, 2));
        assert_stream_eq!("__fo", token(Ident("__fo".into()), (0, 1, 1), (3, 1, 4)), eof(3, 1, 4, 3));
        assert_stream_eq!("__2fo", token(Ident("__2fo".into()), (0, 1, 1), (4, 1, 5)), eof(4, 1, 5, 4));
        // PascalCase
        assert_stream_eq!("FooBar", token(Ident("FooBar".into()), (0, 1, 1), (5, 1, 6)), eof(5, 1, 6, 5));
        assert_stream_eq!("fOo2BaR", token(Ident("fOo2BaR".into()), (0, 1, 1), (6, 1, 7)), eof(6, 1, 7, 6));
        // camelCase
        assert_stream_eq!("fooBarBa", token(Ident("fooBarBa".into()), (0, 1, 1), (7, 1, 8)), eof(7, 1, 8, 7));
        // SCREAMING_SNAKE_CASE
        assert_stream_eq!("HALF_LIFE", token(Ident("HALF_LIFE".into()), (0, 1, 1), (8, 1, 9)), eof(8, 1, 9, 8));
        // snake_case
        assert_stream_eq!("portal_two", token(Ident("portal_two".into()), (0, 1, 1), (9, 1, 10)), eof(9, 1, 10, 9));
        // A general script function beginning with "_"
        assert_stream_eq!("__DumpScope", token(Ident("__DumpScope".into()), (0, 1, 1), (10, 1, 11)), eof(10, 1, 11, 10));
        assert_stream_eq!("__0foobarbaz", token(Ident("__0foobarbaz".into()), (0, 1, 1), (11, 1, 12)), eof(11, 1, 12, 11));
        assert_stream_eq!("___0123456789", token(Ident("___0123456789".into()), (0, 1, 1), (12, 1, 13)), eof(12, 1, 13, 12));
    }

    #[test]
    #[rustfmt::skip]
    fn keywords() {
        assert_stream_eq!("base", token(Base, (0, 1, 1), (3, 1, 4)), eof(3, 1, 4, 3));
        assert_stream_eq!("break", token(Break, (0, 1, 1), (4, 1, 5)), eof(4, 1, 5, 4));
        assert_stream_eq!("case", token(Case, (0, 1, 1), (3, 1, 4)), eof(3, 1, 4, 3));
        assert_stream_eq!("catch", token(Catch, (0, 1, 1), (4, 1, 5)), eof(4, 1, 5, 4));
        assert_stream_eq!("class", token(Class, (0, 1, 1), (4, 1, 5)), eof(4, 1, 5, 4));
        assert_stream_eq!("clone", token(Clone, (0, 1, 1), (4, 1, 5)), eof(4, 1, 5, 4));
        assert_stream_eq!("const", token(Const, (0, 1, 1), (4, 1, 5)), eof(4, 1, 5, 4));
        assert_stream_eq!("constructor", token(Constructor, (0, 1, 1), (10, 1, 11)), eof(10, 1, 11, 10));
        assert_stream_eq!("continue", token(Continue, (0, 1, 1), (7, 1, 8)), eof(7, 1, 8, 7));
        assert_stream_eq!("default", token(Default, (0, 1, 1), (6, 1, 7)), eof(6, 1, 7, 6));
        assert_stream_eq!("delete", token(Delete, (0, 1, 1), (5, 1, 6)), eof(5, 1, 6, 5));
        assert_stream_eq!("do", token(Do, (0, 1, 1), (1, 1, 2)), eof(1, 1, 2, 1));
        assert_stream_eq!("else", token(Else, (0, 1, 1), (3, 1, 4)), eof(3, 1, 4, 3));
        assert_stream_eq!("enum", token(Enum, (0, 1, 1), (3, 1, 4)), eof(3, 1, 4, 3));
        assert_stream_eq!("extends", token(Extends, (0, 1, 1), (6, 1, 7)), eof(6, 1, 7, 6));
        assert_stream_eq!("false", token(False, (0, 1, 1), (4, 1, 5)), eof(4, 1, 5, 4));
        assert_stream_eq!("__FILE__", token(File, (0, 1, 1), (7, 1, 8)), eof(7, 1, 8, 7));
        assert_stream_eq!("for", token(For, (0, 1, 1), (2, 1, 3)), eof(2, 1, 3, 2));
        assert_stream_eq!("foreach", token(Foreach, (0, 1, 1), (6, 1, 7)), eof(6, 1, 7, 6));
        assert_stream_eq!("function", token(Function, (0, 1, 1), (7, 1, 8)), eof(7, 1, 8, 7));
        assert_stream_eq!("if", token(If, (0, 1, 1), (1, 1, 2)), eof(1, 1, 2, 1));
        assert_stream_eq!("in", token(In, (0, 1, 1), (1, 1, 2)), eof(1, 1, 2, 1));
        assert_stream_eq!("instanceof", token(Instanceof, (0, 1, 1), (9, 1, 10)), eof(9, 1, 10, 9));
        assert_stream_eq!("__LINE__", token(Line, (0, 1, 1), (7, 1, 8)), eof(7, 1, 8, 7));
        assert_stream_eq!("local", token(Local, (0, 1, 1), (4, 1, 5)), eof(4, 1, 5, 4));
        assert_stream_eq!("null", token(Null, (0, 1, 1), (3, 1, 4)), eof(3, 1, 4, 3));
        assert_stream_eq!("rawcall", token(Rawcall, (0, 1, 1), (6, 1, 7)), eof(6, 1, 7, 6));
        assert_stream_eq!("resume", token(Resume, (0, 1, 1), (5, 1, 6)), eof(5, 1, 6, 5));
        assert_stream_eq!("return", token(Return, (0, 1, 1), (5, 1, 6)), eof(5, 1, 6, 5));
        assert_stream_eq!("static", token(Static, (0, 1, 1), (5, 1, 6)), eof(5, 1, 6, 5));
        assert_stream_eq!("switch", token(Switch, (0, 1, 1), (5, 1, 6)), eof(5, 1, 6, 5));
        assert_stream_eq!("this", token(This, (0, 1, 1), (3, 1, 4)), eof(3, 1, 4, 3));
        assert_stream_eq!("throw", token(Throw, (0, 1, 1), (4, 1, 5)), eof(4, 1, 5, 4));
        assert_stream_eq!("true", token(True, (0, 1, 1), (3, 1, 4)), eof(3, 1, 4, 3));
        assert_stream_eq!("try", token(Try, (0, 1, 1), (2, 1, 3)), eof(2, 1, 3, 2));
        assert_stream_eq!("typeof", token(Typeof, (0, 1, 1), (5, 1, 6)), eof(5, 1, 6, 5));
        assert_stream_eq!("while", token(While, (0, 1, 1), (4, 1, 5)), eof(4, 1, 5, 4));
        assert_stream_eq!("yield", token(Yield, (0, 1, 1), (4, 1, 5)), eof(4, 1, 5, 4));
    }

    #[test]
    #[rustfmt::skip]
    fn symbols() {
        assert_stream_eq!("+", token(Plus, (0, 1, 1), (0, 1, 1)), eof(0, 1, 1, 0));
        assert_stream_eq!("+=", token(PlusEq, (0, 1, 1), (1, 1, 2)), eof(1, 1, 2, 1));
        assert_stream_eq!("++", token(PlusPlus, (0, 1, 1), (1, 1, 2)), eof(1, 1, 2, 1));
        assert_stream_eq!("-", token(Minus, (0, 1, 1), (0, 1, 1)), eof(0, 1, 1, 0));
        assert_stream_eq!("-=", token(MinusEq, (0, 1, 1), (1, 1, 2)), eof(1, 1, 2, 1));
        assert_stream_eq!("--", token(MinusMinus, (0, 1, 1), (1, 1, 2)), eof(1, 1, 2, 1));
        assert_stream_eq!("*", token(Mult, (0, 1, 1), (0, 1, 1)), eof(0, 1, 1, 0));
        assert_stream_eq!("*=", token(MultEq, (0, 1, 1), (1, 1, 2)), eof(1, 1, 2, 1));
        assert_stream_eq!("/", token(Div, (0, 1, 1), (0, 1, 1)), eof(0, 1, 1, 0));
        assert_stream_eq!("/=", token(DivEq, (0, 1, 1), (1, 1, 2)), eof(1, 1, 2, 1));
        assert_stream_eq!("%", token(Mod, (0, 1, 1), (0, 1, 1)), eof(0, 1, 1, 0));
        assert_stream_eq!("%=", token(ModEq, (0, 1, 1), (1, 1, 2)), eof(1, 1, 2, 1));

        assert_stream_eq!("&", token(BitAnd, (0, 1, 1), (0, 1, 1)), eof(0, 1, 1, 0));
        assert_stream_eq!("|", token(BitOr, (0, 1, 1), (0, 1, 1)), eof(0, 1, 1, 0));
        assert_stream_eq!("^", token(BitXor, (0, 1, 1), (0, 1, 1)), eof(0, 1, 1, 0));
        assert_stream_eq!("~", token(BitNot, (0, 1, 1), (0, 1, 1)), eof(0, 1, 1, 0));

        assert_stream_eq!("&&", token(And, (0, 1, 1), (1, 1, 2)), eof(1, 1, 2, 1));
        assert_stream_eq!("||", token(Or, (0, 1, 1), (1, 1, 2)), eof(1, 1, 2, 1));
        assert_stream_eq!("!", token(Not, (0, 1, 1), (0, 1, 1)), eof(0, 1, 1, 0));

        assert_stream_eq!("<<", token(ShiftLeft, (0, 1, 1), (1, 1, 2)), eof(1, 1, 2, 1));
        assert_stream_eq!(">>", token(ShiftRight, (0, 1, 1), (1, 1, 2)), eof(1, 1, 2, 1));
        assert_stream_eq!(">>>", token(UShiftRight, (0, 1, 1), (2, 1, 3)), eof(2, 1, 3, 2));

        assert_stream_eq!("<", token(Lt, (0, 1, 1), (0, 1, 1)), eof(0, 1, 1, 0));
        assert_stream_eq!("<=", token(Le, (0, 1, 1), (1, 1, 2)), eof(1, 1, 2, 1));
        assert_stream_eq!(">", token(Gt, (0, 1, 1), (0, 1, 1)), eof(0, 1, 1, 0));
        assert_stream_eq!(">=", token(Ge, (0, 1, 1), (1, 1, 2)), eof(1, 1, 2, 1));
        assert_stream_eq!("==", token(EqEq, (0, 1, 1), (1, 1, 2)), eof(1, 1, 2, 1));
        assert_stream_eq!("!=", token(Ne, (0, 1, 1), (1, 1, 2)), eof(1, 1, 2, 1));
        assert_stream_eq!("<=>", token(Spaceship, (0, 1, 1), (2, 1, 3)), eof(2, 1, 3, 2));

        assert_stream_eq!("=", token(Eq, (0, 1, 1), (0, 1, 1)), eof(0, 1, 1, 0));
        assert_stream_eq!("<-", token(Newslot, (0, 1, 1), (1, 1, 2)), eof(1, 1, 2, 1));
        assert_stream_eq!(",", token(Comma, (0, 1, 1), (0, 1, 1)), eof(0, 1, 1, 0));
        assert_stream_eq!("?", token(Question, (0, 1, 1), (0, 1, 1)), eof(0, 1, 1, 0));

        assert_stream_eq!("(", token(ParenOpen, (0, 1, 1), (0, 1, 1)), eof(0, 1, 1, 0));
        assert_stream_eq!(")", token(ParenClose, (0, 1, 1), (0, 1, 1)), eof(0, 1, 1, 0));
        assert_stream_eq!("[", token(SquareOpen, (0, 1, 1), (0, 1, 1)), eof(0, 1, 1, 0));
        assert_stream_eq!("]", token(SquareClose, (0, 1, 1), (0, 1, 1)), eof(0, 1, 1, 0));
        assert_stream_eq!("{", token(BraceOpen, (0, 1, 1), (0, 1, 1)), eof(0, 1, 1, 0));
        assert_stream_eq!("}", token(BraceClose, (0, 1, 1), (0, 1, 1)), eof(0, 1, 1, 0));
        assert_stream_eq!("</", token(AttrOpen, (0, 1, 1), (1, 1, 2)), eof(1, 1, 2, 1));
        assert_stream_eq!("/>", token(AttrClose, (0, 1, 1), (1, 1, 2)), eof(1, 1, 2, 1));
        assert_stream_eq!(".", token(Dot, (0, 1, 1), (0, 1, 1)), eof(0, 1, 1, 0));
        assert_stream_eq!("...", token(DotDotDot, (0, 1, 1), (2, 1, 3)), eof(2, 1, 3, 2));
        assert_stream_eq!(":", token(Colon, (0, 1, 1), (0, 1, 1)), eof(0, 1, 1, 0));
        assert_stream_eq!(";", token(Semicolon, (0, 1, 1), (0, 1, 1)), eof(0, 1, 1, 0));
        assert_stream_eq!("::", token(Scope, (0, 1, 1), (1, 1, 2)), eof(1, 1, 2, 1));
        assert_stream_eq!("@", token(At, (0, 1, 1), (0, 1, 1)), eof(0, 1, 1, 0));
    }

    #[test]
    fn unexpected_symbol() {
        assert_stream_eq!(
            "ä",
            error(UnexpectedChar('ä'), (0, 1, 1), (1, 1, 1)),
            eof(0, 1, 1, 1)
        );
        assert_stream_eq!(
            "ä松🐿",
            // 2 bytes
            error(UnexpectedChar('ä'), (0, 1, 1), (1, 1, 1)),
            // 3 bytes
            error(UnexpectedChar('松'), (2, 1, 2), (4, 1, 2)),
            // 4 bytes
            error(UnexpectedChar('🐿'), (5, 1, 3), (8, 1, 3)),
            eof(5, 1, 3, 8)
        );
    }

    #[test]
    fn invalid_token() {
        assert_stream_eq!(
            "..",
            error(InvalidToken("..".into()), (0, 1, 1), (1, 1, 2)),
            eof(1, 1, 2, 1)
        );
    }

    #[test]
    fn line_comments() {
        assert_stream_eq!(
            "//",
            token(LineComment("//".into()), (0, 1, 1), (1, 1, 2)),
            eof(1, 1, 2, 1)
        );
        assert_stream_eq!(
            "// comment",
            token(LineComment("// comment".into()), (0, 1, 1), (9, 1, 10)),
            eof(9, 1, 10, 9)
        );
        assert_stream_eq!(
            "// chloë",
            token(LineComment("// chloë".into()), (0, 1, 1), (8, 1, 8)),
            eof(7, 1, 8, 8)
        );
        assert_stream_eq!(
            "#",
            token(LineComment("#".into()), (0, 1, 1), (0, 1, 1)),
            eof(0, 1, 1, 0)
        );
        assert_stream_eq!(
            "# comment",
            token(LineComment("# comment".into()), (0, 1, 1), (8, 1, 9)),
            eof(8, 1, 9, 8)
        );
        assert_stream_eq!(
            "# chloë",
            token(LineComment("# chloë".into()), (0, 1, 1), (7, 1, 7)),
            eof(6, 1, 7, 7)
        );
    }
}
