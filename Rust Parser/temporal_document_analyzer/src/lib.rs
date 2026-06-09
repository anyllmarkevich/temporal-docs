pub mod text_edits;
use core::{panic, time};
use csv;
use rayon::{self, join};
use serde::de::Error;
use serde::Serialize;
use similar::{self, Change, ChangeTag, DiffableStr, DiffableStrRef, TextDiff};
use std::collections::HashMap;
use std::fmt::format;
use std::fs::File;
use std::io::{self, Write};
use std::{
    fs,
    path::{Path, PathBuf},
};
use text_edits::*;
use undoc::docx::DocxParser;
use unicode_segmentation::{UWordBounds, UnicodeSegmentation};
use walkdir::WalkDir;

/// This struct captures and saves a complete record of the every document written by every person, along with a complete record of changes between time periods.
/// # Example Usage
/// ```
/// use std::path::{self, Path};
/// use temporal_document_analyzer::{self, DatabaseHistory};
/// let input_path = Path::new("~/users/example_user/data_folder/");
/// let output_path = Path::new("~/users/example_user/save_folder/");
/// let database = DatabaseHistory::build(input_path); // Create database and calculate edit history
/// database.print_changelist(); // Print changes to the console
/// database.save(output_path); // Save final text and edit history for every person into a directory, easily readable using accompanying R functions
/// ```
pub struct DatabaseHistory {
    hash: HashMap<String, PersonHistory>,
}
impl DatabaseHistory {
    pub fn build(path: &Path) -> DatabaseHistory {
        let hash = Self::hash_people(path);
        DatabaseHistory { hash: hash }
    }

    pub fn print_changelist(&self) {
        self.hash.iter().for_each(|(_, x)| x.print_history());
    }

    fn hash_people(path: &Path) -> HashMap<String, PersonHistory> {
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
                    .and_modify(|person| {
                        person.add_path(file.path().to_string_lossy().into_owned())
                    })
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

    pub fn save(&self, path: &Path) {
        if !path.read_dir().unwrap().next().is_none() || !path.exists() {
            panic!("The ouput path provided already exists. Executuion has been halted to prevent overriding data.")
            // This is the wrong place for this check!!! Move it to the user-interfacing commands when possible
        }
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
        people_data_paths_wtr.flush().unwrap();
        let mut savetype_wtr =
            csv::Writer::from_path(path.join("SaveType.csv")).expect("Could not open saving path.");
        SaveType::list_savetypes()
            .iter()
            .for_each(|x| savetype_wtr.write_record(&[x]).expect("Writing issue."));
        savetype_wtr.flush().unwrap();
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
    pub fn get_time_period(&self) -> &String {
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
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PersonHistory {
    filename: String,
    filesize: u64,
    wordcount: u32,
    #[serde(skip_serializing)]
    content: Vec<FileInstance>,
    #[serde(skip_serializing)]
    diffmap: Vec<(String, EditInstance)>,
    #[serde(skip_serializing)]
    final_text: String,
}
impl PersonHistory {
    pub fn build(person: NewPerson) -> PersonHistory {
        let time_periods: Vec<String> = person
            .content
            .iter()
            .map(|file| file.get_time_period().clone())
            .collect();
        let diffmap: Vec<(String, EditInstance)> = EditInstance::edits_from_history(
            person
                .content
                .iter()
                .map(|file| file.get_text().clone())
                .collect::<Vec<String>>(),
        )
        .into_iter()
        .zip(time_periods)
        .map(|(edits, time)| (time, edits))
        .collect();
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
        self.diffmap.iter().for_each(|(time, edits)| {
            println!(
                "The following words were written during timeperiod {}: \"{}\"",
                time,
                edits.get_text(SaveType::WordAdditions)
            )
        });
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
        self.diffmap.iter().for_each(|(time, edit)| {
            timeperiod_wtr
                .write_record(&[time])
                .expect("Failed to serialize edit.");
            edit.write(&my_path.join("Times"), time.to_string());
        });
        timeperiod_wtr.flush().unwrap();
        my_path
    }
    pub fn get_name(&self) -> &String {
        &self.filename
    }
}
