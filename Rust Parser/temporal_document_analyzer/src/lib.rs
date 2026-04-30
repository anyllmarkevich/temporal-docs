use rayon::{self, join};
use similar::{self, ChangeTag, DiffableStr, TextDiff};
use std::collections::HashMap;
use std::{fs, path::Path};
use undoc::docx::DocxParser;
use unicode_segmentation::UnicodeSegmentation;
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

#[derive(Debug, Clone)]
struct File_Instance {
    path: String,
    text: String,
}
impl File_Instance {
    fn build(path: String) -> File_Instance {
        File_Instance {
            path: path.clone(),
            text: DocxParser::open(path)
                .expect("Could not open a file.")
                .parse()
                .unwrap()
                .plain_text(),
        }
    }
}

#[derive(Debug)]
pub struct Person {
    filename: String,
    content: Vec<File_Instance>,
}
impl Person {
    fn new(filename: String, path: String) -> Person {
        Person {
            filename: filename,
            content: vec![File_Instance::build(path)],
        }
    }
    fn add_path(&mut self, path: String) {
        self.content.push(File_Instance::build(path));
        self.content.sort_by_key(|s| s.path.clone());
    }
    pub fn find_diffs(&self) -> Vec<String> {
        let mut diffs_vec: Vec<String> = self
            .content
            .iter()
            .map(|instance| {
                instance
                    .text
                    .clone()
                    .unicode_sentences()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>()
                    .join("\n")
            })
            .collect::<Vec<String>>()
            .windows(2)
            .map(|window| {
                TextDiff::from_lines(window[0].clone(), window[1].clone())
                    .iter_all_changes()
                    .filter(|change| change.tag() == ChangeTag::Insert)
                    .map(|change| change.value().to_string())
                    .collect::<Vec<String>>()
                    .join("")
            })
            .collect();
        diffs_vec.insert(0, self.content[0].clone().text);
        diffs_vec
    }
}
