use core::panic;
use csv;
use rayon::{self, join};
use serde::Serialize;
use similar::{self, ChangeTag, DiffableStr, DiffableStrRef, TextDiff};
use std::collections::HashMap;
use std::error::Error;
use std::fmt::format;
use std::fs::File;
use std::io::Write;
use std::{
    fs,
    path::{Path, PathBuf},
};
use undoc::docx::DocxParser;
use unicode_segmentation::{UWordBounds, UnicodeSegmentation};
use walkdir::WalkDir;

pub struct DatabaseHistory {
    hash: HashMap<String, PersonHistory>,
}
impl DatabaseHistory {
    pub fn build(path: &Path) -> Result<DatabaseHistory, Box<dyn Error>> {
        let hash = Self::hash_people(path)?;
        Ok(DatabaseHistory { hash: hash })
    }

    pub fn print_changelist(&self) {
        self.hash.iter().for_each(|(_, x)| x.print_history());
    }

    fn hash_people(path: &Path) -> Result<HashMap<String, PersonHistory>, Box<dyn Error>> {
        let mut people_hash: HashMap<String, NewPerson> = HashMap::new();
        WalkDir::new(path)
            .max_depth(3)
            .min_depth(3)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.metadata()
                    .expect("Could not access metadata for input files")
                    .is_file()
            })
            .try_for_each(|file| -> Result<(), Box<dyn Error>> {
                let filename = file.path().file_name()?.to_string_lossy().into_owned();
                people_hash
                    .entry(filename.clone())
                    .and_modify(|person| {
                        person.add_path(file.path().to_string_lossy().into_owned())
                    })
                    .or_insert(NewPerson::build(
                        filename,
                        file.path().to_string_lossy().into_owned(),
                    ));
                Ok(())
            });
        Ok(people_hash
            .into_iter()
            .map(|(k, v)| (k, v.construct_history()))
            .collect())
    }

    pub fn save(&self, path: &Path) {
        if path.exists() {
            panic!("The ouput path provided already exists. Executuion has been halted to prevent overriding data.")
            // This is the wrong place for this check!!! Move it to the user-interfacing commands when possible
        }
        //println!("{:?}", path.join("AWA.csv"));
        fs::create_dir_all(path).expect("Couldn't create file structure.");
        let mut summary_wtr = csv::Writer::from_path(path.join("PeopleSummary.csv"))
            .expect("Couldn't open saving path.");
        self.hash
            .iter()
            .for_each(|(_, v)| summary_wtr.serialize(v).expect("Writing problem."));
        summary_wtr.flush().unwrap();
        let person_data_path = path.join("People");
        let mut people_data_paths_wtr = csv::Writer::from_path(path.join("PeopleInfo.csv"))
            .expect("Couldn't open saving path.");
        self.hash.iter().for_each(|(_, person)| {
            // Both output a file containing summary statistics for a person and save the generated file path for that person to memory.
            let person_path = person.write(&person_data_path);
            // Save a CSV containing paths to people's data.
            people_data_paths_wtr
                .write_record(&[
                    person.get_name(),
                    &person_path
                        .components()
                        .skip(path.components().count())
                        .collect::<PathBuf>()
                        .into_os_string()
                        .into_string()
                        .unwrap(),
                ])
                .expect("Writing issue.")
        });
        people_data_paths_wtr.flush().unwrap()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct FileInstance {
    path: String,
    time: String,
    filesize: u64,
    text: String,
}
impl FileInstance {
    fn build(path: String) -> FileInstance {
        FileInstance {
            path: path.clone(),
            time: Path::new(&path)
                .components()
                .nth_back(1)
                .expect("Couldn't find folder name.")
                .as_os_str()
                .to_string_lossy()
                .to_string(),
            filesize: fs::metadata(&path).unwrap().len(),
            text: DocxParser::open(path)
                .expect("Couldn't open a file.")
                .parse()
                .unwrap()
                .plain_text(),
        }
    }
    pub fn get_text(&self) -> &String {
        &self.text
    }
    pub fn get_size(&self) -> &u64 {
        &self.filesize
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct EditInstance {
    time: String,
    change_wordcount: u32,
    change_sentence_count: u32,
    #[serde(skip_serializing)]
    changes: String,
}
impl EditInstance {
    fn new(timeperiod: String, changes: String) -> EditInstance {
        EditInstance {
            time: timeperiod,
            change_wordcount: changes.unicode_words().count() as u32,
            change_sentence_count: changes.unicode_sentences().count() as u32,
            changes,
        }
    }
    pub fn get_edits(&self) -> &String {
        &self.changes
    }
    pub fn get_timeperiod(&self) -> &String {
        &self.time
    }
    pub fn write(&self, path: &Path) -> &String {
        let time_path = path.join(&self.time);
        fs::create_dir_all(&time_path).expect("Couldn't create file structure.");
        let mut time_text_file =
            File::create(time_path.join("Additions.txt")).expect("Couldn't create a new file.");
        time_text_file
            .write_all(&self.changes.as_bytes())
            .expect("Could not write additions file.");
        &self.time
    }
}

#[derive(Debug)]
struct NewPerson {
    filename: String,
    content: Vec<FileInstance>,
}

impl NewPerson {
    fn build(filename: String, path: String) -> NewPerson {
        NewPerson {
            filename: filename,
            content: vec![FileInstance::build(path)],
        }
    }
    fn add_path(&mut self, path: String) {
        self.content.push(FileInstance::build(path));
        self.content.sort_by_key(|s| s.path.clone());
    }
    pub fn construct_history(self) -> PersonHistory {
        PersonHistory::build(self)
    }
    /// This function compares each snapshot of a person's writing and finds changes between each version.
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
                        .map(|x| x.as_str().unwrap().trim())
                        .collect::<Vec<&str>>(),
                    &window[1]
                        .unicode_sentences()
                        .map(|x| x.as_str().unwrap().trim())
                        .collect::<Vec<&str>>(),
                )
                .iter_all_changes()
                .filter(|change| change.tag() == ChangeTag::Insert)
                .map(|change: similar::Change<_>| change.value().to_string())
                .collect::<Vec<String>>()
                .join(" ")
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

#[derive(Debug, Clone, serde::Serialize)]
pub struct PersonHistory {
    filename: String,
    filesize: u64,
    wordcount: u32,
    #[serde(skip_serializing)]
    content: Vec<FileInstance>,
    #[serde(skip_serializing)]
    diffmap: Vec<EditInstance>,
    #[serde(skip_serializing)]
    final_text: String,
}
impl PersonHistory {
    pub fn build(person: NewPerson) -> PersonHistory {
        let diffmap = person.find_diffs();
        let content = person.content;
        let final_text = content
            .iter()
            .max_by_key(|x| x.time.clone())
            .expect("Couldn't find max time period.")
            .get_text()
            .to_string();
        PersonHistory {
            filename: person.filename,
            filesize: *content
                .iter()
                .max_by_key(|x| x.time.clone())
                .expect("Couldn't find max time period.")
                .get_size(),
            wordcount: final_text.unicode_words().count() as u32,
            content,
            diffmap,
            final_text,
        }
    }
    pub fn print_history(&self) {
        println!("———————————————————————————");
        println!("Printing hisotry from filenames \"{}\".", self.filename);
        self.diffmap
            .iter()
            .for_each(|x| println!("For time {}, \"{}\"", x.get_timeperiod(), x.get_edits()));
    }
    pub fn write(&self, path: &Path) -> PathBuf {
        let my_path = path.join(&self.filename);
        fs::create_dir_all(&my_path).expect("Couldn't create file structure.");
        let mut final_text_file =
            File::create(my_path.join("FinalText.txt")).expect("Couldn't create a new file.");
        final_text_file
            .write_all(&self.final_text.as_bytes())
            .expect("Could not write final text state.");
        let mut timeperiod_wtr =
            csv::Writer::from_path(my_path.join("timeperiod.csv")).expect("Failed to open path.");
        self.diffmap.iter().for_each(|x| {
            timeperiod_wtr
                .serialize(x)
                .expect("Failed to serialize edit.");
            x.write(&my_path.join("Times"));
        });
        my_path
    }
    pub fn get_name(&self) -> &String {
        &self.filename
    }
}
