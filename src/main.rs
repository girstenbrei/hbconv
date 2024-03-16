mod homebank;
mod inputs;

use std::{fs::File, path::PathBuf};

use clap::Parser;
use homebank::Record;
use inputs::postbank::PostbankIter;
use miette::{Context, IntoDiagnostic, Result};

/// A conversion tool to produce homebank compatible csv files
#[derive(Parser)]
struct Args {
    #[arg(short, long, env)]
    output: PathBuf,
    input: PathBuf,
    // #[arg(short, long, env, value_enum)]
    // format: Loader,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let input = File::open(args.input)
        .into_diagnostic()
        .wrap_err("Failed opening input file")?;
    let input = PostbankIter::new(input);

    let output = File::create(args.output)
        .into_diagnostic()
        .wrap_err("Failed opening output file")?;
    let mut output = Record::writer(output);

    for record in input {
        let record = match record {
            Ok(r) => r,
            Err(err) => {
                eprintln!("{:?}", err);
                continue;
            }
        };

        let hb_record: Record = record.into();
        hb_record.write(&mut output)?;
    }

    output
        .flush()
        .into_diagnostic()
        .wrap_err("Failed flushing output")?;

    Ok(())
}
