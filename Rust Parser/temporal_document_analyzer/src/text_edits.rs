//! Tools to convert a set of strings representing sequential temporal snapshots of an evolving document or text into information on the sequential changes between each string.

use core::panic;
use csv;
use rayon::{self, join};
use serde::de::Error;
use serde::Serialize;
use similar::{self, Change, ChangeTag, DiffableStr, DiffableStrRef, TextDiff};
use std::fs::File;
use std::io::{self, Write};
use std::{
    fs,
    path::{Path, PathBuf},
};
use unicode_segmentation::{UWordBounds, UnicodeSegmentation};

#[derive(Debug, Clone)]
pub struct EditInstance {
    sentence_additions: String,
    sentence_edits: String,
    word_additions: String,
    word_deletions: String,
    text: String,
}
impl EditInstance {
    /// Creates a vector containing all the data extracted about a series of edis from a vector of Strings representing text snapshots.
    pub fn edit_from_text_comparison(old: &String, current: &String) -> EditInstance {
        let sentence_diffs: TextDiff<'_, '_, str> = TextDiff::from_slices(
            &old.unicode_sentences()
                .map(|x| x.as_str().unwrap().trim())
                .collect::<Vec<&str>>(),
            &current
                .unicode_sentences()
                .map(|x| x.as_str().unwrap().trim())
                .collect::<Vec<&str>>(),
        );
        let word_diffs: TextDiff<'_, '_, str> = TextDiff::from_slices(
            &old.unicode_words()
                .map(|x| x.as_str().unwrap().trim())
                .collect::<Vec<&str>>(),
            &current
                .unicode_words()
                .map(|x| x.as_str().unwrap().trim())
                .collect::<Vec<&str>>(),
        );
        EditInstance {
            sentence_additions: Self::tag_to_string(&sentence_diffs, ChangeTag::Insert),
            sentence_edits: Self::edit_to_string(&sentence_diffs),
            word_additions: Self::tag_to_string(&word_diffs, ChangeTag::Insert),
            word_deletions: Self::tag_to_string(&word_diffs, ChangeTag::Delete),
            text: current.to_string(),
        }
    }

    fn edit_from_text_snapshot(snapshot: &String) -> EditInstance {
        EditInstance {
            sentence_additions: snapshot.clone(),
            sentence_edits: snapshot.clone(),
            word_additions: snapshot.clone(),
            word_deletions: String::new(),
            text: snapshot.to_string(),
        }
    }

    fn edits_from_history(history: Vec<String>) -> Vec<EditInstance> {
        let mut edits: Vec<EditInstance> = vec![Self::edit_from_text_snapshot(&history[0])];
        edits.append(
            &mut history
                .windows(2)
                .map(|window| Self::edit_from_text_comparison(&window[0], &window[1]))
                .collect::<Vec<EditInstance>>(),
        );
        edits
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

    /// Output a vector containing the filenames of each types of data saved, such as sentence-level additions and edits.
    // Update this function to include new save types if any new output text types are added. This will allow R to properly load whatever new data is being produced.
    pub fn list_savetypes() -> Vec<String> {
        vec![
            "SentenceAdditions".to_string(),
            "SentenceEdits".to_string(),
            "WordAdditions".to_string(),
            "WordDeletions".to_string(),
        ]
    }

    fn save_to_file(path: &Path, filename: &str, content: &String) -> Result<(), io::Error> {
        fs::create_dir_all(&path)?;
        let mut text_file = File::create(path.join(filename))?;
        text_file.write_all(&content.as_bytes())?;
        text_file.flush()?;
        Ok(())
    }

    pub fn write(&self, path: &Path, timeperiod_name: String) {
        let time_path = path.join(timeperiod_name);
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
    }
}
