use clap::Parser;
use std::path::{Path, PathBuf};
use temporal_document_analyzer::*;

fn main() {
    let args = Cli::parse();
    println!(
        "Seeking data at: {:?}\nOutputting data to: {:?}",
        args.input_path, args.output_path
    );

    //let dir_path = Path::new("/Users/anyll/Documents/My files/  Topics/Work/Work Files/CU Boulder/Andy Martin/Evolutionary Biology/Refined Notebook Code/Examples/Example Folder Structure 1/");
    //let save_path = Path::new("/Users/anyll/Documents/My files/  Topics/Work/Work Files/CU Boulder/Andy Martin/Evolutionary Biology/Refined Notebook Code/Outputs/Output Tests Generation 1/Test 1/");
    //let database = DatabaseHistory::build(dir_path);
    //database.print_changelist();
    //database.save(save_path);

    let database = DatabaseHistory::build(&args.input_path);
    database.print_changelist();
    database.save(&args.output_path);

    //println!("{:?}", hash_people(dir_path))
}

#[derive(Parser)]
struct Cli {
    input_path: PathBuf,
    output_path: PathBuf,
}
