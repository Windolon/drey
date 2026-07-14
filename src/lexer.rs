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

/// Parsed tokens - the main output of [`lex()`].
#[derive(Debug, PartialEq)]
pub struct Token {
    /// The kind of this token.
    pub kind: TokenKind,
    /// Line and column numbers that this token starts at.
    pub start: Position,
    /// Line and column numbers that this token ends at.
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
    Ident(String),

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
}

/// An error returned by [`lex()`] when a part of the input string cannot be lexed.
///
/// After reporting the problematic part of the string via this error,
/// the lexer continues down the input and does not terminate.
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
    /// An invalid token was encountered.
    InvalidToken(String),
}

/// Creates an iterator that produces [`Token`]s wrapped behind
/// [`Result`]s from an input source string.
pub fn lex(source: &str) -> impl Iterator<Item = Result<Token, LexerError>> {
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

    fn previous_byte(&self) -> Option<u8> {
        self.source.get(self.index - 1).copied()
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

    fn create_on_line(
        &self,
        kind: TokenKind,
        start_column: usize,
    ) -> Option<Result<Token, LexerError>> {
        Some(Ok(Token::new(
            kind,
            Position::new(self.line, start_column),
            Position::new(self.line, self.column - 1),
        )))
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

            // /, /=, line comment and block comment.
            b'/' => self.slash(),

            // Line comment beginning with a #.
            b'#' => todo!(),

            // @ and verbatim strings.
            b'@' => self.at(),

            // +, +=, ++,
            // -, -= and --.
            b'+' | b'-' => self.plus_or_minus(),

            // *, %, !, =,
            // *=, %=, != and ==.
            b'*' | b'%' | b'!' | b'=' => self.symbols_with_eq_only(),

            // &, |, :,
            // &&, || and ::.
            b'&' | b'|' | b':' => self.symbols_with_repeat_only(),

            // <, <<, <-, <=, <=> and </.
            b'<' => self.less_than(),

            // >, >>, >>>, >= and />.
            b'>' => self.greater_than(),

            // "." and "...".
            b'.' => self.dot(),

            // Whitespaces.
            // Exact definition for whitespaces in Squirrel can be found at
            // https://github.com/albertodemichelis/squirrel/blob/f9267f2f2/squirrel/sqlexer.cpp#L133
            b' ' | b'\t' | b'\r' => self.whitespace(),

            // Newline.
            b'\n' => self.newline(),

            // Tokens consisting of only one symbol.
            b'^' => self.single_symbol_token(BitXor),
            b'~' => self.single_symbol_token(BitNot),
            b',' => self.single_symbol_token(Comma),
            b'?' => self.single_symbol_token(Question),
            b'(' => self.single_symbol_token(ParenOpen),
            b')' => self.single_symbol_token(ParenClose),
            b'[' => self.single_symbol_token(SquareOpen),
            b']' => self.single_symbol_token(SquareClose),
            b'{' => self.single_symbol_token(BraceOpen),
            b'}' => self.single_symbol_token(BraceClose),
            b';' => self.single_symbol_token(Semicolon),

            // Unexpected byte. Advance until the next lexable byte and
            // report the offender to the user.
            _ => self.unexpected(),
        }
    }

    fn idents_and_keywords(&mut self) -> Option<Result<Token, LexerError>> {
        let start_index = self.index;
        let start_column = self.column;

        while let Some(b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_') = self.next_byte(true) {}

        let value = self.string_from(start_index);

        // This keyword lookup is from Inko, and it is likely as efficient as it gets without being
        // too complex.
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

        self.create_on_line(kind, start_column)
    }

    fn single_symbol_token(&mut self, kind: TokenKind) -> Option<Result<Token, LexerError>> {
        let start_column = self.column;
        self.next_byte(true);
        self.create_on_line(kind, start_column)
    }

    fn slash(&mut self) -> Option<Result<Token, LexerError>> {
        match self.next_byte(false) {
            Some(b'*') => todo!(),

            Some(b'/') => todo!(),

            Some(b'=') => self.slash_token_with_two_columns(DivEq),

            Some(b'>') => self.slash_token_with_two_columns(AttrClose),

            _ => {
                let start_column = self.column;
                self.column += 1;
                self.create_on_line(Div, start_column)
            }
        }
    }

    fn slash_token_with_two_columns(
        &mut self,
        kind: TokenKind,
    ) -> Option<Result<Token, LexerError>> {
        let start_column = self.column;
        self.next_byte(false);
        self.column += 2;
        self.create_on_line(kind, start_column)
    }

    fn at(&mut self) -> Option<Result<Token, LexerError>> {
        let start_column = self.column;

        match self.next_byte(false) {
            Some(b'"') => todo!(),
            _ => {
                self.column += 1;
                self.create_on_line(At, start_column)
            }
        }
    }

    fn plus_or_minus(&mut self) -> Option<Result<Token, LexerError>> {
        let start_column = self.column;

        let kind = match self.current_byte()? {
            b'+' => match self.next_byte(true) {
                Some(b'=') => PlusEq,
                Some(b'+') => PlusPlus,
                _ => Plus,
            },

            b'-' => match self.next_byte(true) {
                Some(b'=') => MinusEq,
                Some(b'-') => MinusMinus,
                _ => Minus,
            },

            _ => {
                unreachable!("plus_or_minus() called to lex tokens that do not start with + or -")
            }
        };

        if kind != Plus && kind != Minus {
            self.next_byte(true);
        }

        self.create_on_line(kind, start_column)
    }

    fn symbols_with_eq_only(&mut self) -> Option<Result<Token, LexerError>> {
        let start_column = self.column;
        let is_eq_kind = matches!(self.next_byte(true), Some(b'='));

        let kind = match self.previous_byte() {
            Some(b'*') if is_eq_kind => MultEq,
            Some(b'*') => Mult,
            Some(b'%') if is_eq_kind => ModEq,
            Some(b'%') => Mod,
            Some(b'!') if is_eq_kind => Ne,
            Some(b'!') => Not,
            Some(b'=') if is_eq_kind => EqEq,
            Some(b'=') => Eq,
            _ => unreachable!(
                "symbols_with_eq_only() called to lex tokens that do not start with *, %, ! or ="
            ),
        };

        if is_eq_kind {
            self.next_byte(true);
        }

        self.create_on_line(kind, start_column)
    }

    fn symbols_with_repeat_only(&mut self) -> Option<Result<Token, LexerError>> {
        let start_byte = self.current_byte();
        let start_column = self.column;
        let is_repeat = self.next_byte(true) == start_byte;

        let kind = match start_byte {
            Some(b'&') if is_repeat => And,
            Some(b'&') => BitAnd,
            Some(b'|') if is_repeat => Or,
            Some(b'|') => BitOr,
            Some(b':') if is_repeat => Scope,
            Some(b':') => Colon,
            _ => unreachable!(
                "symbols_with_repeat_only() called to lex tokens that do not start with &, | or :"
            ),
        };

        if is_repeat {
            self.next_byte(true);
        }

        self.create_on_line(kind, start_column)
    }

    fn less_than(&mut self) -> Option<Result<Token, LexerError>> {
        let start_column = self.column;

        let kind = match self.next_byte(true) {
            Some(b'<') => ShiftLeft,
            Some(b'-') => Newslot,
            Some(b'/') => AttrOpen,
            Some(b'=') => match self.next_byte(true) {
                Some(b'>') => Spaceship,
                _ => Le,
            },
            _ => Lt,
        };

        if kind != Lt && kind != Le {
            self.next_byte(true);
        }

        self.create_on_line(kind, start_column)
    }

    fn greater_than(&mut self) -> Option<Result<Token, LexerError>> {
        let start_column = self.column;

        let kind = match self.next_byte(true) {
            Some(b'=') => Ge,
            Some(b'>') => match self.next_byte(true) {
                Some(b'>') => UShiftRight,
                _ => ShiftRight,
            },
            _ => Gt,
        };

        if kind != Gt && kind != ShiftRight {
            self.next_byte(true);
        }

        self.create_on_line(kind, start_column)
    }

    fn dot(&mut self) -> Option<Result<Token, LexerError>> {
        let start_column = self.column;

        let kind = match self.next_byte(true) {
            Some(b'.') => match self.next_byte(true) {
                Some(b'.') => DotDotDot,
                // ".." is an invalid token and results in a lexer error.
                // https://github.com/albertodemichelis/squirrel/blob/f9267f2f2/squirrel/sqlexer.cpp#L226
                _ => return self.error_on_line(InvalidToken("..".into()), start_column),
            },
            _ => Dot,
        };

        if kind != Dot {
            self.next_byte(true);
        }

        self.create_on_line(kind, start_column)
    }

    fn whitespace(&mut self) -> Option<Result<Token, LexerError>> {
        todo!()
    }

    fn newline(&mut self) -> Option<Result<Token, LexerError>> {
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
            let vec_source: Vec<_> = lex($source).collect();
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
        let mut tokens = lex("");
        assert_eq!(tokens.next(), None);
        assert_eq!(tokens.next(), None);
    }

    #[test]
    #[rustfmt::skip]
    fn idents() {
        // Unused variable
        assert_stream_eq!("_", token(Ident("_".into()), (1, 1), (1, 1)));
        assert_stream_eq!("f", token(Ident("f".into()), (1, 1), (1, 1)));
        assert_stream_eq!("F", token(Ident("F".into()), (1, 1), (1, 1)));
        assert_stream_eq!("f1", token(Ident("f1".into()), (1, 1), (1, 2)));
        assert_stream_eq!("_1", token(Ident("_1".into()), (1, 1), (1, 2)));
        assert_stream_eq!("__", token(Ident("__".into()), (1, 1), (1, 2)));
        // General variable
        assert_stream_eq!("foo", token(Ident("foo".into()), (1, 1), (1, 3)));
        assert_stream_eq!("__fo", token(Ident("__fo".into()), (1, 1), (1, 4)));
        assert_stream_eq!("__2fo", token(Ident("__2fo".into()), (1, 1), (1, 5)));
        // PascalCase
        assert_stream_eq!("FooBar", token(Ident("FooBar".into()), (1, 1), (1, 6)));
        assert_stream_eq!("fOo2BaR", token(Ident("fOo2BaR".into()), (1, 1), (1, 7)));
        // camelCase
        assert_stream_eq!("fooBarBa", token(Ident("fooBarBa".into()), (1, 1), (1, 8)));
        // SCREAMING_SNAKE_CASE
        assert_stream_eq!("HALF_LIFE", token(Ident("HALF_LIFE".into()), (1, 1), (1, 9)));
        // snake_case
        assert_stream_eq!("portal_two", token(Ident("portal_two".into()), (1, 1), (1, 10)));
        // A general script function beginning with "_"
        assert_stream_eq!("__DumpScope", token(Ident("__DumpScope".into()), (1, 1), (1, 11)));
        assert_stream_eq!("__0foobarbaz", token(Ident("__0foobarbaz".into()), (1, 1), (1, 12)));
        assert_stream_eq!("___0123456789", token(Ident("___0123456789".into()), (1, 1), (1, 13)));
    }

    #[test]
    #[rustfmt::skip]
    fn keywords() {
        assert_stream_eq!("base", token(Base, (1, 1), (1, 4)));
        assert_stream_eq!("break", token(Break, (1, 1), (1, 5)));
        assert_stream_eq!("case", token(Case, (1, 1), (1, 4)));
        assert_stream_eq!("catch", token(Catch, (1, 1), (1, 5)));
        assert_stream_eq!("class", token(Class, (1, 1), (1, 5)));
        assert_stream_eq!("clone", token(Clone, (1, 1), (1, 5)));
        assert_stream_eq!("const", token(Const, (1, 1), (1, 5)));
        assert_stream_eq!("constructor", token(Constructor, (1, 1), (1, 11)));
        assert_stream_eq!("continue", token(Continue, (1, 1), (1, 8)));
        assert_stream_eq!("default", token(Default, (1, 1), (1, 7)));
        assert_stream_eq!("delete", token(Delete, (1, 1), (1, 6)));
        assert_stream_eq!("do", token(Do, (1, 1), (1, 2)));
        assert_stream_eq!("else", token(Else, (1, 1), (1, 4)));
        assert_stream_eq!("enum", token(Enum, (1, 1), (1, 4)));
        assert_stream_eq!("extends", token(Extends, (1, 1), (1, 7)));
        assert_stream_eq!("false", token(False, (1, 1), (1, 5)));
        assert_stream_eq!("__FILE__", token(File, (1, 1), (1, 8)));
        assert_stream_eq!("for", token(For, (1, 1), (1, 3)));
        assert_stream_eq!("foreach", token(Foreach, (1, 1), (1, 7)));
        assert_stream_eq!("function", token(Function, (1, 1), (1, 8)));
        assert_stream_eq!("if", token(If, (1, 1), (1, 2)));
        assert_stream_eq!("in", token(In, (1, 1), (1, 2)));
        assert_stream_eq!("instanceof", token(Instanceof, (1, 1), (1, 10)));
        assert_stream_eq!("__LINE__", token(Line, (1, 1), (1, 8)));
        assert_stream_eq!("local", token(Local, (1, 1), (1, 5)));
        assert_stream_eq!("null", token(Null, (1, 1), (1, 4)));
        assert_stream_eq!("rawcall", token(Rawcall, (1, 1), (1, 7)));
        assert_stream_eq!("resume", token(Resume, (1, 1), (1, 6)));
        assert_stream_eq!("return", token(Return, (1, 1), (1, 6)));
        assert_stream_eq!("static", token(Static, (1, 1), (1, 6)));
        assert_stream_eq!("switch", token(Switch, (1, 1), (1, 6)));
        assert_stream_eq!("this", token(This, (1, 1), (1, 4)));
        assert_stream_eq!("throw", token(Throw, (1, 1), (1, 5)));
        assert_stream_eq!("true", token(True, (1, 1), (1, 4)));
        assert_stream_eq!("try", token(Try, (1, 1), (1, 3)));
        assert_stream_eq!("typeof", token(Typeof, (1, 1), (1, 6)));
        assert_stream_eq!("while", token(While, (1, 1), (1, 5)));
        assert_stream_eq!("yield", token(Yield, (1, 1), (1, 5)));
    }

    #[test]
    fn symbols() {
        assert_stream_eq!("+", token(Plus, (1, 1), (1, 1)));
        assert_stream_eq!("+=", token(PlusEq, (1, 1), (1, 2)));
        assert_stream_eq!("++", token(PlusPlus, (1, 1), (1, 2)));
        assert_stream_eq!("-", token(Minus, (1, 1), (1, 1)));
        assert_stream_eq!("-=", token(MinusEq, (1, 1), (1, 2)));
        assert_stream_eq!("--", token(MinusMinus, (1, 1), (1, 2)));
        assert_stream_eq!("*", token(Mult, (1, 1), (1, 1)));
        assert_stream_eq!("*=", token(MultEq, (1, 1), (1, 2)));
        assert_stream_eq!("/", token(Div, (1, 1), (1, 1)));
        assert_stream_eq!("/=", token(DivEq, (1, 1), (1, 2)));
        assert_stream_eq!("%", token(Mod, (1, 1), (1, 1)));
        assert_stream_eq!("%=", token(ModEq, (1, 1), (1, 2)));

        assert_stream_eq!("&", token(BitAnd, (1, 1), (1, 1)));
        assert_stream_eq!("|", token(BitOr, (1, 1), (1, 1)));
        assert_stream_eq!("^", token(BitXor, (1, 1), (1, 1)));
        assert_stream_eq!("~", token(BitNot, (1, 1), (1, 1)));

        assert_stream_eq!("&&", token(And, (1, 1), (1, 2)));
        assert_stream_eq!("||", token(Or, (1, 1), (1, 2)));
        assert_stream_eq!("!", token(Not, (1, 1), (1, 1)));

        assert_stream_eq!("<<", token(ShiftLeft, (1, 1), (1, 2)));
        assert_stream_eq!(">>", token(ShiftRight, (1, 1), (1, 2)));
        assert_stream_eq!(">>>", token(UShiftRight, (1, 1), (1, 3)));

        assert_stream_eq!("<", token(Lt, (1, 1), (1, 1)));
        assert_stream_eq!("<=", token(Le, (1, 1), (1, 2)));
        assert_stream_eq!(">", token(Gt, (1, 1), (1, 1)));
        assert_stream_eq!(">=", token(Ge, (1, 1), (1, 2)));
        assert_stream_eq!("==", token(EqEq, (1, 1), (1, 2)));
        assert_stream_eq!("!=", token(Ne, (1, 1), (1, 2)));
        assert_stream_eq!("<=>", token(Spaceship, (1, 1), (1, 3)));

        assert_stream_eq!("=", token(Eq, (1, 1), (1, 1)));
        assert_stream_eq!("<-", token(Newslot, (1, 1), (1, 2)));
        assert_stream_eq!(",", token(Comma, (1, 1), (1, 1)));
        assert_stream_eq!("?", token(Question, (1, 1), (1, 1)));

        assert_stream_eq!("(", token(ParenOpen, (1, 1), (1, 1)));
        assert_stream_eq!(")", token(ParenClose, (1, 1), (1, 1)));
        assert_stream_eq!("[", token(SquareOpen, (1, 1), (1, 1)));
        assert_stream_eq!("]", token(SquareClose, (1, 1), (1, 1)));
        assert_stream_eq!("{", token(BraceOpen, (1, 1), (1, 1)));
        assert_stream_eq!("}", token(BraceClose, (1, 1), (1, 1)));
        assert_stream_eq!("</", token(AttrOpen, (1, 1), (1, 2)));
        assert_stream_eq!("/>", token(AttrClose, (1, 1), (1, 2)));
        assert_stream_eq!(".", token(Dot, (1, 1), (1, 1)));
        assert_stream_eq!("...", token(DotDotDot, (1, 1), (1, 3)));
        assert_stream_eq!(":", token(Colon, (1, 1), (1, 1)));
        assert_stream_eq!(";", token(Semicolon, (1, 1), (1, 1)));
        assert_stream_eq!("::", token(Scope, (1, 1), (1, 2)));
        assert_stream_eq!("@", token(At, (1, 1), (1, 1)));
    }

    #[test]
    fn unexpected_symbol() {
        assert_stream_eq!("ä", error(UnexpectedSymbol("ä".into()), (1, 1), (1, 1)));
        assert_stream_eq!("äöü", error(UnexpectedSymbol("äöü".into()), (1, 1), (1, 3)));
    }

    #[test]
    fn invalid_token() {
        assert_stream_eq!("..", error(InvalidToken("..".into()), (1, 1), (1, 2)));
    }
}
