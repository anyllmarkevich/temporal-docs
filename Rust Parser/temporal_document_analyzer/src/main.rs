use anyhow::{anyhow, Result};
use clap::Parser;
use std::path::PathBuf;
use temporal_docx::{text_edits::SaveType, *};

fn main() -> Result<()> {
    let args = Cli::parse();
    if args.output_path.exists() && !args.output_path.read_dir().unwrap().next().is_none() {
        return Err(anyhow!("The ouput path provided already contains files or directories. Executuion has been halted to prevent overriding data contained in the specified output path: {}", args.output_path.to_string_lossy()));
    }
    println!(
        "Seeking data at: {:?}\nOutputting data to: {:?}",
        args.input_path, args.output_path
    );
    let database = DatabaseHistory::build(&args.input_path)?;
    //database.print_changelist(&SaveType::WordAdditions);
    database.save(&args.output_path)?;
    println!("Done!");
    Ok(())
}

#[derive(Parser)]
struct Cli {
    input_path: PathBuf,
    output_path: PathBuf,
}
