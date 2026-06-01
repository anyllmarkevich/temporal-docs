use clap::Parser;
use std::path::{Path, PathBuf};
use temporal_document_analyzer::*;

fn main() {
    let args = Cli::parse();
    println!(
        "Seeking data at: {:?}\nOutputting data to: {:?}",
        args.input_path, args.output_path
    );
    let database = DatabaseHistory::build(&args.input_path);
    database.print_changelist();
    database.save(&args.output_path);
}

#[derive(Parser)]
struct Cli {
    input_path: PathBuf,
    output_path: PathBuf,
}
