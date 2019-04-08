use failure::Error;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct SearchOpts {
    /// The directory that the index is located in.
    #[structopt(long = "index_dir", parse(from_os_str), default_value = ".looker")]
    index_dir: PathBuf,
    /// The query to search for.
    search_term: String,
}

pub fn search_index(opts: SearchOpts) -> Result<(), Error> {
    unimplemented!()
}
