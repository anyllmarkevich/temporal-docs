# Temporal Document Analyzer
This repository converts organized snapshots of Microsoft Word files from multiple authors into a temporal data about edits, additions, and deletions made by each author between snapshots, all nicely formatted in R objects to aide analysis. 
> **Note:** this program ***does not*** provide the detailed timestamp data logged by the "track changes" feature common among most word processing software. This program is intended as a coarse and easily managed data extraction tool, not a document history tracker.

Although this functionality is conceptually useful in many fields, our aim is to aide educators, who are often faced with the challenge of understanding student educational journeys beyond the reductive metric of assignment letter grades. This repository can help educators and education researchers gain insights into what students are learning, the challenges they face, and the unique trajectories of every student by making temporal data about student notebooks readily accessible for analysis in R.

This project relies on the [similar](https://github.com/mitsuhiko/similar) crate for identifying changes between documents and the [unicode_segmentation](https://github.com/unicode-rs/unicode-segmentation) crate for identifying word and sentence boundaries.
## Pedagogical Setup
This program requires regular snapshots of student notebooks, formatted as Microsoft Word files. We recommend that educators assign specific notebooks to students through Google Drive or another similar file sharing platform, ensuring that the educator can easily download the notebooks periodically. Each student notebook should be uniquely named. The educator can then periodically save the notebooks into sequentially named folders for each time period. The Rust program provided in this repository can then easily extract text written, deleted, or edited during each time period, saving it into another folder that the accompanying R program can read. After calling a couple of functions in R, educators will have access to a wealth of information about the text written by students in their notebooks throughout the class.
## Using Temporal Documents
### Saving Source Documents
Create a folder where you would like to store the snapshots of people's documents. Periodically create a subfolder and alphanumerically label it to indicate the sequential order of the snapshots. Download the latest version of the documents you wish to track of (note that Microsoft Word and downloaded Google Docs documents are supported as they use the same file format) and save them into this subfolder. Make sure to keep the names of each person's document identical across subfolders! Repeat the process of creating the folder and saving the latest version of the documents into the newest folder.
The input directory to the program should be formatted as follows:
```
Data_Folder
├── timeperiod_001
│   ├── Alice_Doc_snapshot.docx
│   └── Bob_Doc_snapshot.docx
├── timeperiod_002
│   ├── Alice_Doc_snapshot.docx
│   └── Bob_Doc_snapshot.docx
└── timeperiod_003
    ├── Alice_Doc_snapshot.docx
    └── Bob_Doc_snapshot.docx
```
Here are important formatting rules to keep in mind:
- Snapshot folders must be alphanumerically arranged in chronological order.
    - This means that numbers should be formatted as `001`, `002`, `003`, etc. This is critical because the program will correctly interpret `009` as coming before `010`, but will incorrectly place `10` before `9`, breaking the underlying edits algorithm.
    - Dates should follow a Year-Month-Day pattern. `2012-12-31` will work great, but `December 31 2012`, `30-12-2012`, or `12-30-2012` may all cause issues.
- All documents attributable to the same author should have the exact same unique filename. Otherwise, they will be counted as different people or data from multiple people will be merged together.
- Currently, each person may only have one document listed in each time period.
### Using the Rust Crate
Once you've installed the program, run it using this command (replacing the example file paths with your own): `temporal_docx "path/to/input_data" "path/to/where_you_wish_to_save_output"`. This will be a lot easier very soon!
### Reading Data in R
Make sure to include a copy of the `DataParser.R` file in your R project (download from the dropdown menu on [this page](https://github.com/anyllmarkevich/temporal-docs/blob/main/R%20Interpreter/DataParser.R)). Load the file into your own R code using `source(DataParser.R)`. To load your data into the R program, run the `get_temporal_doc_data(path, insert_na = TRUE)` function, inputting the output path you set when running the Rust program. The resulting object can be accessed directly or using one of the functions that take an object as input. Changing the default `insure_na = TRUE` input to `insert_na = FALSE` will prevent the program from inserting null values for time periods where no snapshots of a person's document were saved (missing snapshots).
#### Importing Data
`get_temporal_doc_data(path, skip_na = TRUE)` returns a list of every person who wrote a document, using a filepath to the output of the Rust program as input. Each list item contains a data table where rows are time periods (defined by the document snapshots downloaded by the user) and the columns are different types of data or metadata. If `NA` values were inserted when fetching the data, setting the `skip_na` input to its default value of `TRUE` will return the latest non-missing version of a document. Else, the latest version may be an `NA` value.

The currently available data columns are:
- `SentenceAdditions`: All Unicode sentences that contained newly written text since the last time period.
- `SentenceEdits`: All Unicode sentences where text was added or deleted since the last time period.
- `WordAdditions`: All newly written Unicode words since the last time period.
- `WordDeletions`: All deleted Unicode words since the last time period.
- `Text`: The current state of the entire document.
- `filesize`: The size of this original document snapshot in bytes (includes images and other data not considered by this program).
- `word_count`: The number of Unicode words in this document snapshot.
- `sentence_count`: The number of Unicode sentences in this document snapshot.
#### Analyzing Data
Although the list object created by `get_temporal_doc_data(path)` can be accessed directly for analysis, built-in functions can help quickly extract useful information form this data structure.

The currently available functions are:
- `get_final_doc_version(object)` returns a vector of every person who wrote a document where each item contains the latest downloaded version of their writing.
## Installation
This program is currently only tested on MacOS. It probably runs fine on Linux, but it does not currently support Windows (as far as I know — I haven't tried!).
Begin by installing [Rust](https://rust-lang.org), ideally through [rustup](https://rustup.rs). Afterward, run `cargo install temporal_docx` in your Command Line Interface.
If you wish to import the output data into R, download the R script built for this purpose from the dropdown menu on [this page](https://github.com/anyllmarkevich/temporal-docs/blob/main/R%20Interpreter/DataParser.R).
You're all set!
### From Source
Begin by installing [Rust](https://rust-lang.org/tools/install/) through [rustup](https://rustup.rs) and [R](https://www.r-project.org) (I also recommend installing [R Studio](https://posit.co/downloads)). Download the source code from this repository. Open a console window in `Rust Parser/temporal_document_analyzer` and compile the program using `cargo run` (see the "Using the Rust Crate" section). Add inputs to the program like this: `cargo run -- "path/to/input_data" "path/to/where_you_wish_to_save_output"`.
## Examples
Consider the following example to understand the current behavior of the parsing algorithm (which is subject to change).
**Original Text at Time 1:**

```
This is the original text. This sentence will be deleted. A sentence about birds will be added after this sentence. The word deer will be deleted. This sentence will have the name of an amphibian added. The word tree will be pluralized. The word wolves and the word eels will be converted to the singular form. The word frogs will be changed to the name for a type of water vapor.
This is a new paragraph. A sentence about fish will be added to the beginning of this paragraph.
```

**Original Text at Time 2:**

```
This is the original text. A sentence about birds will be added after this sentence. Birds are cool! The word will be deleted. This sentence will have the name of an amphibian salamander added. The word trees will be pluralized. The word wolf and the word eel will be converted to the singular form. The word fog will be changed to the name for a type of water vapor.
Fish are a diverse group of species. This is a new paragraph. A sentence about fish will be added to the beginning of this paragraph.
```

These are the outputs given by the program for Time 2 (these are specifically the outputs of the `text_edits` module in the Rust crate, which can be reused by other Rust programs):
**SentenceAdditions:**

```
Birds are cool! The word will be deleted. This sentence will have the name of an amphibian salamander added. The word trees will be pluralized. The word wolf and the word eel will be converted to the singular form. The word fog will be changed to the name for a type of water vapor. Fish are a diverse group of species.
```

**SentenceEdits:**

```
This sentence will be deleted. The word deer will be deleted. This sentence will have the name of an amphibian added. The word tree will be pluralized. The word wolves and the word eels will be converted to the singular form. The word frogs will be changed to the name for a type of water vapor. Birds are cool! The word will be deleted. This sentence will have the name of an amphibian salamander added. The word trees will be pluralized. The word wolf and the word eel will be converted to the singular form. The word fog will be changed to the name for a type of water vapor. Fish are a diverse group of species.
```

**WordAdditions:**

```
Birds are cool salamander trees wolf eel fog Fish are a diverse group of species
```

**WordDeletions:**

```
This sentence will be deleted deer tree wolves eels frogs
```

Notice that the sentence-level changes capture the entire sentence, even if the edit is limited to one or a few words. These two outputs are useful when trying to capture the context around edits or when running text analysis algorithms that rely on complete sentences or correct grammar. Please also notice that word deletions occur whenever a word is changed. I am considering changing this behavior in the future — please leave an issue or a pull request if you would like deletion data to be less lenient.
## Contributing
You are welcome to leave issues with any bugs you have encountered or feature requests you may have. Or better yet, leave a pull request implementing these features! I will do my best to get to bugs, features requests, and pull requests as soon as possible! However, please do not submit fully AI-generated pull requests or bug reports. It's fine if you use an LLM to help you write code: I just don't want to review fully AI-written material. Thank you!
