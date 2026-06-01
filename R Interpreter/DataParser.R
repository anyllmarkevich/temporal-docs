path <- "/Users/anyll/Documents/My files/  Topics/Work/Work Files/CU Boulder/Andy Martin/Evolutionary Biology/Refined Notebook Code/Outputs/Output Tests Generation 1/Test 1/"

people_info <- read.csv(paste(path, "PeopleInfo.csv", sep = ""), header = FALSE)

savetypes <- c("SentenceAdditions", "SentenceEdits", "WordAdditions", "WordDeletions")
people_change_data <- list()
people_final_text <- list()

for (person in people_info[,1]) {
  times <- read.csv(paste(path, "People/",person,"/timeperiod.csv", sep = ""), header = TRUE)
  #print(times)
  temp_data_frame <- data.frame(matrix(ncol = length(savetypes), nrow = 0))
  for (time in times[,1]) {
    row <- c()
    for (savetype in savetypes) {
      text_path <- paste(path, "People/",person,"/Times/", time, "/", savetype,".txt", sep = "")
      text <- readChar(text_path, file.info(text_path)$size)
      if (is.null(text)) {
        text <- NA
      }
      row <- c(row, text)
    }
    temp_data_frame <- rbind(temp_data_frame, row)
  }
  colnames(temp_data_frame) <- savetypes
  rownames(temp_data_frame) <- times[,1]
  people_change_data[[length(people_change_data) + 1]] <- temp_data_frame
  final_text_path <- paste(path, "People/",person,"/FinalText.txt", sep = "")
  people_final_text[[length(people_final_text) + 1]] <- readChar(final_text_path, file.info(final_text_path)$size)
  #people_change_data <- append(people_change_data, temp_data_frame)
}
names(people_change_data) <- people_info[,1]
names(people_final_text) <- people_info[,1]
print(people_change_data)
print(people_final_text)

