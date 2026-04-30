use rayon;
use similar::{self, DiffableStr, TextDiff};
use std::collections::HashMap;
use std::{fs, path::Path};
use walkdir::WalkDir;

pub fn hash_people(path: &Path) -> HashMap<String, Person> {
    let mut people_hash: HashMap<String, Person> = HashMap::new();
    WalkDir::new(path)
        .max_depth(3)
        .min_depth(3)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.metadata().unwrap().is_file())
        .for_each(|file| {
            let filename = file
                .path()
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned();
            people_hash
                .entry(filename.clone())
                .and_modify(|person| person.add_path(file.path().to_string_lossy().into_owned()))
                .or_insert(Person::new(
                    filename,
                    file.path().to_string_lossy().into_owned(),
                ));
        });
    people_hash
}

pub struct Person {
    filename: String,
    paths: Vec<String>,
}
impl Person {
    fn new(filename: String, path: String) -> Person {
        Person {
            filename: filename,
            paths: vec![path],
        }
    }
    fn add_path(&mut self, path: String) {
        self.paths.push(path);
    }
}
