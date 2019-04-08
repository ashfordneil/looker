use failure::Error;
use ignore::Walk;
use log::warn;
use std::{fs, path::PathBuf};
use structopt::StructOpt;
use tantivy::{
    doc,
    schema::{self, Schema},
    Index,
};

/// The build command creates a new tantivy index, from a code repository.
#[derive(Debug, StructOpt)]
pub struct BuildOpts {
    /// The directory to build the index in.
    #[structopt(long = "index_dir", parse(from_os_str), default_value = ".looker")]
    index_dir: PathBuf,
    /// The directory to search for code in.
    #[structopt(parse(from_os_str), default_value = ".")]
    search_dir: PathBuf,
}

/// Create an index for later searching.
pub fn build_index(opts: BuildOpts) -> Result<(), Error> {
    // create the schema
    let mut schema_builder = Schema::builder();
    let file_name = schema_builder.add_text_field("file_name", schema::STRING | schema::STORED);
    let file_contents =
        schema_builder.add_text_field("file_contents", schema::TEXT | schema::STORED);
    let schema = schema_builder.build();

    // create the index
    fs::create_dir_all(&opts.index_dir)?;
    let index = Index::create_in_dir(opts.index_dir, schema)?;
    let mut writer = index.writer(100_000_000)?;

    Walk::new(opts.search_dir)
        .filter_map(|file| match file {
            Ok(file) => Some(file),
            Err(error) => {
                warn!("Walking directory: {:?}", error);
                None
            }
        })
        .filter(|file| {
            if let Some(file_type) = file.file_type() {
                file_type.is_file()
            } else {
                false
            }
        })
        .for_each(|file| {
            let path = file.into_path();
            let contents = match fs::read_to_string(&path) {
                Ok(contents) => contents,
                Err(error) => {
                    warn!("Reading file: {:?}", error);
                    return;
                }
            };
            let name = path.into_os_string().to_string_lossy().to_string();

            writer.add_document(doc! {
                file_name => name,
                file_contents => contents,
            });
        });

    writer.commit()?;

    Ok(())
}
