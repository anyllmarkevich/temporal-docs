pub mod text_edits;
use core::panic;
use csv;
use rayon::prelude::*;
use serde::Serialize;
use similar::DiffableStr;
use std::{
    collections::HashMap,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};
use text_edits::*;
use undoc::docx::DocxParser;
use unicode_segmentation::UnicodeSegmentation;
use walkdir::WalkDir;

/// Captures and saves a complete record of the every document written by every person in the database, along with a complete record of changes between time periods.
/// # Example Usage
/// ```no_run
/// use std::path::{self, Path};
/// use temporal_document_analyzer::{self, DatabaseHistory};
/// let input_path = Path::new("examples/data_folder/");
/// let output_path = Path::new("examples/save_folder/");
/// let database = DatabaseHistory::build(input_path); // Create database and calculate edit history
/// database.print_changelist(); // Print changes to the console
/// database.save(output_path); // Save final text and edit history for every person into a directory, easily readable using accompanying R functions
/// ```
pub struct DatabaseHistory {
    data: Vec<PersonHistory>,
}
impl DatabaseHistory {
    /// Create a database containing a complete history of edits between document snapshots from multiple people. Needs a path pointing to directory with a properly specified structure within as input.
    pub fn build(path: &Path) -> DatabaseHistory {
        let data = Self::extract_data(path);
        DatabaseHistory { data }
    }
    /// Easily print a rudimentary history of word-level additions for debugging purposes
    pub fn print_changelist(&self) {
        self.data.iter().for_each(|x| x.print_history());
    }
    /// Traverse input database, extracting a complete list of people who's writing to track and the raw text data of each document snapshot. Then convert this data into a history of edits between snapshots.
    fn extract_data(path: &Path) -> Vec<PersonHistory> {
        let people_hash: HashMap<String, Vec<String>> = WalkDir::new(path)
            .max_depth(2)
            .min_depth(2)
            .into_iter()
            .par_bridge()
            .filter_map(|e| e.ok())
            .filter(|e| e.metadata().unwrap().is_file())
            .map(|file| {
                let name = file
                    .path()
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                let path = file.path().to_string_lossy().into_owned();
                (name, path)
            })
            .fold(
                || HashMap::new(),
                |mut temp_map, (name, path)| {
                    temp_map
                        .entry(name)
                        .or_insert_with(|| Vec::new())
                        .push(path);
                    temp_map
                },
            )
            .reduce(
                || HashMap::new(),
                |mut map, new_map| {
                    new_map.into_iter().for_each(|(name, paths)| {
                        map.entry(name).or_insert_with(|| Vec::new()).extend(paths);
                    });
                    map
                },
            );
        people_hash
            .into_par_iter()
            .map(|(name, paths)| PersonHistory::build(name, paths))
            .collect()
    }
    /// Save the entire database of edits between document snapshots written by multiple people to a new directory as a series of folders, text files, and CSV files. This structure can easily be read by the accompanying R program. The specified directory must be empty.
    pub fn save(&self, path: &Path) {
        if path.exists() && !path.read_dir().unwrap().next().is_none() {
            panic!("The ouput path provided already exists. Executuion has been halted to prevent overriding data.")
            // This is the wrong place for this check!!! Move it to the user-interfacing commands when possible
        }
        fs::create_dir_all(path).expect("Couldn't create file structure.");
        let mut summary_wtr = csv::Writer::from_path(path.join("PeopleSummary.csv"))
            .expect("Couldn't open saving path.");
        self.data
            .iter()
            .for_each(|v| summary_wtr.serialize(v).expect("Writing problem."));
        summary_wtr.flush().unwrap();
        let person_data_path = path.join("People");
        let mut people_data_paths_wtr = csv::Writer::from_path(path.join("PeopleInfo.csv"))
            .expect("Couldn't open saving path.");
        self.data.iter().for_each(|person| {
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

// Contains information about a single snapshot of a document, including the raw text data, the path were the data was found, and the original file size.
#[derive(Debug, Clone, Serialize)]
pub struct FileInstance {
    path: String,
    time: String,
    filesize: u64,
    text: String,
}
impl FileInstance {
    /// Save information on a new file containing a document snapshot of a person's writing.
    pub fn build(path: String) -> FileInstance {
        FileInstance {
            path: path.clone(),
            time: Path::new(&path)
                .components()
                .nth_back(1)
                .expect("Couldn't find folder name")
                .as_os_str()
                .to_string_lossy()
                .to_string(),
            filesize: fs::metadata(&path).unwrap().len(),
            text: DocxParser::open(path)
                .expect("Couldn't open document in the Microsoft Word parser")
                .parse()
                .expect("Could not parse Microsoft Word document")
                .plain_text(),
        }
    }
    /// Get the raw text contained in the file.
    pub fn get_text(&self) -> &String {
        &self.text
    }
    /// Get the size of the file in bytes.
    pub fn get_size(&self) -> &u64 {
        &self.filesize
    }
    /// Get the name of the enclosing folder, which should correspond to the time period after which this snapshot was taken if the database is formatted properly.
    pub fn get_time_period(&self) -> &String {
        &self.time
    }
}

/// Handles converting raw data about a single person's writing into a series of edits, storing this data, and allowing the API to access it in a variety of ways.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PersonHistory {
    filename: String,
    #[serde(skip_serializing)]
    filesizes: Vec<u64>,
    #[serde(skip_serializing)]
    diffmap: Vec<(String, EditInstance)>,
}
impl PersonHistory {
    /// Converts a NewPerson struct into a PersonHistory struct by calculating differences between writing snapshots and saving a bunch of data on the writing of that person. This conversion is handled from the NewPerson struct from the API's perspective, even though it simply calls this function.
    pub fn build(filename: String, paths: Vec<String>) -> PersonHistory {
        let mut content: Vec<FileInstance> = paths
            .iter()
            .map(|path| FileInstance::build(path.to_string()))
            .collect::<Vec<FileInstance>>();
        content.sort_by_key(|file| file.get_time_period().to_owned());
        let filesizes: Vec<u64> = content
            .iter()
            .map(|file| file.get_size().to_owned())
            .collect();
        let time_periods: Vec<String> = content
            .iter()
            .map(|file| file.get_time_period().clone())
            .collect();
        let diffmap: Vec<(String, EditInstance)> = EditInstance::edits_from_history(
            content
                .into_iter()
                .map(|file| file.text)
                .collect::<Vec<String>>(),
        )
        .into_iter()
        .zip(time_periods)
        .map(|(edits, time)| (time, edits))
        .collect();
        PersonHistory {
            filename,
            filesizes,
            diffmap,
        }
    }
    /// Very basic output of word-level additions to that console for debugging.
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
    /// Save all the data about a person's writing and the edits they made at each time period to a specific filepath.
    pub fn write(&self, path: &Path) -> PathBuf {
        let my_path = path.join(&self.filename);
        fs::create_dir_all(&my_path).expect("Couldn't create file structure.");
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
    /// Get the name used to identify a person in the database (the filename of their writing snapshots)
    pub fn get_name(&self) -> &String {
        &self.filename
    }
}
