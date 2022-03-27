use csv::ReaderBuilder;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SplitLineError {
    #[error("line: {}, invalid text: {}", .linenum, .text)]
    InvalidText { linenum: usize, text: String },
}

pub fn split_csv_line(li: usize, line: &str) -> Result<Vec<String>, SplitLineError> {
    let mut rdr = ReaderBuilder::new()
        .delimiter(b',')
        .has_headers(false)
        .from_reader(line.as_bytes());
    if let Some(result) = rdr.records().next() {
        if let Ok(record) = result {
            Ok(record.iter().map(|item| item.to_string()).collect())
        } else {
            Err(SplitLineError::InvalidText {
                linenum: li + 1,
                text: line.to_string(),
            })
        }
    } else {
        Ok(vec![])
    }
}

pub fn split_tsv_line(_li: usize, line: &str) -> Result<Vec<String>, SplitLineError> {
    Ok(line.split('\t').map(|item| item.to_string()).collect())
}
