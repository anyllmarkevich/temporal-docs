pub mod text_edits;
use core::panic;
use csv;
use indicatif::{ParallelProgressIterator, ProgressIterator};
use itertools::{izip, MultiUnzip};
use rayon::prelude::*;
use serde::Serialize;
use std::{
    collections::HashMap,
    fs,
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
/// use temporal_docx::{self, DatabaseHistory, text_edits::SaveType};
/// let input_path = Path::new("examples/data_folder/");
/// let output_path = Path::new("examples/save_folder/");
/// let database = DatabaseHistory::build(input_path); // Create database and calculate edit history
/// database.print_changelist(&SaveType::SentenceAdditions); // Print the everyone's history of sentences that were added or added to between snapshots.
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
    /// Easily print a history of a specific type of edit or text data, as determined by the SaveType input.
    pub fn print_changelist(&self, save_type: &SaveType) {
        self.data.iter().for_each(|x| x.print_history(save_type));
    }
    /// Traverse input database, extracting a complete list of people who's writing to track and the raw text data of each document snapshot. Then convert this data into a history of edits between snapshots.
    fn extract_data(path: &Path) -> Vec<PersonHistory> {
        println!("Locating files...");
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
        let progress_len = people_hash.len() as u64;
        println!("Converting files to temporal data...");
        people_hash
            .into_par_iter()
            .progress_count(progress_len)
            .map(|(name, paths)| PersonHistory::build(name, paths))
            .collect()
    }
    /// Save the entire database of edits between document snapshots written by multiple people to a new directory as a series of folders, text files, and CSV files. This structure can easily be read by the accompanying R program. The specified directory must be empty.
    pub fn save(&self, path: &Path) {
        println!("Saving data to files...");
        if path.exists() && !path.read_dir().unwrap().next().is_none() {
            panic!("The ouput path provided already exists. Executuion has been halted to prevent overriding data.")
            // This is the wrong place for this check!!! Move it to the user-interfacing commands when possible
        }
        fs::create_dir_all(path).expect("Couldn't create file structure.");
        let person_data_path = path.join("People");
        let mut people_data_paths_wtr = csv::Writer::from_path(path.join("PeopleInfo.csv"))
            .expect("Couldn't open saving path.");
        let progress_len = self.data.len() as u64;
        self.data
            .iter()
            .progress_count(progress_len)
            .for_each(|person| {
                // Both output a file containing summary statistics for a person and save the generated filepath for that person to memory.
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
struct FileInstance {
    path: String,
    time: String,
    filesize: u64,
    text: String,
}
impl FileInstance {
    /// Save information on a new file containing a document snapshot of a person's writing.
    fn build(path: String) -> FileInstance {
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
    fn get_text(&self) -> &String {
        &self.text
    }
    /// Get the size of the file in bytes.
    fn get_size(&self) -> &u64 {
        &self.filesize
    }
    /// Get the name of the enclosing folder, which should correspond to the time period after which this snapshot was taken if the database is formatted properly.
    fn get_time_period(&self) -> &String {
        &self.time
    }
}

/// Saves processed data about a single person/document for a time period (as determined by the saved snapshots). This includes information on changes since the last time period and information on the file representing the current time period (file size, word counts, etc.).
#[derive(Debug, Clone, Serialize)]
pub struct TimePeriod {
    time: String,
    filesize: u64,
    word_count: usize,
    sentence_count: usize,
    #[serde(skip)]
    edits: EditInstance,
}

impl TimePeriod {
    /// Get the name of the time period, which will typically be the name of the directory enclosing the document representing this time period.
    pub fn get_time_period(&self) -> &String {
        &self.time
    }
    /// Get the file size of the associated document in bytes.
    pub fn get_file_size(&self) -> &u64 {
        &self.filesize
    }
    /// Get the number of Unicode words in the associated document.
    pub fn get_word_count(&self) -> &usize {
        &self.word_count
    }
    /// Get the number of Unicode sentences in the associated document.
    pub fn get_sentence_count(&self) -> &usize {
        &self.sentence_count
    }
    /// Borrow text data and edits since the last snapshot (packaged in an EditInstance struct). If ownership of this data is required and this TimePeriod instance is no longer needed, the extract_data() function is preferable.
    pub fn get_edits(&self) -> &EditInstance {
        &self.edits
    }
    /// Get ownership of text data and edits since the last snapshot (packaged in an EditInstance struct), thereby destroying the TimePeriod instance. This function may be favored other get_edits() if it helps avoid copying data.
    pub fn extract_edits(self) -> EditInstance {
        self.edits
    }
    /// This function really only exists because EditInstance requires a vector of all the document versions to build a history of edits, while each TimePeriod instance should contain information on only one version at a time. This whole function simply unwraps all the data in each FileInstance so that the data can be used sequentially by EditInstance, before chopping it back up into TimePeriods. The only reason it is so complicated is to avoid copying the text data, which may be very large depending on the files the user is working with. Yes, each TimePeriod could take multiple files as input to construct the EditInstances one by one, but I would rather keep all that functionality hidden in EditInstance.
    fn build_from_history(files: Vec<FileInstance>) -> Vec<TimePeriod> {
        let (times, filesizes, word_counts, sentence_counts, text): (
            Vec<String>,
            Vec<u64>,
            Vec<usize>,
            Vec<usize>,
            Vec<String>,
        ) = files
            .into_iter()
            .map(|f| {
                (
                    f.get_time_period().clone(),
                    f.get_size().clone(),
                    f.get_text().unicode_words().count(),
                    f.get_text().unicode_sentences().count(),
                    f.text,
                )
            })
            .multiunzip();
        let edits: Vec<EditInstance> = EditInstance::edits_from_history(text); // This line is the only reason this function exists.
        izip!(times, filesizes, word_counts, sentence_counts, edits)
            .into_iter()
            .map(
                |(time, filesize, word_count, sentence_count, edits)| TimePeriod {
                    time,
                    filesize,
                    word_count,
                    sentence_count,
                    edits,
                },
            )
            .collect()
    }
}

/// Handles converting raw data about a single person's writing into a series of edits, storing this data, and allowing the API to access it in a variety of ways.
#[derive(Debug, Clone)]
pub struct PersonHistory {
    filename: String,
    data: Vec<TimePeriod>,
}
impl PersonHistory {
    /// Creates a NewPerson struct, calculating differences between writing snapshots and storing a bunch of data on the writing of that person. Requires the filename of the person's documents and a vector containing paths to every document the person wrote.
    pub fn build(filename: String, paths: Vec<String>) -> PersonHistory {
        let mut content: Vec<FileInstance> = paths
            .iter()
            .map(|path| FileInstance::build(path.to_string()))
            .collect::<Vec<FileInstance>>();
        content.sort_by_key(|file| file.get_time_period().to_owned());
        PersonHistory {
            filename,
            data: TimePeriod::build_from_history(content),
        }
    }
    /// Prints a specific kind of data for the entire history of a person's writing, as determined by the SaveType input. For instance, this function can print all word-level additions, of the fulltext of each snapshot.
    pub fn print_history(&self, edit_type: &SaveType) {
        println!("———————————————————————————");
        println!("Printing hisotry from filenames \"{}\".", self.filename);
        self.data.iter().for_each(|time_period| {
            println!(
                "The following words were written during timeperiod {}: \"{}\"",
                time_period.time,
                time_period.edits.get_text(&edit_type)
            )
        });
    }
    /// Save all the data about a person's writing and the edits they made at each time period to a specific filepath.
    pub fn write(&self, path: &Path) -> PathBuf {
        let my_path = path.join(&self.filename);
        fs::create_dir_all(&my_path).expect("Couldn't create file structure.");
        let mut timeperiod_wtr =
            csv::Writer::from_path(my_path.join("timeperiod.csv")).expect("Failed to open path.");
        self.data.iter().for_each(|time_period| {
            timeperiod_wtr
                .serialize(time_period)
                .expect("Failed to serialize edit.");
            time_period
                .edits
                .write(&my_path.join("Times"), time_period.time.to_string());
        });
        timeperiod_wtr.flush().unwrap();
        my_path
    }
    /// Get the name used to identify a person in the database (the filename of their writing snapshots)
    pub fn get_name(&self) -> &String {
        &self.filename
    }
    /// Access the text data about a persons writing, including edits between documents. The extract_data() function may be more efficient if ownership is required.
    pub fn get_data(&self) -> &Vec<TimePeriod> {
        &self.data
    }
    /// Consumes PersonHistory object to return ownership to text data about a person's writing, including edits between documents. This function may be more efficient than get_data() if it allows not copying the data and the PersonHistory instance is no longer needed.
    pub fn extract_data(self) -> Vec<TimePeriod> {
        self.data
    }
}
