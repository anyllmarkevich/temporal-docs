pub mod text_edits;
use anyhow::{Context, Result};
use core::panic;
use csv;
use indicatif::{ParallelProgressIterator, ProgressIterator};
use itertools::{izip, MultiUnzip};
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
use walkdir::{DirEntry, WalkDir};

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
    time_periods: Vec<String>,
}
impl DatabaseHistory {
    /// Create a database containing a complete history of edits between document snapshots from multiple people. Needs a path pointing to directory with a properly specified structure within as input.
    pub fn build(path: &Path) -> Result<DatabaseHistory> {
        println!("Locating time period directories...");
        // Get data on files and folders on the input dataset.
        let file_structure: Vec<DirEntry> = WalkDir::new(path)
            .min_depth(1)
            .max_depth(2)
            .into_iter()
            .filter_map(|e| e.ok())
            .collect();
        // Get the names of all time periods.
        let time_periods: Vec<String> = file_structure
            .iter()
            .filter(|e| e.depth() == 1 && e.file_type().is_dir())
            .map(|dir| {
                dir.path()
                    .file_name()
                    .expect("")
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        // Pass info on all files and folders to this function to get actual data on people's writing.
        let data = Self::extract_data(file_structure)?;
        Ok(DatabaseHistory { data, time_periods })
    }
    /// Easily print a history of a specific type of edit or text data, as determined by the SaveType input.
    pub fn print_changelist(&self, save_type: &SaveType) {
        self.data.iter().for_each(|x| x.print_history(save_type));
    }
    /// Traverse input database, extracting a complete list of people who's writing to track and the raw text data of each document snapshot. Then convert this data into a history of edits between snapshots.
    fn extract_data(files: Vec<DirEntry>) -> Result<Vec<PersonHistory>> {
        println!("Locating files...");
        let people_hash: HashMap<String, Vec<String>> = files
            .into_par_iter()
            .filter(|e| e.depth() == 2 && e.file_type().is_file())
            .map(|file| {
                let name = file
                    .path()
                    .file_name()
                    .expect("Should never fail. Failed to read a filename")
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
            .collect::<Result<Vec<PersonHistory>>>()
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
                            .expect("Something went wrong while writing.")
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
    /// Return a vector of the names of all the time periods this data structure contains.
    pub fn get_time_periods(&self) -> &Vec<String> {
        &self.time_periods
    }
    /// Return a vector all the data on each person in this data structure. The take_data() function may be preferable for performance if the DatabaseHistory instance is no longer needed but ownership of the data is required.
    pub fn get_data(&self) -> &Vec<PersonHistory> {
        &self.data
    }
    /// Consume this data structure to get ownership of all the data on each person in the dataset. This function may be preferable to get_data() if ownership of the data is required, the DatabaseHistory instance is no longer needed, and the performance cost of cloning the data is unacceptable.
    pub fn take_data(self) -> Vec<PersonHistory> {
        self.data
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
    fn build(path: String) -> Result<FileInstance> {
        Ok(FileInstance {
            path: path.clone(),
            time: Path::new(&path)
                .components()
                .nth_back(1)
                .with_context(|| {
                    format!(
                        "Couldn't find folder name of specified file path: {}.\nPlease ensure all documents in this directory are Microsoft Word files (docx)",
                        path.to_string_lossy()
                    )
                })?
                .as_os_str()
                .to_string_lossy()
                .to_string(),
            filesize: fs::metadata(&path).unwrap().len(),
            text: DocxParser::open(&path)
                .with_context(|| {
                    format!("Could not open file at path: {}.\nPlease ensure all documents in this directory are Microsoft Word files (.docx file extension)", path.to_string_lossy())
                })?
                .parse().with_context(||format!("Could not parse file found at {}.\nPlease ensure all documents in this directory are uncorrupted Microsoft Word files.", path.to_string_lossy()))?
                .plain_text(),
        })
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
    pub fn build(filename: String, paths: Vec<String>) -> Result<PersonHistory> {
        let mut content: Vec<FileInstance> = paths
            .iter()
            .map(|path| FileInstance::build(path.to_string()))
            .collect::<Result<Vec<FileInstance>>>()
            .with_context(|| {
                format!(
                    "Failed to extract data for person using files named {}",
                    filename
                )
            })?;
        content.sort_by_key(|file| file.get_time_period().to_owned());
        Ok(PersonHistory {
            filename,
            data: TimePeriod::build_from_history(content),
        })
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
    /// Saves all the information about a single kind of edit to a text file, taking the path, the name of the kind of edit, and the text data as input.
    fn write_edit_type(time_period_path: &Path, edit_type: &String, text: &String) -> Result<()> {
        let mut text_file = File::create(time_period_path.join(format!("{}.txt", edit_type)))
            .with_context(|| {
                format!(
                    "Could not create \"{}.txt\" file at the following path: {}",
                    edit_type.to_string_lossy(),
                    time_period_path.to_string_lossy()
                )
            })?;
        text_file.write_all(&text.as_bytes()).with_context(|| {
            format!(
                "Could not write data to \"{}.txt\" file at the following path: {}",
                edit_type.to_string_lossy(),
                time_period_path.to_string_lossy()
            )
        })?;
        text_file.flush().with_context(|| {
            format!(
                "Could not flush data to \"{}.txt\" file at the following path: {}",
                edit_type.to_string_lossy(),
                time_period_path.to_string_lossy()
            )
        })?;
        Ok(())
    }
    /// Saves an entire TimePeriod to text and CSV files. Takes a TimePeriod, a path, and a CSV writer instance for the enclosing person as input.
    fn write_time_period(
        time_period: &TimePeriod,
        path: &Path,
        time_period_writer: &mut csv::Writer<File>,
    ) -> Result<()> {
        // Use CSV writer instance to save general information about the time period.
        let _ = time_period_writer.serialize(time_period);
        // Set up the paths to use for the information on edits during this time period, and create the directory.
        let time_period_path = &path.join("Times").join(time_period.time.to_string());
        fs::create_dir_all(&time_period_path).with_context(|| {
            format!(
                "Could not create necessary directories to make the following path valid: {}",
                time_period_path.to_string_lossy()
            )
        })?;
        // Save all the data on edits inside the directory created for this purpose by extracting text from the enclosed EditInstance and writing that text.
        let _ = time_period
            .edits
            .get_all_edits()
            .iter()
            .try_for_each(|(edit_type, text)| {
                Self::write_edit_type(time_period_path, edit_type, text)
            });
        Ok(())
    }

    /// Save all the data about a person's writing and the edits they made at each time period to a specific filepath.
    pub fn write(&self, path: &Path) -> Result<PathBuf> {
        let my_path = path.join(&self.filename);
        fs::create_dir_all(&my_path).with_context(|| {
            format!(
                "Could not create necessary directories to make the following path valid: {}",
                my_path.to_string_lossy()
            )
        })?;
        let mut timeperiod_wtr = csv::Writer::from_path(my_path.join("timeperiod.csv"))
            .with_context(|| {
                format!(
                    "Could not create \"timeperiod.csv\" file at the following path: {}",
                    my_path.to_string_lossy(),
                )
            })?;
        let _ = self.data.iter().try_for_each(|time_period| {
            Self::write_time_period(time_period, &my_path, &mut timeperiod_wtr)
        });
        let _ = timeperiod_wtr.flush().with_context(|| {
            format!(
                "Could not flush new file \"timeperiod.csv\" at the following path: {}",
                my_path.to_string_lossy()
            )
        });
        Ok(my_path)
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
