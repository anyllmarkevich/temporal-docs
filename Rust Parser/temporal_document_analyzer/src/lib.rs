use core::panic;
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
use undoc::docx::DocxParser;
use unicode_segmentation::{UWordBounds, UnicodeSegmentation};
use walkdir::WalkDir;

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
    sentence_additions: String,
    #[serde(skip_serializing)]
    sentence_edits: String,
    #[serde(skip_serializing)]
    word_additions: String,
    #[serde(skip_serializing)]
    word_deletions: String,
}
impl EditInstance {
    fn new(
        timeperiod: String,
        sentence_additions: String,
        sentence_edits: String,
        word_additions: String,
        word_deletions: String,
    ) -> EditInstance {
        EditInstance {
            time: timeperiod,
            change_wordcount: sentence_additions.unicode_words().count() as u32,
            change_sentence_count: sentence_additions.unicode_sentences().count() as u32,
            sentence_additions,
            sentence_edits,
            word_additions,
            word_deletions,
        }
    }
    pub fn get_edits(&self) -> &String {
        &self.sentence_additions
    }
    pub fn get_timeperiod(&self) -> &String {
        &self.time
    }
    fn save_to_file(path: &Path, filename: &str, content: &String) -> Result<(), io::Error> {
        fs::create_dir_all(&path)?;
        let mut text_file = File::create(path.join(filename))?;
        text_file.write_all(&content.as_bytes())?;
        text_file.flush()?;
        Ok(())
    }

    pub fn write(&self, path: &Path) -> &String {
        let time_path = path.join(&self.time);
        Self::save_to_file(
            &time_path,
            "SentenceAdditions.txt",
            &self.sentence_additions,
        )
        .expect("Failed to save data");
        Self::save_to_file(&time_path, "SentenceEdits.txt", &self.sentence_edits)
            .expect("Failed to save data");
        Self::save_to_file(&time_path, "WordAdditions.txt", &self.word_additions)
            .expect("Failed to save data");
        Self::save_to_file(&time_path, "WordDeletions.txt", &self.word_deletions)
            .expect("Failed to save data");
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

    // Conveniently extracts changes with any tag except "Change::Equal" as a String.
    fn edit_to_string<T: similar::DiffableStr + ToString + ?Sized>(diff: &TextDiff<T>) -> String {
        diff.iter_all_changes()
            .filter(|change| change.tag() == ChangeTag::Delete || change.tag() == ChangeTag::Insert)
            .map(|change: similar::Change<_>| change.value().to_string())
            .collect::<Vec<String>>()
            .join(" ")
    }

    // Conveniently extracts changes with a specific tag as a String.
    fn tag_to_string<T: similar::DiffableStr + ToString + ?Sized>(
        diff: &TextDiff<T>,
        tag: ChangeTag,
    ) -> String {
        diff.iter_all_changes()
            .filter(|change| change.tag() == tag)
            .map(|change: similar::Change<_>| change.value().to_string())
            .collect::<Vec<String>>()
            .join(" ")
    }
    /// This function compares each snapshot of a person's writing and finds changes between each version.
    fn find_diffs(&self) -> Vec<EditInstance> {
        // Extract raw text data from each timeperiod snapshot.
        let raw_text: Vec<String> = self
            .content
            .iter()
            .map(|instance| instance.text.clone())
            .collect::<Vec<String>>();
        // Create comparison objects for each timeperiod.
        let linediffs_vec: Vec<TextDiff<'_, '_, str>> = raw_text
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
            })
            .collect();
        let worddiffs_vec: Vec<TextDiff<'_, '_, str>> = raw_text
            .windows(2)
            .map(|window| TextDiff::from_words(&window[0], &window[1]))
            .collect();
        // Convert these comparison objects into addition and deletion strings within a tuple for each timeperiod.
        let adds_dels_vec: Vec<(String, String, String, String)> = linediffs_vec
            .iter()
            .zip(worddiffs_vec)
            .map(|(sentence_period, word_period)| {
                (
                    Self::tag_to_string(sentence_period, ChangeTag::Insert),
                    Self::edit_to_string(sentence_period),
                    Self::tag_to_string(&word_period, ChangeTag::Insert),
                    Self::tag_to_string(&word_period, ChangeTag::Delete),
                )
            })
            .collect();
        // Save these addition and deletion strings into a struct that also contains timeperiod information.
        let mut diffs_vec: Vec<EditInstance> = adds_dels_vec
            .iter()
            .zip(
                self.content
                    .iter()
                    .skip(1)
                    .map(|x| x.time.clone())
                    .into_iter()
                    .collect::<Vec<String>>(),
            )
            .map(|((s_adds, s_edits, w_adds, w_dels), time)| {
                EditInstance::new(
                    time,
                    s_adds.to_string(),
                    s_edits.to_string(),
                    w_adds.to_string(),
                    w_dels.to_string(),
                )
            })
            .collect();
        // Add all text from the first timeperiod as a change as this initial text is not considered by the comparison algorithm.
        diffs_vec.insert(
            0,
            EditInstance::new(
                self.content[0].time.clone(),
                self.content[0].text.clone(),
                self.content[0].text.clone(),
                self.content[0].text.clone(),
                String::new(),
            ),
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
