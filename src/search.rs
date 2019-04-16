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

        let matches = highlighter.search(file_contents);
        let points_of_interest = matches
            .iter()
            .enumerate()
            .flat_map(|(index, (start, stop))| {
                iter::once((*start, index)).chain(iter::once((*stop, index)))
            })
            .collect::<BTreeMap<_, _>>();

        println!("{}{}{}", Fg(Blue), file_name, Fg(Reset));
        // find the relevant lines that will need to be printed
        let mut inside_pattern = false;
        for line in file_contents.lines() {
            let start = (line.as_ptr() as usize) - (file_contents.as_ptr() as usize);
            let stop = start + line.len();

            let mut anything_to_print = false;

            let mut relevant_matches = points_of_interest
                // get the matches that start or stop inside this line
                .range(start..stop)
                .map(|(_position, this_match)| this_match)
                // turn index into (start, stop)
                .map(|index| matches[*index])
                // combine these ranges
                .sorted()
                .dedup();

            let mut last_seen = if inside_pattern {
                // there is a match at the start of this line from the previous line
                let (_start, match_stop) = relevant_matches.next().unwrap();
                if match_stop > stop {
                    // the entire line is inside this match
                    println!("{}", line);
                    continue;
                } else {
                    anything_to_print = true;
                    print!("{}{}", &line[..(match_stop - start)], Fg(Reset));
                    match_stop - start
                }
            } else {
                0
            };

            for (match_start, match_stop) in relevant_matches {
                anything_to_print = true;
                let match_start = match_start - start;
                print!("{}", &line[last_seen..match_start]);

                if match_stop > stop {
                    print!("{}{}", Fg(Red), &line[match_start..]);
                    inside_pattern = true;
                    break;
                } else {
                    let match_stop = match_stop - start;
                    print!("{}{}{}", Fg(Red), &line[match_start..match_stop], Fg(Reset));
                    last_seen = match_stop;
                    inside_pattern = false;
                }
            }

            if anything_to_print {
                println!("{}", &line[last_seen..]);
            }
        }
    }

    Ok(())
}
