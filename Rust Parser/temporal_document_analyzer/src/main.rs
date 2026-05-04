use std::path::Path;
use temporal_document_analyzer::*;

fn main() {
    let dir_path = Path::new("/Users/anyll/Documents/My files/  Topics/Work/Work Files/CU Boulder/Andy Martin/Evolutionary Biology/Refined Notebook Code/Examples/Example Folder Structure 1/");
    let database = DatabaseHistory::build(dir_path);
    database.print_changelist();
    //println!("{:?}", hash_people(dir_path))
}
