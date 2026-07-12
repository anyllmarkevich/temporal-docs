# Returns a List, with an entry for every person in the database. Each list entry is a data table containing each edit type at every time period for that person.
# This function takes the output path given to the Rust program as input. In other words, input the path to the directory containing the formatted temporal data.
get_temporal_doc_data <- function(path, insert_na = TRUE) {
  if (substr(path, nchar(path), nchar(path)) != "/") {
    path <- paste(path, "/", sep = "")
  }
  people_info <- read.csv(paste(path, "PeopleInfo.csv", sep = ""), header = FALSE)
  savetypes <- read.csv(paste(path, "SaveType.csv", sep = ""), header = FALSE)
  all_times <- read.csv(paste(path, "TimePeriods.csv", sep = ""), header = FALSE)
  people_change_data <- list()
  
  for (person in people_info[,1]) {
    times <- read.csv(paste(path, "People/",person,"/timeperiod.csv", sep = ""))
    
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
    
    temp_data_frame <- data.frame(matrix(ncol = length(savetypes[,1]), nrow = 0))
    for (time in all_times[,1]) {
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
      } else if (insert_na) {
        temp_data_frame <- rbind(temp_data_frame, rep(NA, length(savetypes[,1])))
      }
    }
    if (insert_na) {
      colnames(temp_data_frame) <- savetypes[,1]
      rownames(temp_data_frame) <- all_times[,1]
      temp_data_frame <- cbind(temp_data_frame, new_times[,2:length(colnames(new_times))])
      people_change_data[[length(people_change_data) + 1]] <- temp_data_frame
    } else {
      colnames(temp_data_frame) <- savetypes[,1]
      rownames(temp_data_frame) <- times[,1]
      temp_data_frame <- cbind(temp_data_frame, times[,2:length(colnames(times))])
      people_change_data[[length(people_change_data) + 1]] <- temp_data_frame
    }
  }
  names(people_change_data) <- people_info[,1]
  return(people_change_data)
}

# Returns a List, with an entry for every person in the database. Each list entry is string containing the latest saved version of that person's writing.
# This function takes the list outputted by get_temporal_doc_data() as input.
get_final_doc_version <- function(object) {
  people_final_text <- c()
  for (i in 1:length(object)) {
    people_final_text[i] <- tail(unlist(object[[i]][["Text"]]), n=1)
  }
  names(people_final_text) <- names(object)
  return(people_final_text)
}

