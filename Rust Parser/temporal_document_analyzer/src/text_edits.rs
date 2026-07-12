//! Tools to convert a set of strings representing sequential temporal snapshots of an evolving document or text into information on the sequential changes between each string.

use itertools::Itertools;
use similar::{self, ChangeTag, TextDiff};
use std::collections::HashMap;
use std::mem;
use strum::Display;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use unicode_segmentation::UnicodeSegmentation;

/// Stores a string and the differences between it and another temporally preceding string, including sentence-level additions and edits and word-level additions and deletions.Data can be extracted as strings or saved to files within a specified directory.
#[derive(Debug, Clone)]
pub struct EditInstance {
    sentence_additions: String,
    sentence_edits: String,
    word_additions: String,
    word_deletions: String,
    text: String,
}
impl EditInstance {
    /// Calculate and store data on numerous types of edits between two different snapshots of a document.
    pub fn edit_from_text_comparison(old: &String, current: &String) -> EditInstance {
        let sentence_diffs: TextDiff<'_, '_, str> = TextDiff::from_slices(
            &old.unicode_sentences().collect::<Vec<&str>>(),
            &current.unicode_sentences().collect::<Vec<&str>>(),
        );
        let word_diffs: TextDiff<'_, '_, str> = TextDiff::from_slices(
            &old.unicode_words().collect::<Vec<&str>>(),
            &current.unicode_words().collect::<Vec<&str>>(),
        );
        EditInstance {
            sentence_additions: Self::tag_to_string(&sentence_diffs, ChangeTag::Insert),
            sentence_edits: Self::edit_to_string(&sentence_diffs),
            word_additions: Self::tag_to_string(&word_diffs, ChangeTag::Insert),
            word_deletions: Self::tag_to_string(&word_diffs, ChangeTag::Delete),
            text: current.to_string(),
        }
    }
    /// Create and save data on numerous types of edits from a single document snapshot. Although no novel data can be extracted as there is no point of comparison, this function is useful for formatting the initial state of a document in the same structure as future calculated edits. The entire text is assumed to be a novel addition without any deletions.
    pub fn edit_from_text_snapshot(snapshot: &String) -> EditInstance {
        EditInstance {
            sentence_additions: snapshot.unicode_sentences().map(|s| s.trim()).join(" "),
            sentence_edits: snapshot.unicode_sentences().map(|s| s.trim()).join(" "),
            word_additions: snapshot.unicode_words().map(|s| s.trim()).join(" "),
            word_deletions: String::new(),
            text: snapshot.to_string(),
        }
    }

    /// Creates a vector containing all data on edits performed between several temporally sequential snapshots of a string. The first version of the string is assumed to comprise a single addition without any deletions.
    pub fn edits_from_history(history: Vec<String>) -> Vec<EditInstance> {
        let mut edits: Vec<EditInstance> = vec![Self::edit_from_text_snapshot(&history[0])];
        edits.append(
            &mut history
                .windows(2)
                .map(|window| Self::edit_from_text_comparison(&window[0], &window[1]))
                .collect::<Vec<EditInstance>>(),
        );
        edits
    }

    /// Conveniently extracts changes with any tag except "Change::Equal" as a String.
    fn edit_to_string<T: similar::DiffableStr + ToString + ?Sized>(diff: &TextDiff<T>) -> String {
        diff.iter_all_changes()
            .filter(|change| change.tag() == ChangeTag::Delete || change.tag() == ChangeTag::Insert)
            .map(|change: similar::Change<_>| change.value().to_string().trim().to_string())
            .collect::<Vec<String>>()
            .join(" ")
    }

    /// Conveniently extracts changes with a specific tag as a String.
    fn tag_to_string<T: similar::DiffableStr + ToString + ?Sized>(
        diff: &TextDiff<T>,
        tag: ChangeTag,
    ) -> String {
        diff.iter_all_changes()
            .filter(|change| change.tag() == tag)
            .map(|change: similar::Change<_>| change.value().to_string().trim().to_string())
            .collect::<Vec<String>>()
            .join(" ")
    }

    /// Return various types of edits or current text, specifying what kind of text is returned using an enum.
    pub fn get_text(&self, of_type: &SaveType) -> &String {
        match of_type {
            SaveType::SentenceAdditions => &self.sentence_additions,
            SaveType::SentenceEdits => &self.sentence_edits,
            SaveType::WordAdditions => &self.word_additions,
            SaveType::WordDeletions => &self.word_deletions,
            SaveType::Text => &self.text,
        }
    }
    /// Like get_text(), returns various types of edits or current text, specifying what kind of text is returned using an enum. However, this function is designed to give ownership of the data in order to avoid cloning large amounts of data, and thus will destroy parts of the contents of this instance.
    fn take_text(&mut self, of_type: &SaveType) -> String {
        match of_type {
            SaveType::SentenceAdditions => mem::take(&mut self.sentence_additions),
            SaveType::SentenceEdits => mem::take(&mut self.sentence_edits),
            SaveType::WordAdditions => mem::take(&mut self.word_additions),
            SaveType::WordDeletions => mem::take(&mut self.word_deletions),
            SaveType::Text => mem::take(&mut self.text),
        }
    }
    pub fn get_all_edits(&self) -> HashMap<String, &String> {
        SaveType::iter().map(|x| (x.to_string(), x)).fold(
            HashMap::new(),
            |mut map, (name, savetype)| {
                map.insert(name, Self::get_text(&self, &savetype));
                map
            },
        )
    }
    /// Consumes instance and returns all the data contained within as a hashmap with minimal performance overhead by avoiding cloning data. The keys are the type of data, and the values are the text data.
    pub fn extract_all_edits(mut self) -> HashMap<String, String> {
        SaveType::iter().map(|x| (x.to_string(), x)).fold(
            HashMap::new(),
            |mut map, (name, savetype)| {
                map.insert(name, Self::take_text(&mut self, &savetype));
                map
            },
        )
    }
}

// Update this enum to include new save types if any new output text types are added. This will allow R to properly load whatever new data is being produced.
#[derive(Debug, Display, EnumIter, Clone)]
pub enum SaveType {
    #[strum(serialize = "SentenceAdditions")]
    SentenceAdditions,
    #[strum(serialize = "SentenceEdits")]
    SentenceEdits,
    #[strum(serialize = "WordAdditions")]
    WordAdditions,
    #[strum(serialize = "WordDeletions")]
    WordDeletions,
    #[strum(serialize = "Text")]
    Text,
}

impl SaveType {
    /// Output a vector containing the filenames of each types of data saved, such as sentence-level additions and edits.
    pub fn list_savetypes() -> Vec<String> {
        Self::iter().map(|x| x.to_string()).collect()
    }
}
