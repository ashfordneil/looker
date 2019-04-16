use crate::{highlight::Highlighter, lexer::CTokenizer};
use failure::{bail, format_err, Error};
use log::debug;
use std::{collections::BTreeMap, iter, path::PathBuf};
use structopt::StructOpt;
use tantivy::{
    collector::TopDocs,
    query::{PhraseQuery, Query, TermQuery},
    schema::{IndexRecordOption, Value},
    tokenizer::{TokenStream, Tokenizer},
    Index, Term,
};
use termion::color::{Blue, Fg, Red, Reset};
use itertools::Itertools;

#[derive(Debug, StructOpt)]
pub struct SearchOpts {
    /// The directory that the index is located in.
    #[structopt(long = "index-dir", parse(from_os_str), default_value = ".looker")]
    index_dir: PathBuf,
    /// The maximum number of search results to return
    #[structopt(long = "limit", short = "l", default_value = "3")]
    limit: usize,
    /// The phrase to search for.
    query: String,
}

pub fn search_index(opts: SearchOpts) -> Result<(), Error> {
    let SearchOpts {
        index_dir,
        query: query_string,
        limit,
    } = opts;
    let index = Index::open_in_dir(index_dir)?;
    index.tokenizers().register("c", CTokenizer);

    let schema = index.schema();
    let file_name = schema
        .get_field("file_name")
        .ok_or_else(|| format_err!("Cannot find field 'file_name' in index"))?;
    let file_contents = schema
        .get_field("file_contents")
        .ok_or_else(|| format_err!("Cannot find field 'file_contents' in index"))?;

    let tokens = CTokenizer.token_stream(query_string.as_str());

    let mut terms = tokens
        // convert to strings
        .map(|(start, stop)| &query_string[start..stop])
        .inspect(|text| debug!("Token {:?}", text))
        // convert to terms
        .map(|text| Term::from_field_text(file_contents, text))
        .collect::<Vec<_>>();

    let query = if terms.len() == 1 {
        let term = terms.remove(0);
        Box::new(TermQuery::new(
            term,
            IndexRecordOption::WithFreqsAndPositions,
        )) as Box<dyn Query>
    } else {
        Box::new(PhraseQuery::new(terms)) as Box<dyn Query>
    };

    let searcher = index.reader()?.searcher();
    let results: Vec<_> = searcher.search(&query, &TopDocs::with_limit(limit))?;

    let highlighter = Highlighter::new(&query_string);
    for (_score, result) in results {
        let doc = searcher.doc(result)?;
        let file_name = {
            let contents = doc
                .get_first(file_name)
                .ok_or(format_err!("No file name"))?;
            match contents {
                Value::Str(text) => text,
                val => bail!("Invalid value for 'file_name' {:?}", val),
            }
        };
        let file_contents = {
            let contents = doc
                .get_first(file_contents)
                .ok_or(format_err!("No file contents"))?;
            match contents {
                Value::Str(text) => text,
                val => bail!("Invalid value for 'file_contents' {:?}", val),
            }
        };

        println!("{}{}{}", Fg(Blue), file_name, Fg(Reset));
        print_lines(&highlighter, file_contents);
    }

    Ok(())
}

fn print_lines(highlighter: &Highlighter, contents: &str) {
    let matches = highlighter.search(contents);
    let points_of_interest = matches
        .iter()
        .enumerate()
        .flat_map(|(index, (start, stop))| {
            iter::once((*start, index)).chain(iter::once((*stop, index)))
        })
        .collect::<BTreeMap<_, _>>();

    let mut currently_inside_pattern = false;

    'line: for line in contents.lines() {
        let mut printed_anything_this_line = false;

        let start = (line.as_ptr() as usize) - (contents.as_ptr() as usize);
        let stop = start + line.len();

        let mut relevant_matches = points_of_interest
            // get the relevant matches that start or stop inside this line
            .range(start..stop)
            .map(|(_position, this_match)| this_match)
            // turn index into (start, stop)
            .map(|index| matches[*index])
            // get ready for iterating through
            .sorted()
            .dedup();

        let mut last_seen = if currently_inside_pattern {
            // find the end of the pattern at the start of this line
            match relevant_matches.next() {
                Some((_start, match_stop)) if match_stop <= stop => {
                    let match_stop = match_stop - start;

                    printed_anything_this_line = true;
                    print!("{}{}", &line[..match_stop], Fg(Reset));

                    match_stop
                }
                _ => {
                    // we are currently inside a pattern
                    // no patterns start or stop on this line
                    // therefore this entire line is just part of the pattern
                    println!("{}", line);
                    continue 'line;
                }
            }
        } else {
            // start from the beginning of this line, as we are not currently inside a pattern
            0
        };
        currently_inside_pattern = false;

        for (match_start, match_stop) in relevant_matches {
            let match_start = match_start - start;

            printed_anything_this_line = true;
            print!("{}", &line[last_seen..match_start]);

            if match_stop > stop {
                currently_inside_pattern = true;
                println!("{}{}", Fg(Red), &line[match_start..]);
                continue 'line;
            } else {
                let match_stop = match_stop - start;
                print!("{}{}{}", Fg(Red), &line[match_start..match_stop], Fg(Reset));
                last_seen = match_stop;
            }
        }

        if printed_anything_this_line {
            println!("{}", &line[last_seen..]);
        }
    }
}
