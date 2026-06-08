# Temporal Document Analyzer
This repository converts organized snapshots of Microsoft Word files from multiple authors into a temporal data about edits, additions, and deletions made by each author between snapshots, all nicely formatted in R objects to aide analysis. Note this program **does not** provide the detailed timestamp data logged by the "track changes" feature common among most word processing software. This program is intended as a coarse and easily managed data extraction tool, not a document history tracker.

Although this functionality is conceptually useful in many fields, our aim is to aide educators, who are often faced with the challenge of understanding student educational journeys beyond the reductive metric of assignment letter grades. This repository can help educators and education researchers gain insights into what students are learning, the challenges they face, and the unique trajectories of every student by making temporal data about student notebooks readily accessible for analysis in R.
## Pedagogical Setup
This program requires regular snapshots of student notebooks, formatted as Microsoft Word files. We recommend that educators assign specific notebooks to students through Google Drive or another similar file sharing platform, ensuring that the educator can easily download the notebooks periodically. Each student notebook should be uniquely named. The educator can then periodically save the notebooks into sequentially named folders for each time period. The Rust program provided in this repository can then easily extract text written, deleted, or edited during each time period, saving it into another folder that the accompanying R program can read. After calling a couple of functions in R, educators will have access to a wealth of information on the text student's wrote in their notebooks throughout the class.
## Using Temporal Documents
### Saving Source Documents
### Using the Rust Crate
### Reading Data in R
Make sure to include a copy of the `DataParser.R` file in you R project. Load the file into your own R code using `source(DataParser.R)`. Simply input the output file path you set earlier into the two following functions, save the output, and access the data for your own analyses.
- `get_temporal_doc_data(path)` returns a list of every person who wrote a document. Each list item contains a data table where rows are time periods (defined by the document snapshots downloaded by the user) and the columns are different types of text identified as edited since the preceding time period.
- `get_final_doc_version(path)` returns a list of every person who wrote a document where each list item contains the latest downloaded version of their writing.
### Examples
## Installation
