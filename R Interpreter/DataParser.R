path <- "/Users/anyll/Documents/My files/  Topics/Work/Work Files/CU Boulder/Andy Martin/Evolutionary Biology/Refined Notebook Code/Outputs/Output Tests Generation 1/Test 2/"

# Returns a List, with an entry for every person in the database. Each list entry is a data table containing each edit type at every time period for that person.
# This function takes the output path given to the Rust program as input. In other words, input the path to the directory containing the formatted temporal data.
get_temporal_doc_data <- function(path) {
  people_info <- read.csv(paste(path, "PeopleInfo.csv", sep = ""), header = FALSE)
  savetypes <- read.csv(paste(path, "SaveType.csv", sep = ""), header = FALSE)
  people_change_data <- list()
  
  for (person in people_info[,1]) {
    times <- read.csv(paste(path, "People/",person,"/timeperiod.csv", sep = ""), header = TRUE)
    #print(times)
    temp_data_frame <- data.frame(matrix(ncol = length(savetypes[,1]), nrow = 0))
    for (time in times[,1]) {
      row <- c()
      for (savetype in savetypes[,1]) {
        text_path <- paste(path, "People/",person,"/Times/", time, "/", savetype,".txt", sep = "")
        text <- readChar(text_path, file.info(text_path)$size)
        if (is.null(text)) {
          text <- NA
        }
        row <- c(row, text)
      }
      temp_data_frame <- rbind(temp_data_frame, row)
    }
    colnames(temp_data_frame) <- savetypes[,1]
    rownames(temp_data_frame) <- times[,1]
    people_change_data[[length(people_change_data) + 1]] <- temp_data_frame
  }
  names(people_change_data) <- people_info[,1]
  return(people_change_data)
}

# Returns a List, with an entry for every person in the database. Each list entry is string containing the latest saved version of that person's writing.
# This function takes the output path given to the Rust program as input. In other words, input the path to the directory containing the formatted temporal data.
get_final_doc_version <- function(path) {
  people_info <- read.csv(paste(path, "PeopleInfo.csv", sep = ""), header = FALSE)
  people_final_text <- list()
  
  for (person in people_info[,1]) {
    final_text_path <- paste(path, "People/",person,"/FinalText.txt", sep = "")
    people_final_text[[length(people_final_text) + 1]] <- readChar(final_text_path, file.info(final_text_path)$size)
  }
  names(people_final_text) <- people_info[,1]
  return(people_final_text)
}

