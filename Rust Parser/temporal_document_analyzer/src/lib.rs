use csv;
use rayon::{self, join};
use similar::{self, ChangeTag, DiffableStr, DiffableStrRef, TextDiff};
use std::collections::HashMap;
use std::{fs, path::Path};
use undoc::docx::DocxParser;
use unicode_segmentation::{UWordBounds, UnicodeSegmentation};
use walkdir::WalkDir;

pub struct DatabaseHistory {
    summary_stats: SummaryStats,
    hash: HashMap<String, PersonHistory>,
}
impl DatabaseHistory {
    pub fn build(path: &Path) -> DatabaseHistory {
        let hash = Self::hash_people(path);
        DatabaseHistory {
            summary_stats: SummaryStats::build(&hash),
            hash: hash,
        }
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
}

struct SummaryStats {
    wordcount: Vec<u32>,
    file_size: Vec<u64>,
    people: Vec<String>,
}

impl SummaryStats {
    fn build(map: &HashMap<String, PersonHistory>) -> SummaryStats {
        let mut people: Vec<String> = Vec::new();
        map.iter().for_each(|(k, _)| people.push(k.clone()));
        SummaryStats {
            wordcount: people
                .iter()
                .map(|person| {
                    map.get(person)
                        .expect("Couldn't find a person's data based on their name.")
                        .get_final_text()
                        .unicode_words()
                        .count() as u32
                })
                .collect(),
            file_size: people
                .iter()
                .map(|person| {
                    map.get(person)
                        .expect("Couldn't find a person's data based on their name.")
                        .get_final_file_size()
                        .clone()
                })
                .collect(),
            people,
        }
    }
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct PersonHistory {
    diffmap: Vec<EditInstance>,
    filename: String,
    content: Vec<FileInstance>,
}
impl PersonHistory {
    pub fn print_history(&self) {
        println!("———————————————————————————");
        println!("Printing hisotry from filenames \"{}\".", self.filename);
        self.diffmap
            .iter()
            .for_each(|x| println!("For time {}, \"{}\"", x.get_timeperiod(), x.get_edits()));
    }
    pub fn get_final_text(&self) -> &String {
        self.content
            .iter()
            .max_by_key(|x| x.time.clone())
            .expect("Couldn't find max time period.")
            .get_text()
    }
    pub fn get_final_file_size(&self) -> &u64 {
        self.content
            .iter()
            .max_by_key(|x| x.time.clone())
            .expect("Couldn't find max time period.")
            .get_size()
    }
}
