use failure::Error;
use structopt::StructOpt;

mod build;
mod highlight;
mod lexer;
mod search;

use self::{
    build::{build_index, BuildOpts},
    search::{search_index, SearchOpts},
};

#[derive(Debug, StructOpt)]
#[structopt(name = "looker", about = "code repository search engine")]
enum Options {
    /// Build an index for later searching.
    #[structopt(name = "build")]
    Build(BuildOpts),
    /// Search the existing index (will fail if the index does not exist).
    #[structopt(name = "search")]
    Search(SearchOpts),
}

fn main() -> Result<(), Error> {
    let opts = Options::from_args();
    env_logger::init();

    match opts {
        Options::Build(opts) => build_index(opts),
        Options::Search(opts) => search_index(opts),
    }
}
