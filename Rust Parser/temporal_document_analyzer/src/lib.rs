use rayon::{self, join};
use similar::{self, ChangeTag, DiffableStr, DiffableStrRef, TextDiff};
use std::collections::HashMap;
use std::{fs, path::Path};
use undoc::docx::DocxParser;
use unicode_segmentation::{UWordBounds, UnicodeSegmentation};
use walkdir::WalkDir;

pub fn hash_people(path: &Path) -> HashMap<String, PersonHistory> {
    let mut people_hash: HashMap<String, NewPerson> = HashMap::new();
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
                .or_insert(NewPerson::build(
                    filename,
                    file.path().to_string_lossy().into_owned(),
                ));
        });
    people_hash
        .into_iter()
        .map(|(k, v)| (k, v.construct_history()))
        .collect()
}

#[derive(Debug, Clone)]
pub struct File_Instance {
    path: String,
    time: String,
    text: String,
}
impl File_Instance {
    fn build(path: String) -> File_Instance {
        File_Instance {
            path: path.clone(),
            time: Path::new(&path)
                .components()
                .nth_back(1)
                .expect("Couldn't find folder name.")
                .as_os_str()
                .to_string_lossy()
                .to_string(),
            text: DocxParser::open(path)
                .expect("Could not open a file.")
                .parse()
                .unwrap()
                .plain_text(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct EditInstance {
    time: String,
    changes: String,
}
impl EditInstance {
    fn new(timeperiod: String, changes: String) -> EditInstance {
        EditInstance {
            time: timeperiod,
            changes: changes,
        }
    }
    pub fn get_edits(&self) -> &String {
        &self.changes
    }
    pub fn get_timeperiod(&self) -> &String {
        &self.time
    }
}

#[derive(Debug)]
struct NewPerson {
    filename: String,
    content: Vec<File_Instance>,
}

impl NewPerson {
    fn build(filename: String, path: String) -> NewPerson {
        NewPerson {
            filename: filename,
            content: vec![File_Instance::build(path)],
        }
    }
    fn add_path(&mut self, path: String) {
        self.content.push(File_Instance::build(path));
        self.content.sort_by_key(|s| s.path.clone());
    }
    pub fn construct_history(self) -> PersonHistory {
        PersonHistory {
            diffmap: self.find_diffs(),
            filename: self.filename,
            content: self.content,
        }
    }
    fn find_diffs(&self) -> Vec<EditInstance> {
        let mut diffs_vec: Vec<EditInstance> = self
            .content
            .iter()
            .map(|instance| instance.text.clone())
            .collect::<Vec<String>>()
            .windows(2)
            .map(|window| {
                TextDiff::from_slices(
                    &window[0]
                        .unicode_sentences()
                        .map(|x| x.as_str().unwrap())
                        .collect::<Vec<&str>>(),
                    &window[1]
                        .unicode_sentences()
                        .map(|x| x.as_str().unwrap())
                        .collect::<Vec<&str>>(),
                )
                .iter_all_changes()
                .filter(|change| change.tag() == ChangeTag::Insert)
                .map(|change: similar::Change<_>| change.value().to_string())
                .collect::<Vec<String>>()
                .join("")
            })
            .zip(
                self.content
                    .iter()
                    .skip(1)
                    .map(|x| x.time.clone())
                    .into_iter()
                    .collect::<Vec<String>>(),
            )
            .map(|(diffs, time)| EditInstance::new(time, diffs))
            .collect();
        diffs_vec.insert(
            0,
            EditInstance::new(self.content[0].time.clone(), self.content[0].text.clone()),
        );
        diffs_vec
    }
}

#[derive(Debug, Clone)]
pub struct PersonHistory {
    diffmap: Vec<EditInstance>,
    filename: String,
    content: Vec<File_Instance>,
}
impl PersonHistory {
    pub fn print_history(&self) {
        println!("———————————————————————————");
        println!("Printing hisotry from filenames \"{}\".", self.filename);
        self.diffmap
            .iter()
            .for_each(|x| println!("For time {}, \"{}\"", x.get_timeperiod(), x.get_edits()));
    }
}
