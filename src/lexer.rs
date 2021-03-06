use lazy_static::lazy_static;
use log::warn;
use regex::{Regex, RegexBuilder, RegexSet, RegexSetBuilder};
use std::str;
use tantivy::tokenizer::{Token, TokenStream, Tokenizer};

lazy_static! {
    static ref REGULAR_EXPRESSIONS: &'static [&'static str] = &[
        // comments
        r"^/\*([^*]|\*[^/])*\*/",
        r"^//([^\n\\]*\\\n)*[^\n]*\n",
        // quotes
        r#"^"([^"]|\\")*""#,
        r"^'(\\?[^'\n]|\\')'",
        // preprocessor
        r"^#(\S*)",
        r"^<[^>]+>", // for #include
        // parens
        r"^[()\[\]{}]",
        // operators
        r"^(->|<<|>>|\|\||&&|--|\+\+|[-+*|&%/=]=)",
        r"^[-<>~!%^&*/+=?|.,:;]",
        // identifier
        r"^[_A-Za-z]\w*",
        // constants
        r"^[0-9]*\.?[0-9]+([eE][-+]?[0-9]+)?",
        // whitespace
        r"^\s+",
    ];
    static ref COMPILED_REGULAR_EXPRESSIONS: Vec<Regex> = REGULAR_EXPRESSIONS
        .iter()
        .map(|regex| {
            RegexBuilder::new(regex)
                .dot_matches_new_line(true)
                .build()
                .unwrap()
        })
        .collect();
    static ref COMPILED_RECOVERY_REGULAR_EXPRESSIONS: Vec<Regex> = REGULAR_EXPRESSIONS
        .iter()
        .map(|regex| {
            RegexBuilder::new(regex)
                .dot_matches_new_line(true)
                .multi_line(true)
                .build()
                .unwrap()
        })
        .collect();
    static ref REGEX_SET: RegexSet = RegexSetBuilder::new(&REGULAR_EXPRESSIONS[..])
        .dot_matches_new_line(true)
        .build()
        .unwrap();
    static ref REGEX_SET_RECOVERY: RegexSet = RegexSetBuilder::new(&REGULAR_EXPRESSIONS[..])
        .dot_matches_new_line(true)
        .multi_line(true)
        .build()
        .unwrap();
}

/// A tokenizer for the C programming language, powered by sublime text syntax highlighting file.
#[derive(Debug, Copy, Clone)]
pub struct CTokenizer;

impl<'a> Tokenizer<'a> for CTokenizer {
    type TokenStreamImpl = CTokenStream<'a>;

    fn token_stream(&self, text: &'a str) -> Self::TokenStreamImpl {
        let token = Token::default();
        let elapsed = 0;

        CTokenStream {
            text,
            token,
            elapsed,
        }
    }
}

/// A stream of C programming language tokens
#[derive(Debug)]
pub struct CTokenStream<'a> {
    /// The start of the file itself, for token referencing
    text: &'a str,
    /// The token currently being investigated
    token: Token,
    /// The amount of characters currently consumed
    elapsed: usize,
}

impl<'a> Iterator for CTokenStream<'a> {
    // start, stop
    type Item = (usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        let token = <Self as TokenStream>::next(self)?;
        Some((token.offset_from, token.offset_to))
    }
}

impl<'a> TokenStream for CTokenStream<'a> {
    fn advance(&mut self) -> bool {
        loop {
            let &mut Self {
                text,
                ref mut token,
                ..
            } = self;

            let position = {
                // try to get the next token on this line
                match &REGEX_SET.matches(text).iter().collect::<Vec<_>>()[..] {
                    [single_regex] => COMPILED_REGULAR_EXPRESSIONS[*single_regex]
                        .find(text)
                        .unwrap(),
                    [] => {
                        if text != "" {
                            warn!("Aborting lex");
                        }
                        return false;
                    }
                    multiple_matches => multiple_matches
                        .into_iter()
                        .map(|&index| COMPILED_REGULAR_EXPRESSIONS[index].find(text).unwrap())
                        .max_by_key(|position| position.end() - position.start())
                        .unwrap(),
                }
            };

            self.text = str::from_utf8(&text.as_bytes()[position.end()..]).unwrap();

            token.offset_from = self.elapsed;
            self.elapsed += position.end();
            token.offset_to = self.elapsed;

            if position.as_str().trim() != "" {
                if token.offset_to - token.offset_from > (1 << 16) {
                    continue;
                }

                token.text = position.as_str().trim().into();
                token.position = token.position.wrapping_add(1);

                return true;
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
