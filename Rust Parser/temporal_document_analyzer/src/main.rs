use std::path::Path;
use temporal_document_analyzer::*;

use walkdir;

fn main() {
    let dir_path = Path::new("/Users/anyll/Documents/My files/  Topics/Work/Work Files/CU Boulder/Andy Martin/Evolutionary Biology/Refined Notebook Code/Examples/Example Folder Structure 1/");
    //println!("{:?}", hash_people(dir_path))
    println!(
        "{:?}",
        hash_people(dir_path)
            .values()
            .map(|x| x.find_diffs())
            .collect::<Vec<Vec<String>>>()
    )
}
