use std::cmp;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::PathBuf;

use anyhow::{self, Context};
use structopt::StructOpt;
use thiserror::Error;

use tapr::constants::*;
use tapr::formatter::*;
use tapr::safe_terminal_size::*;
use tapr::table_reader::*;
use tapr::utils::*;

fn determine_column_widths(
    line_cells: Vec<&[String]>,
    linenum_width: usize,
    terminal_width: usize,
) -> Result<Vec<usize>, DetColumnWidthError> {
    let column_width_minmedmaxs = get_raw_column_widths(&line_cells);
    if linenum_width > 0 {
        det_print_column_widths(
            &column_width_minmedmaxs,
            terminal_width - (linenum_width + *frame::CHAR_WIDTH),
        )
    } else {
        det_print_column_widths(&column_width_minmedmaxs, terminal_width)
    }
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

#[derive(Error, Debug)]
pub enum TaprError {
    #[error("options --csv and --tsv are mutually exclusive")]
    OptionsCSVAndTSVAreMutuallyExclusive,

    #[error("line {}: decode error", .linenum)]
    DecodeError { linenum: usize },
}

fn main() -> anyhow::Result<()> {
    assert!(*frame::CHAR_WIDTH == 1);

    let opt = Opt::from_args();
    // println!("{:#?}", opt);
    if opt.csv && opt.tsv {
        return Err(TaprError::OptionsCSVAndTSVAreMutuallyExclusive.into());
    }
    let line_sampling = (opt.line_sampling == 0).q(1, opt.line_sampling);

    // get terminal width
    let size = safe_terminal_size();
    let (Width(width), _) = size.with_context(|| "fail to detect terminal width")?;
    let terminal_width: usize = width as usize;

    // read input lines
    let input_file = opt.input.clone().into_os_string().into_string().unwrap();
    let mut lines: Vec<String> = vec![];
    if input_file == "-" {
        let stdin = io::stdin();
        for (li, line) in stdin.lock().lines().enumerate() {
            let line = line.context(TaprError::DecodeError { linenum: li + 1 })?;
            lines.push(line);
        }
    } else {
        let fp =
            File::open(opt.input).with_context(|| format!("fail to open file: {}", input_file))?;
        for (li, line) in io::BufReader::new(fp).lines().enumerate() {
            let line = line.context(TaprError::DecodeError { linenum: li + 1 })?;
            lines.push(line);
        }
    };

    // if input is empty, then exits without printing something
    if lines.is_empty() {
        return Ok(());
    }

    // calculate the width for line number (if needed)
    let linenum_width = opt.line_number.q((lines.len()).to_string().len(), 0);

    // determine width of each column
    let includes_tab = lines.iter().any(|line| line.contains('\t'));
    let cell_separator = (opt.csv || !opt.tsv && !includes_tab).q(',', '\t');
    let split_to_cells: fn(usize, &str) -> Result<Vec<String>, _> =
        (cell_separator == ',').q(split_csv_line, split_tsv_line);
    let lines_sampled = &lines[..cmp::min(lines.len(), line_sampling)];
    let mut line_cells_sampled: Vec<Vec<String>> = vec![];
    for (li, line) in lines_sampled.iter().enumerate() {
        let cells = split_to_cells(li, line)?;
        line_cells_sampled.push(cells);
    }
    let line_cells_sampled: Vec<&[String]> =
        line_cells_sampled.iter().map(|lc| lc.as_ref()).collect();
    let column_widths: Vec<usize> =
        determine_column_widths(line_cells_sampled, linenum_width, terminal_width)?;

    // print lines as a table
    print_horizontal_line(frame::CROSSING_TOP, &column_widths, linenum_width);
    if opt.header {
        for (li, line) in lines.iter().enumerate() {
            let line = if line.is_empty() { " " } else { line };
            let cells = split_to_cells(li, line)?;
            print_line(li, &cells, &column_widths, linenum_width);
            if li == 0 {
                print_horizontal_line(frame::CROSSING_MIDDLE, &column_widths, linenum_width);
            }
        }
    } else {
        for (li, line) in lines.iter().enumerate() {
            let line = if line.is_empty() { " " } else { line };
            let cells = split_to_cells(li, line)?;
            print_line(li + 1, &cells, &column_widths, linenum_width);
        }
    }
    print_horizontal_line(frame::CROSSING_BOTTOM, &column_widths, linenum_width);

    Ok(())
}
