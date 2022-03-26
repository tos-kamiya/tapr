use std::fs::File;
use std::io;
use std::io::BufRead;
use std::path::PathBuf;

use structopt::StructOpt;
use terminal_size::{Width, terminal_size};

use tapr::constants::*;
use tapr::table_reader::*;
use tapr::formatter::*;

/// Table Pretty-print. print TSV or CSV file.
#[derive(StructOpt, Debug)]
#[structopt(name = "tapr")]
struct Opt {
    /// Force treat input as CSV
    #[structopt(short = "c", long)]
    csv: bool,

    /// Force treat input as TSV
    #[structopt(short = "t", long)]
    tsv: bool,

    /// Print line number
    #[structopt(short = "n", long)]
    line_number: bool,

    /// Print first line as a header
    #[structopt(short = "H", long)]
    header: bool,

    /// Input file
    #[structopt(parse(from_os_str))]
    input: Option<PathBuf>,
}

fn main() {
    assert!(*FRAME_CHAR_WIDTH == 1);

    let opt = Opt::from_args();
    // println!("{:#?}", opt);
    if opt.csv && opt.tsv {
        eprintln!("Error: option --csv and --tsv are mutually exclusive");
        std::process::exit(1);
    }

    // get terminal width
    let size = terminal_size();
    let (Width(width), _) = size.unwrap();
    let terminal_width: usize = width as usize;

    // read input lines
    let lines: Vec<String> = match opt.input {
        None => {
            let stdin = io::stdin();
            stdin.lock().lines().map(|line| line.unwrap()).collect()
        },
        Some(f) => {
            let f0 = f.clone();
            if let Ok(fp) = File::open(f) {
                io::BufReader::new(fp).lines().map(|line| line.unwrap()).collect()
            } else {
                let f0 = f0.into_os_string().into_string().unwrap();
                eprintln!("Error: fail to open file: {}", f0);
                std::process::exit(1);
            }
        },
    };

    // determine cell separator
    let includes_tab = lines.iter().any(|line| line.contains('\t'));
    let cell_separator = if opt.csv || ! opt.tsv && ! includes_tab { ',' } else { '\t' };

    // calculate the width for line number (if needed)
    let linenum_width = if opt.line_number {
        (lines.len()).to_string().len()
    } else {
        0
    };

    // split each line into cells
    let line_cells: Vec<Vec<String>> = if cell_separator == ',' {
        lines.iter().enumerate().map(|(li, line)| split_csv_line(li, line)).collect()
    } else {
        lines.iter().enumerate().map(|(li, line)| split_tsv_line(li, line)).collect()
    };
    drop(lines);
    let line_cells: Vec<&[String]> = line_cells.iter().map(|lc| lc.as_ref()).collect();

    // determine width of each column
    let column_width_minmedmaxs = get_raw_column_widths(&line_cells);
    let cws = if linenum_width > 0 {
        det_print_column_widths(&column_width_minmedmaxs, terminal_width - (linenum_width + *FRAME_CHAR_WIDTH))
    } else {
        det_print_column_widths(&column_width_minmedmaxs, terminal_width)
    };
    let column_widths = cws.unwrap_or_else(|| {
        eprintln!("Error: terminal width too small for input table.");
        std::process::exit(1);
    });

    // print lines as a table
    print_horizontal_line(TMB::Top, &column_widths, linenum_width);
    if opt.header {
        for (li, cells) in line_cells.iter().enumerate() {
            print_line(li, cells, &column_widths, linenum_width);
            if li == 0 {
                print_horizontal_line(TMB::Middle, &column_widths, linenum_width);
            }
        }
    } else {
        for (li, cells) in line_cells.iter().enumerate() {
            print_line(li + 1, cells, &column_widths, linenum_width);
        }
    }
    print_horizontal_line(TMB::Bottom, &column_widths, linenum_width);
}
