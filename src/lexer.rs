use lazy_static::lazy_static;
use std::{cell::RefCell, str::Lines, thread_local, vec::IntoIter};
use syntect::{
    easy::HighlightLines,
    highlighting::{Style, Theme},
    parsing::{SyntaxReference, SyntaxSet},
};
use tantivy::tokenizer::{Token, TokenStream, Tokenizer};

lazy_static! {
    static ref SYNTAX_SET: SyntaxSet = SyntaxSet::load_defaults_newlines();
    static ref C_LANGUAGE: SyntaxReference = SYNTAX_SET.find_syntax_by_token("c").unwrap().clone();
    static ref THEME: Theme = Default::default();
}

thread_local! {
    static LINE_PARSER: RefCell<HighlightLines<'static>> =
        RefCell::new(HighlightLines::new(&C_LANGUAGE, &THEME));
}

/// A tokenizer for the C programming language, powered by sublime text syntax highlighting file.
#[derive(Debug, Copy, Clone)]
pub struct CTokenizer;

impl<'a> Tokenizer<'a> for CTokenizer {
    type TokenStreamImpl = CTokenStream<'a>;

    fn token_stream(&self, text: &'a str) -> Self::TokenStreamImpl {
        let raw = text.as_bytes().as_ptr();
        let lines = text.lines();
        let current_line = None;
        let token = Token::default();

        CTokenStream {
            raw,
            lines,
            current_line,
            token,
        }
    }
}

/// A stream of C programming language tokens
#[derive(Debug)]
pub struct CTokenStream<'a> {
    /// The start of the file itself, for token referencing
    raw: *const u8,
    /// The lines in the file (must end with \n)
    lines: Lines<'a>,
    /// The current (parsed) line of the file
    current_line: Option<IntoIter<(Style, &'a str)>>,
    /// The token currently being investigated
    token: Token,
}

impl<'a> TokenStream for CTokenStream<'a> {
    fn advance(&mut self) -> bool {
        let &mut Self {
            raw,
            ref mut current_line,
            ref mut lines,
            ref mut token,
        } = self;

        loop {
            // try to get the next token on this line
            if let Some((_style, next_token)) = current_line.as_mut().and_then(|line| line.next()) {
                let next_token = next_token.trim();
                token.text = next_token.into();
                token.position = token.position.wrapping_add(1);
                token.offset_from = {
                    let current_pos = next_token.as_bytes().as_ptr() as isize;
                    let base_pos = raw as isize;

                    match current_pos - base_pos {
                        i if i >= 0 => i as usize,
                        _ => unreachable!(),
                    }
                };
                token.offset_to = token.offset_from.wrapping_add(next_token.as_bytes().len());

                return true;
            } else {
                // there is no next token on the current line - fetch the next line and loop around
                *current_line = match lines.next() {
                    Some(line) => LINE_PARSER.with(|parser| {
                        let mut parser = parser.borrow_mut();
                        let symbols = parser.highlight(line, &SYNTAX_SET);
                        Some(symbols.into_iter())
                    }),
                    None => {
                        // we are out of lines
                        return false;
                    }
                };
            }
        }
    }

    fn token(&self) -> &Token {
        &self.token
    }

    fn token_mut(&mut self) -> &mut Token {
        &mut self.token
    }
}
