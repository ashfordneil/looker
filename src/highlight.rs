use crate::lexer::CTokenizer;
use log::info;
use tantivy::tokenizer::Tokenizer;

/// A highlighter that will notice regions of tokenized text that match a query of multiple terms.
pub struct Highlighter<'a> {
    /// The ordered terms of the search phrase
    terms: Vec<&'a str>,
    /// The jump table for a Knuth Morris Pratt search through a document for the phrase.
    leaps: Vec<usize>,
}

impl<'a> Highlighter<'a> {
    /// Create a new highlighter
    pub fn new(terms: &'a str) -> Self {
        let terms: Vec<_> = CTokenizer
            .token_stream(terms)
            .map(|(start, stop)| &terms[start..stop])
            .collect();
        let mut leaps = Vec::new();

        leaps.push(0);

        let mut len = 0;
        for term in &terms {
            let new_leap = loop {
                if term == &terms[len] {
                    len += 1;
                    break len;
                } else if len == 0 {
                    break 0;
                } else {
                    len = leaps[len - 1];
                }
            };
            leaps.push(new_leap);
        }

        Highlighter { terms, leaps }
    }

    /// Use the Knuth Morris Pratt algorithm to search for the search phrase within the input.
    /// Return the (start, stop) indices of each match within the search string.
    pub fn search(&self, input: &str) -> Vec<(usize, usize)> {
        let tokens = CTokenizer.token_stream(input).collect::<Vec<_>>();

        let mut j = 0;
        let mut i = 0;

        let mut output = Vec::new();

        while i < tokens.len() {
            let (start, stop) = tokens[i];
            let token = &input[start..stop];

            if token == self.terms[j] {
                j += 1;
                i += 1;
            }

            if j == self.terms.len() {
                let (start, _) = tokens[i - j];
                output.push((start, stop));
                j = self.leaps[j - 1];
            } else if i < tokens.len() {
                let (start, stop) = tokens[i];
                let token = &input[start..stop];
                if self.terms[j] != token {
                    if j != 0 {
                        j = self.leaps[j - 1];
                    } else {
                        i = i + 1;
                    }
                }
            }
        }

        output
    }
}
