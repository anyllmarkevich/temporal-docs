use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use temporal_docx::{text_edits::SaveType, *};

fn main() -> Result<()> {
    let args = Cli::parse();
    println!(
        "Seeking data at: {:?}\nOutputting data to: {:?}",
        args.input_path, args.output_path
    );
    let database = DatabaseHistory::build(&args.input_path)?;
    //database.print_changelist(&SaveType::WordAdditions);
    database.save(&args.output_path);
    println!("Done!");
    Ok(())
}

#[derive(Parser)]
struct Cli {
    input_path: PathBuf,
    output_path: PathBuf,
}
