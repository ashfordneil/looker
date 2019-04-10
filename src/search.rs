use crate::lexer::CTokenizer;
use failure::{format_err, Error};
use log::debug;
use std::path::PathBuf;
use structopt::StructOpt;
use tantivy::{
    collector::TopDocs,
    tokenizer::{Tokenizer, TokenStream},
    query::{FuzzyTermQuery, PhraseQuery, Query},
    Index, Term,
};

#[derive(Debug, StructOpt)]
pub struct SearchOpts {
    /// The directory that the index is located in.
    #[structopt(long = "index_dir", parse(from_os_str), default_value = ".looker")]
    index_dir: PathBuf,
    /// The maximum number of search results to return
    #[structopt(long = "limit", short = "l", default_value = "3")]
    limit: usize,
    /// The query to search for.
    #[structopt(subcommand)]
    search_term: SearchType,
}

#[derive(Debug, StructOpt)]
enum SearchType {
    /// Search by file name
    #[structopt(name = "file-name")]
    FileName{ name: String },
    /// Search by file contents
    #[structopt(name = "contents")]
    Contents{ query: String },
}

pub fn search_index(opts: SearchOpts) -> Result<(), Error> {
    let index = Index::open_in_dir(opts.index_dir)?;
    let schema = index.schema();

    let query = {
        match opts.search_term {
            SearchType::FileName{ name } => {
                let field = schema
                    .get_field("file_name")
                    .ok_or_else(|| format_err!("Cannot find field 'file_name' in index"))?;
                let term = Term::from_field_text(field, name.as_str());
                let query = FuzzyTermQuery::new(term, 2, false);
                Box::new(query) as Box<dyn Query>
            }
            SearchType::Contents{ query } => {
                index.tokenizers().register("c", CTokenizer);
                let field = schema
                    .get_field("file_contents")
                    .ok_or_else(|| format_err!("Cannot find field 'file_contents' in index"))?;

                let tokens = {
                    let mut tokens = Vec::new();
                    let mut stream = CTokenizer.token_stream(query.as_str());
                    while stream.advance() {
                        tokens.push(stream.token().text.clone());
                    }

                    tokens
                };

                let terms = tokens
                    .iter()
                    .inspect(|text| debug!("Token {:?}", text))
                    .map(|text| Term::from_field_text(field, text.as_str()))
                    .collect::<Vec<_>>();
                let query = PhraseQuery::new(terms);
                Box::new(query) as Box<dyn Query>
            }
        }
    };

    let searcher = index.reader()?.searcher();
    let results: Vec<_> = searcher.search(&query, &TopDocs::with_limit(opts.limit))?;

    for (_score, result) in results {
        let doc = searcher.doc(result)?;
        println!("{}", schema.to_json(&doc));
    }

    Ok(())
}
