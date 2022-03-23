use std::fs::File;
use std::io;
use std::io::BufRead;
use std::path::PathBuf;

use structopt::StructOpt;
use terminal_size::{Width, terminal_size};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

const MAX_UNFOLDED_COLUMN_WIDTH: usize = 7;
const ANSI_ESCAPE_TEXT_COLOR: &[&str] = &["\u{1b}[34m", "\u{1b}[32m"];
const ANSI_ESCAPE_FRAME_COLOR: &[&str] = &["\u{1b}[37m", "\u{1b}[37m"];
const ANSI_ESCAPE_RESET_COLOR: &str = "\u{1b}[0m";

fn str_width(s: &str) -> usize {
    let mut w: usize = 0;
    for ss in UnicodeSegmentation::graphemes(s, true) {
        w += UnicodeWidthStr::width(ss);
    }
    w
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
        if mid_max(&mmm) > MAX_UNFOLDED_COLUMN_WIDTH {
            need_to_alloc += mid_max(mmm) - MAX_UNFOLDED_COLUMN_WIDTH;
        } else if mmm.2 < MAX_UNFOLDED_COLUMN_WIDTH {
            extra_allocable += MAX_UNFOLDED_COLUMN_WIDTH - mmm.2;
        }
    }
    if need_to_alloc <= 0 {
        need_to_alloc = 1;
    }
    let allocable: isize = (terminal_width + extra_allocable) as isize - (column_count * (MAX_UNFOLDED_COLUMN_WIDTH + 1)) as isize;
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

fn all_digits(items: &[&str]) -> bool {
    for item in items {
        for c in item.chars() {
            if ! ('0' <= c && c <= '9') {
                return false;
            }
        }
    }
    true
}

fn print_spaces(width: usize) {
    if width > 0 {
        print!("{:<width$}", " ", width = width);
    }
}

fn format_print_cell<S: AsRef<str>>(cell_split: &[S], column_width: usize, items_all_digits: bool) {
    if items_all_digits {
        print_spaces(column_width - cell_split.len());
        for ss in cell_split {
            let ss = ss.as_ref();
            print!("{}", ss);
        }
    } else {
        let mut w = 0;
        for ss in cell_split {
            let ss = ss.as_ref();
            print!("{}", ss);
            let ssl = UnicodeWidthStr::width(ss);
            w += ssl;
        }
        print_spaces(column_width - w);
    }
}

fn format_print_line(line_number: usize, line: &str, cell_separator: char, column_widths: &[usize], linenum_width: usize) {
    let column_count = column_widths.len();

    let mut cell_splits: Vec<Vec<&str>> = vec![];
    for field in line.split(cell_separator) {
        let v: Vec<&str> = UnicodeSegmentation::graphemes(field, true).collect(); 
        cell_splits.push(v);
    }
    while cell_splits.len() < column_count {
        cell_splits.push(vec![]);
    }
    let items_all_digits: Vec<bool> = cell_splits.iter().map(|items| all_digits(items)).collect();

    let mut linenum_printed = false;
    let mut dones: Vec<usize> = vec![0; column_count];
    while (0..column_count).any(|ci| dones[ci] < cell_splits[ci].len()) {
        if linenum_width > 0 {
            print!("{}", ANSI_ESCAPE_TEXT_COLOR[line_number % 2]);
            if linenum_printed {
                print_spaces(linenum_width);
            } else {
                let linenum_str = line_number.to_string();
                print_spaces(linenum_width - linenum_str.len());
                print!("{}", linenum_str);
            }
            print!("{}\u{23d0}", ANSI_ESCAPE_FRAME_COLOR[line_number % 2]);
            linenum_printed = true;
        }
    
        let mut todos: Vec<usize> = vec![0; column_count];
        for ci in 0..column_count {
            let csc = &cell_splits[ci];
            let cwc = column_widths[ci];
            todos[ci] = dones[ci];
            let mut w = 0;
            for ii in dones[ci]..cell_splits[ci].len() {
                let ssl = UnicodeWidthStr::width(csc[ii]);
                if w == 0 || w + ssl <= cwc {
                    todos[ci] = ii + 1;
                    w += ssl;
                } else {
                    break; // for ii
                }
            }
        }

        for ci in 0..column_count {
            let csc = &cell_splits[ci];
            let cwc = column_widths[ci];
            let iadc = items_all_digits[ci];
            print!("{}", ANSI_ESCAPE_TEXT_COLOR[line_number % 2]);
            format_print_cell(&csc[dones[ci]..todos[ci]], cwc, iadc);
            if ci == column_count - 1 {
                break; // for ci
            }
            print!("{}\u{23d0}", ANSI_ESCAPE_FRAME_COLOR[line_number % 2]);
        }
        println!("{}", ANSI_ESCAPE_RESET_COLOR);

        dones = todos;
    }
}

/// Table Pretty-print. print TSV or CSV file.
#[derive(StructOpt, Debug)]
#[structopt(name = "tapr")]
struct Opt {
    /// Treat input as CSV (even when including tab characters)
    #[structopt(short, long)]
    csv: bool,

    /// Print line number
    #[structopt(short, long)]
    linenum: bool,

    /// Input file
    #[structopt(parse(from_os_str))]
    input: Option<PathBuf>,
}

fn main() {
    let opt = Opt::from_args();
    // println!("{:#?}", opt);

    let size = terminal_size();
    let (Width(width), _) = size.unwrap();
    let terminal_width: usize = width as usize;

    let lines: Vec<String> = if let Some(f) = opt.input {
        let fp = File::open(f).unwrap();
        io::BufReader::new(fp).lines().map(|line| line.unwrap()).collect()
    } else {
        let stdin = io::stdin();
        stdin.lock().lines().map(|line| line.unwrap()).collect()
    };
    let includes_tab = lines.iter().any(|line| line.contains('\t'));
    let cell_separator = if opt.csv || ! includes_tab { ',' } else { '\t' };
    let linenum_width = if opt.linenum {
        (lines.len()).to_string().len()
    } else {
        0
    };

    let column_width_minmedmaxs = get_column_widths(&lines, cell_separator);

    let cws = if linenum_width > 0 {
        det_print_width_of_columns(&column_width_minmedmaxs, terminal_width - (linenum_width + 1))
    } else {
        det_print_width_of_columns(&column_width_minmedmaxs, terminal_width)
    };
    if let Some(column_widths) = cws {
        for (li, line) in lines.iter().enumerate() {
            format_print_line(li + 1, line, cell_separator, &column_widths, linenum_width);
        }
    } else {
        eprintln!("Error: terminal width too small for input table.");
        std::process::exit(1);
    }
}
