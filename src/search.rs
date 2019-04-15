use crate::lexer::CTokenizer;
use failure::{bail, format_err, Error};
use log::debug;
use std::path::PathBuf;
use structopt::StructOpt;
use tantivy::{
    collector::TopDocs,
    query::{PhraseQuery, Query, TermQuery},
    schema::{IndexRecordOption, Value},
    tokenizer::{TokenStream, Tokenizer},
    Index, SnippetGenerator, Term,
};
use termion::color::{Blue, Fg, Red, Reset};

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
    let index = Index::open_in_dir(opts.index_dir)?;
    index.tokenizers().register("c", CTokenizer);

    let schema = index.schema();
    let file_name = schema
        .get_field("file_name")
        .ok_or_else(|| format_err!("Cannot find field 'file_name' in index"))?;
    let file_contents = schema
        .get_field("file_contents")
        .ok_or_else(|| format_err!("Cannot find field 'file_contents' in index"))?;

    let tokens = {
        let mut tokens = Vec::new();
        let mut stream = CTokenizer.token_stream(opts.query.as_str());
        stream.process(&mut |token| tokens.push(token.text.clone()));
        tokens
    };

    let terms = tokens
        .iter()
        .inspect(|text| debug!("Token {:?}", text))
        .map(|text| Term::from_field_text(file_contents, text.as_str()))
        .collect::<Vec<_>>();

    let query = match &terms[..] {
        [single_term] => Box::new(TermQuery::new(
            single_term.clone(),
            IndexRecordOption::WithFreqsAndPositions,
        )) as Box<dyn Query>,
        multiple_terms => Box::new(PhraseQuery::new(multiple_terms.to_vec())) as Box<dyn Query>,
    };

    let searcher = index.reader()?.searcher();
    let highlighting = SnippetGenerator::create(&searcher, &query, file_contents)?;
    let results: Vec<_> = searcher.search(&query, &TopDocs::with_limit(opts.limit))?;

    for (_score, result) in results {
        let doc = searcher.doc(result)?;
        let file_name = {
            let contents = doc
                .get_first(file_name)
                .ok_or_else(|| format_err!("No file name"))?;
            match contents {
                Value::Str(text) => text,
                val => bail!("Invalid value for 'file_name' {:?}", val),
            }
        };
        let snippet = highlighting.snippet_from_doc(&doc);

        let text = snippet.fragments();
        let mut last = 0;
        println!("{}{}{}", Fg(Blue), file_name, Fg(Reset));
        for item in snippet.highlighted() {
            let (start, stop) = item.bounds();
            print!("{}", &text[last..start]);
            print!("{}{}{}", Fg(Red), &text[start..stop], Fg(Reset));
            last = stop;
        }
        println!();
    }

    Ok(())
}
