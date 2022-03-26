use std::cmp;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::PathBuf;

use structopt::StructOpt;

use tapr::constants::*;
use tapr::formatter::*;
use tapr::safe_terminal_size::*;
use tapr::table_reader::*;

fn determine_column_widths(
    line_cells_sampled: Vec<&[String]>,
    linenum_width: usize,
    terminal_width: usize,
) -> Vec<usize> {
    let column_width_minmedmaxs = get_raw_column_widths(&line_cells_sampled);
    let cws = if linenum_width > 0 {
        det_print_column_widths(
            &column_width_minmedmaxs,
            terminal_width - (linenum_width + *FRAME_CHAR_WIDTH),
        )
    } else {
        det_print_column_widths(&column_width_minmedmaxs, terminal_width)
    };
    cws.unwrap_or_else(|| {
        eprintln!("Error: terminal width too small for input table.");
        std::process::exit(1);
    })
}

/// Table Pretty-print. print TSV or CSV file.
#[derive(StructOpt, Debug)]
#[structopt(name = "tapr")]
struct Opt {
    /// Force treats input file as CSV
    #[structopt(short = "c", long)]
    csv: bool,

    /// Force treats input file as TSV
    #[structopt(short = "t", long)]
    tsv: bool,

    /// Prints line number
    #[structopt(short = "n", long)]
    line_number: bool,

    /// Prints first line as a header
    #[structopt(short = "H", long)]
    header: bool,

    /// Sampling size of lines to determine width of each column
    #[structopt(long = "line-sampling", default_value = "100")]
    line_sampling: usize,

    /// Input file. Specify `-` to read from the standard input.
    #[structopt(parse(from_os_str))]
    input: PathBuf,
}

fn main() {
    assert!(*FRAME_CHAR_WIDTH == 1);

    let opt = Opt::from_args();
    // println!("{:#?}", opt);
    if opt.csv && opt.tsv {
        eprintln!("Error: option --csv and --tsv are mutually exclusive");
        std::process::exit(1);
    }
    let line_sampling = if opt.line_sampling == 0 {
        1
    } else {
        opt.line_sampling
    };

    // get terminal width
    let size = safe_terminal_size();
    let (Width(width), _) = size.unwrap();
    let terminal_width: usize = width as usize;

    // read input lines
    let input_file = opt.input.clone().into_os_string().into_string().unwrap();
    let lines: Vec<String> = if input_file == "-" {
        let stdin = io::stdin();
        stdin.lock().lines().map(|line| line.unwrap()).collect()
    } else {
        if let Ok(fp) = File::open(opt.input) {
            io::BufReader::new(fp)
                .lines()
                .map(|line| line.unwrap())
                .collect()
        } else {
            eprintln!("Error: fail to open file: {}", input_file);
            std::process::exit(1);
        }
    };

    if lines.is_empty() {
        return;
    }

    // determine cell separator
    let includes_tab = lines.iter().any(|line| line.contains('\t'));
    let cell_separator = if opt.csv || !opt.tsv && !includes_tab {
        ','
    } else {
        '\t'
    };
    let split_to_cells = if cell_separator == ',' {
        split_csv_line
    } else {
        split_tsv_line
    };

    // calculate the width for line number (if needed)
    let linenum_width = if opt.line_number {
        (lines.len()).to_string().len()
    } else {
        0
    };

    // determine width of each column
    let line_cells_sampled: Vec<Vec<String>> = lines[..cmp::min(lines.len(), line_sampling)]
        .iter()
        .enumerate()
        .map(|(li, line)| split_to_cells(li, line))
        .collect();
    let line_cells_sampled: Vec<&[String]> =
        line_cells_sampled.iter().map(|lc| lc.as_ref()).collect();
    let column_widths: Vec<usize> =
        determine_column_widths(line_cells_sampled, linenum_width, terminal_width);

    // print lines as a table
    print_horizontal_line(TMB::Top, &column_widths, linenum_width);
    if opt.header {
        for (li, line) in lines.iter().enumerate() {
            let line = if line.is_empty() { " " } else { line };
            let cells = split_to_cells(li, line);
            print_line(li, &cells, &column_widths, linenum_width);
            if li == 0 {
                print_horizontal_line(TMB::Middle, &column_widths, linenum_width);
            }
        }
    } else {
        for (li, line) in lines.iter().enumerate() {
            let line = if line.is_empty() { " " } else { line };
            let cells = split_to_cells(li, line);
            print_line(li + 1, &cells, &column_widths, linenum_width);
        }
    }
    print_horizontal_line(TMB::Bottom, &column_widths, linenum_width);
}
