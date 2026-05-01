use std::path::Path;
use temporal_document_analyzer::*;

use walkdir;

fn main() {
    let dir_path = Path::new("/Users/anyll/Documents/My files/  Topics/Work/Work Files/CU Boulder/Andy Martin/Evolutionary Biology/Refined Notebook Code/Examples/Example Folder Structure 1/");
    hash_people(dir_path)
        .iter()
        .for_each(|(_, x)| x.print_history());
}
