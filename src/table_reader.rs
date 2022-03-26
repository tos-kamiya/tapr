use csv::ReaderBuilder;

pub fn split_csv_line(li: usize, line: &str) -> Vec<String> {
    let mut rdr = ReaderBuilder::new()
        .delimiter(b',')
        .has_headers(false)
        .from_reader(line.as_bytes());
    if let Some(result) = rdr.records().next() {
        let record = result.unwrap_or_else(|_e| {
            eprintln!("Error: line {}: invalid text: {}", li + 1, line);
            std::process::exit(1);
        });
        let v: Vec<String> = record.iter().map(|item| item.to_string()).collect();
        v
    } else {
        unreachable!();
    }
}

pub fn split_tsv_line(_li: usize, line: &str) -> Vec<String> {
    let v: Vec<String> = line.split('\t').map(|item| item.to_string()).collect();
    v
}
