#[macro_use]
extern crate lazy_static;

use std::fs::File;
use std::io;
use std::io::BufRead;
use std::path::PathBuf;
use std::str;

use structopt::StructOpt;
use terminal_size::{Width, terminal_size};
use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

const MAX_UNFOLDED_COLUMN_WIDTH: usize = 7;
const ANSI_ESCAPE_HEADER_COLOR: &[&str] = &["\u{1b}[37m", "\u{1b}[37m"];
const ANSI_ESCAPE_TEXT_COLOR: &[&str] = &["\u{1b}[34m", "\u{1b}[32m"];
const ANSI_ESCAPE_FRAME_COLOR: &str = "\u{1b}[90m";
const ANSI_ESCAPE_RESET_COLOR: &str = "\u{1b}[0m";

const FRAME_VERTICAL: &str = "\u{2502}";
const FRAME_HORIZONTAL: &str = "\u{2500}";
const FRAME_CROSS_TOP: &str = "\u{252c}";
const FRAME_CROSS_BOTTOM: &str = "\u{2534}";
const FRAME_CROSS_MIDDLE: &str = "\u{253c}";

// const FRAME_VERTICAL: &str = "\u{2503}";
// const FRAME_HORIZONTAL: &str = "\u{2501}";
// const FRAME_CROSS_TOP: &str = "\u{2533}";
// const FRAME_CROSS_BOTTOM: &str = "\u{253b}";
// const FRAME_CROSS_MIDDLE: &str = "\u{254b}";

lazy_static! {
    static ref FRAME_CHAR_WIDTH: usize = UnicodeWidthStr::width(FRAME_VERTICAL);
}

fn str_width(s: &str) -> usize {
    let mut w: usize = 0;
    for ss in UnicodeSegmentation::graphemes(s, true) {
        w += UnicodeWidthStr::width(ss);
    }
    w
}

fn print_str_bar(s: &str, repeat: usize) {
    if repeat > 0 {
        print!("{}", (0..repeat).map(|_| s).collect::<String>());
    }
}

#[derive(Copy, Clone, Debug)]
struct MinMedMax(usize, usize, usize);

fn get_column_widths<S: AsRef<str>>(lines: &[S], cell_separator: char) -> Vec::<MinMedMax> {
    let lines_len = lines.len();

    let mut column_width_lists: Vec<Vec<usize>> = vec![];
    for (ri, line) in lines.iter().enumerate() {
        let line = line.as_ref();
        for (ci, field) in line.split(cell_separator).enumerate() {
            if ci >= column_width_lists.len() {
                column_width_lists.push(vec![0; lines_len]);
            }
            let w = str_width(field);
            column_width_lists[ci][ri] = w;
        }
    }
    for cwl in &mut column_width_lists {
        cwl.sort_unstable();
    }

    let column_count: usize = column_width_lists.len();
    let median_index = (lines.len() + 1) / 2;

    let mut column_widths: Vec::<MinMedMax> = vec![MinMedMax(0, 0, 0); column_count];
    for ci in 0..column_count {
        let cwlc: &[usize] = &column_width_lists[ci];
        column_widths[ci] = MinMedMax(cwlc[0], cwlc[median_index], cwlc[cwlc.len() - 1]);
    }

    column_widths
}

fn det_print_width_of_columns(column_width_minmedmaxs: &[MinMedMax], terminal_width: usize) -> Option<Vec<usize>> {
    let mid_max = |mmm: &MinMedMax| (mmm.1 + mmm.2) / 2;

    let column_count: usize = column_width_minmedmaxs.len();

    let mut need_to_alloc: usize = 0;
    let mut extra_allocable: usize = 0;
    for mmm in column_width_minmedmaxs {
        if mid_max(mmm) > MAX_UNFOLDED_COLUMN_WIDTH {
            need_to_alloc += mid_max(mmm) - MAX_UNFOLDED_COLUMN_WIDTH;
        } else if mmm.2 < MAX_UNFOLDED_COLUMN_WIDTH {
            extra_allocable += MAX_UNFOLDED_COLUMN_WIDTH - mmm.2;
        }
    }
    if need_to_alloc == 0 {
        need_to_alloc = 1;
    }
    let allocable: isize = (terminal_width + extra_allocable) as isize - (column_count * MAX_UNFOLDED_COLUMN_WIDTH + (column_count - 1) * *FRAME_CHAR_WIDTH) as isize;
    if allocable < 0 {
        return None;
    }

    let allocable = allocable as usize;
    let mut column_allocations: Vec<usize> = vec![MAX_UNFOLDED_COLUMN_WIDTH; column_count];
    for ci in 0..column_count {
        let mmm = column_width_minmedmaxs[ci];
        if mid_max(&mmm) > MAX_UNFOLDED_COLUMN_WIDTH {
            column_allocations[ci] += std::cmp::min(mmm.2 - MAX_UNFOLDED_COLUMN_WIDTH, (mid_max(&mmm) - MAX_UNFOLDED_COLUMN_WIDTH) * allocable / need_to_alloc);
        } else if mmm.2 < MAX_UNFOLDED_COLUMN_WIDTH {
            column_allocations[ci] -= MAX_UNFOLDED_COLUMN_WIDTH - mmm.2;
        }
    }

    Some(column_allocations)
}    

fn all_digits(subcells: &[&str]) -> bool {
    for item in subcells {
        for c in item.chars() {
            if ! ('0'..='9').contains(&c) {
                return false;
            }
        }
    }
    true
}

#[derive(Copy, Clone, Debug)]
enum TMB {
    Top,
    Middle,
    Bottom,
}

fn format_print_horizontal_line(tmb: TMB, column_widths: &[usize], linenum_width: usize) {
    assert!(*FRAME_CHAR_WIDTH == 1);

    let column_count = column_widths.len();
    let cross = match tmb { TMB::Top => FRAME_CROSS_TOP, TMB::Middle => FRAME_CROSS_MIDDLE, TMB::Bottom => FRAME_CROSS_BOTTOM };

    print!("{}", ANSI_ESCAPE_FRAME_COLOR);

    if linenum_width > 0 {
        print_str_bar(FRAME_HORIZONTAL, linenum_width);
        print!("{}", cross);
    }

    for ci in 0..column_count {
        print_str_bar(FRAME_HORIZONTAL, column_widths[ci]);
        if ci != column_count - 1 {
            print!("{}", cross);
        }
    }
    println!("{}", ANSI_ESCAPE_RESET_COLOR);
}

fn format_print_cell<S: AsRef<str>>(subcells: &[S], column_width: usize, subcells_all_digits: bool) {
    let w: usize = subcells.iter().map(|s| UnicodeWidthStr::width(s.as_ref())).sum();
    let s: String = subcells.iter().map(|s| s.as_ref()).collect();
    let ns: String = s.nfc().to_string();
    if subcells_all_digits {
        print_str_bar(" ", column_width - w);
        print!("{}", ns);
    } else {
        print!("{}", ns);
        print_str_bar(" ", column_width - w);
    }
}

fn format_print_line(line_number: usize, line: &str, cell_separator: char, column_widths: &[usize], linenum_width: usize) {
    let column_count = column_widths.len();

    let mut subcells: Vec<Vec<&str>> = vec![]; // split each cell into substrings
    for field in line.split(cell_separator) {
        let v: Vec<&str> = UnicodeSegmentation::graphemes(field, true).collect(); 
        subcells.push(v);
    }
    while subcells.len() < column_count {
        subcells.push(vec![]);
    }
    let subcells_all_digits: Vec<bool> = subcells.iter().map(|ss| all_digits(ss)).collect();

    let mut first_physical_line = true;
    let mut dones: Vec<usize> = vec![0; column_count]; // the indices of subcells already printed
    while (0..column_count).any(|ci| dones[ci] < subcells[ci].len()) {
        // print line number
        if linenum_width > 0 {
            print!("{}", ANSI_ESCAPE_TEXT_COLOR[line_number % 2]);
            if line_number != 0 && first_physical_line {
                let linenum_str = line_number.to_string();
                print_str_bar(" ", linenum_width - linenum_str.len());
                print!("{}", linenum_str);
            } else {
                print_str_bar(" ", linenum_width);
            }
            print!("{}{}", ANSI_ESCAPE_FRAME_COLOR, FRAME_VERTICAL);
        }

        // determine the subcells to be printed for each cell in the current line
        let mut todos: Vec<usize> = vec![0; column_count];
        for ci in 0..column_count {
            let csc = &subcells[ci];
            let cwc = column_widths[ci];
            todos[ci] = dones[ci];
            let mut w = 0;
            for ii in dones[ci]..subcells[ci].len() {
                let ssl = UnicodeWidthStr::width(csc[ii]);
                if w == 0 || w + ssl <= cwc {
                    todos[ci] = ii + 1;
                    w += ssl;
                } else {
                    break; // for ii
                }
            }
        }

        // print the subcells
        for ci in 0..column_count {
            let csc = &subcells[ci];
            let cwc = column_widths[ci];
            let sadc = subcells_all_digits[ci];
            let ac = if line_number == 0 { ANSI_ESCAPE_HEADER_COLOR } else { ANSI_ESCAPE_TEXT_COLOR };
            print!("{}", ac[line_number % 2]);
            format_print_cell(&csc[dones[ci]..todos[ci]], cwc, sadc);
            if ci == column_count - 1 {
                break; // for ci
            }
            print!("{}{}{}", ANSI_ESCAPE_RESET_COLOR, ANSI_ESCAPE_FRAME_COLOR, FRAME_VERTICAL);
        }
        println!("{}", ANSI_ESCAPE_RESET_COLOR);

        // update indices to mark the subcells "printed"
        dones = todos;

        first_physical_line = false;
    }
}

/// Table Pretty-print. print TSV or CSV file.
#[derive(StructOpt, Debug)]
#[structopt(name = "tapr")]
struct Opt {
    /// Treat input as CSV (even when including tab characters)
    #[structopt(short = "c", long)]
    csv: bool,

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
    let cell_separator = if opt.csv || ! includes_tab { ',' } else { '\t' };

    // calculate the width for line number (if needed)
    let linenum_width = if opt.line_number {
        (lines.len()).to_string().len()
    } else {
        0
    };

    // determine width of each column
    let column_width_minmedmaxs = get_column_widths(&lines, cell_separator);
    let cws = if linenum_width > 0 {
        det_print_width_of_columns(&column_width_minmedmaxs, terminal_width - (linenum_width + *FRAME_CHAR_WIDTH))
    } else {
        det_print_width_of_columns(&column_width_minmedmaxs, terminal_width)
    };
    let column_widths = cws.unwrap_or_else(|| {
        eprintln!("Error: terminal width too small for input table.");
        std::process::exit(1);
    });

    // print lines as a table
    format_print_horizontal_line(TMB::Top, &column_widths, linenum_width);
    if opt.header {
        for (li, line) in lines.iter().enumerate() {
            format_print_line(li, line, cell_separator, &column_widths, linenum_width);
            if li == 0 {
                format_print_horizontal_line(TMB::Middle, &column_widths, linenum_width);
            }
        }
    } else {
        for (li, line) in lines.iter().enumerate() {
            format_print_line(li + 1, line, cell_separator, &column_widths, linenum_width);
        }
    }
    format_print_horizontal_line(TMB::Bottom, &column_widths, linenum_width);
}
