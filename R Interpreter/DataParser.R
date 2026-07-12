# Returns a List, with an entry for every person in the database. Each list entry is a data table containing each edit type at every time period for that person.
# This function takes the output path given to the Rust program as input. In other words, input the path to the directory containing the formatted temporal data.
# When set to its default of TRUE, the insert_na output will insert a row of NAs whenever a snapshot is missing during a specific time period 
get_temporal_doc_data <- function(path, insert_na = TRUE) {
  # Ensure that the provided path ends with a slash
  if (substr(path, nchar(path), nchar(path)) != "/") {
    path <- paste(path, "/", sep = "")
  }
  # Read summary files
  people_info <- read.csv(paste(path, "PeopleInfo.csv", sep = ""), header = FALSE)
  savetypes <- read.csv(paste(path, "SaveType.csv", sep = ""), header = FALSE)
  all_times <- read.csv(paste(path, "TimePeriods.csv", sep = ""), header = FALSE)
  # Initialize data structure
  people_change_data <- list()
  # Get data about each person
  for (person in people_info[,1]) {
    # Get a list of the time periods of all the snapshots from a specific person, along with data about those snapshots
    times <- read.csv(paste(path, "People/",person,"/timeperiod.csv", sep = ""))
    # Insert rows of NA into the data about each snapshot, depending on if user is using this feature
    if (insert_na) {
      new_times <- times[0,]
      for (i in all_times[,1]) {
        if (i %in% times[,1]) {
          new_times <- rbind(new_times, subset(times, times[,1] == i))
        } else {
          temp <- c(i ,rep(NA, (ncol(times) - 1) ))
          new_times <- rbind(new_times, temp)
          colnames(new_times) <- colnames(times)
        }
      }
    }
    # Initialize each person's data structure
    temp_data_frame <- data.frame(matrix(ncol = length(savetypes[,1]), nrow = 0))
    # Get data about each time period
    for (time in all_times[,1]) {
      # If this snapshot for this time period is not missing, add data about the snapshot to the person's data
      if (time %in% times[,1]){
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
      } else if (insert_na) { # If this time period's snapshot is missing, insert NAs or ignore depending on user input
        temp_data_frame <- rbind(temp_data_frame, rep(NA, length(savetypes[,1])))
      }
    }
    # Collect data into a coherent list of data frames, adjusting data frame dimensions depending on whether NAs were included.
    colnames(temp_data_frame) <- savetypes[,1]
    if (insert_na) {
      rownames(temp_data_frame) <- all_times[,1]
      temp_data_frame <- cbind(temp_data_frame, new_times[,2:length(colnames(new_times))])
    } else {
      rownames(temp_data_frame) <- times[,1]
      temp_data_frame <- cbind(temp_data_frame, times[,2:length(colnames(times))])
    }
    people_change_data[[length(people_change_data) + 1]] <- temp_data_frame
  }
  # Name each person in the output list of people
  names(people_change_data) <- people_info[,1]
  # Return data
  return(people_change_data)
}

# Returns a List, with an entry for every person in the database. Each list entry is string containing the latest saved version of that person's writing.
# This function takes the list outputted by get_temporal_doc_data() as input.
# The skip_na input will return the last non-NA text when set to it's default of TRUE.
get_final_doc_version <- function(object, skip_na = TRUE) {
  # Initialize data structure
  people_final_text <- c()
  # Iterate through each person to find the final version of their document
  for (i in 1:length(object)) {
    if (skip_na) { # If NAs are to be skipped, filter out NAs and find latest version of the document
      texts <- object[[i]][["Text"]]
      people_final_text[i] <- tail(unlist( texts[which(!is.na(texts))] ), n=1)
    } else { # If NAs are to be included, find the latest version of the document
      people_final_text[i] <- tail(unlist(object[[i]][["Text"]]), n=1)
    }
  }
  # Name each person in the output list of people
  names(people_final_text) <- names(object)
  # Return data
  return(people_final_text)
}

