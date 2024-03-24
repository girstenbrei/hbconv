mod homebank;
mod inputs;

use std::{
    fs::File,
    path::{Path, PathBuf},
};

use clap::{Parser, ValueEnum};
use homebank::Record;
use inputs::{postbank::PostbankIter, sparda::TeoIter};
use miette::{Context, IntoDiagnostic, Result};

/// A conversion tool to produce homebank compatible csv files
#[derive(Parser)]
struct Args {
    #[arg(short, long, env)]
    output: PathBuf,
    input: PathBuf,
    #[arg(short, long, env, value_enum)]
    format: Format,
}

#[derive(Debug, Clone, ValueEnum)]
enum Format {
    Postbank,
    Sparda,
}

impl Format {
    fn open_input(&self, input: &Path) -> Result<RecordIterator> {
        let input = File::open(input)
            .into_diagnostic()
            .wrap_err("Failed opening input file")?;
        match self {
            Format::Postbank => {
                let input = PostbankIter::new(input);
                Ok(RecordIterator::new(Box::new(input.into_iter())))
            }
            Format::Sparda => {
                let input = TeoIter::new(input);
                Ok(RecordIterator::new(Box::new(input.into_iter())))
            }
        }
    }
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Open I/O
    let input = args.format.open_input(&args.input)?;
    let output = File::create(args.output)
        .into_diagnostic()
        .wrap_err("Failed opening output file")?;
    let mut output = Record::writer(output);

    for record in input {
        let hb_record = match record {
            Ok(r) => r,
            Err(err) => {
                eprintln!("{:?}", err);
                continue;
            }
        };
        hb_record.write(&mut output)?;
    }

    output
        .flush()
        .into_diagnostic()
        .wrap_err("Failed flushing output")?;

    Ok(())
}

type RecordIteratorRes = Result<Record>;

struct RecordIterator {
    inner: Box<dyn Iterator<Item = RecordIteratorRes>>,
}

impl RecordIterator {
    fn new(inner: Box<dyn Iterator<Item = RecordIteratorRes>>) -> Self {
        Self { inner }
    }
}

impl Iterator for RecordIterator {
    type Item = RecordIteratorRes;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

trait IntoRecord {
    fn into_record(self) -> Record;
}
