use std::fs::File;
use std::io;
use std::io::BufRead;
use std::path::PathBuf;

use structopt::StructOpt;
use terminal_size::{Width, terminal_size};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

const MIN_COLUMN_WIDTH: usize = 7;

fn str_width(s: &str) -> usize {
    let mut w: usize = 0;
    for ss in UnicodeSegmentation::graphemes(s, true) {
        w += UnicodeWidthStr::width(ss);
    }
    w
}

fn get_column_widths<S: AsRef<str>>(lines: &[S], cell_separator: char) -> Vec::<usize> {
    let lines_len = lines.len();

    let mut column_width_lists = Vec::<Vec<usize>>::new();
    for (ri, line) in lines.iter().enumerate() {
        let line = line.as_ref();
        for (ci, field) in line.split(cell_separator).enumerate() {
            if ci >= column_width_lists.len() {
                column_width_lists.push(vec![0; lines_len]);
            }
            column_width_lists[ci][ri] = str_width(field);
        }
    }
    for cwl in &mut column_width_lists {
        cwl.sort_unstable();
    }

    let column_count: usize = column_width_lists.len();
    let median_index = (lines.len() + 1) / 2;

    let mut column_widths: Vec::<usize> = vec![0; column_count];
    for ci in 0..column_count {
        column_widths[ci] = column_width_lists[ci][median_index];
    }

    column_widths
}

fn det_print_width_of_columns(column_widths: &[usize], terminal_width: usize) -> Vec<usize> {
    let column_count: usize = column_widths.len();

    let mut need_to_alloc = 0;
    for cwc in column_widths {
        if *cwc > MIN_COLUMN_WIDTH {
            need_to_alloc += *cwc - MIN_COLUMN_WIDTH;
        }
    }
    let allocable = terminal_width - column_count * (MIN_COLUMN_WIDTH + 1);
    let mut column_allocations: Vec<usize> = vec![MIN_COLUMN_WIDTH;column_count];
    for ci in 0..column_count {
        let cwc = column_widths[ci];
        if cwc > MIN_COLUMN_WIDTH {
            column_allocations[ci] += (cwc - MIN_COLUMN_WIDTH) * allocable / need_to_alloc;
        }
    }

    column_allocations
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

fn print_cell_in_column<S: AsRef<str>>(cell_split: &[S], column_width: usize, items_all_digits: bool) {
    if items_all_digits {
        for _i in 0..(column_width - cell_split.len()) {
            print!(" ");
        }
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
        for _i in w..column_width {
            print!(" ");
        }
    }
}

/// Table Pretty-print. print TSV or CSV file.
#[derive(StructOpt, Debug)]
#[structopt(name = "tapr")]
struct Opt {
    /// Treat input as CSV (even when including tab characters)
    #[structopt(short, long)]
    csv: bool,

    /// Input file
    #[structopt(parse(from_os_str))]
    input: Option<PathBuf>,
}

fn main() {
    let opt = Opt::from_args();
    // println!("{:#?}", opt);

    let size = terminal_size();
    let (Width(width), _) = size.unwrap();
    let terminal_width = width as usize;

    let lines: Vec<String> = if let Some(f) = opt.input {
        let fp = File::open(f).unwrap();
        io::BufReader::new(fp).lines().map(|line| line.unwrap()).collect()
    } else {
        let stdin = io::stdin();
        stdin.lock().lines().map(|line| line.unwrap()).collect()
    };
    let includes_tab = lines.iter().any(|line| line.contains('\t'));
    let cell_separator = if opt.csv || ! includes_tab { ',' } else { '\t' };

    let column_widths = get_column_widths(&lines, cell_separator);

    let column_count: usize = column_widths.len();

    if column_count * (MIN_COLUMN_WIDTH + 1) > terminal_width {
        eprintln!("Error: terminal width too small for input table.");
        std::process::exit(1);
    }

    let column_widths = det_print_width_of_columns(&column_widths, terminal_width);
    for (li, line) in lines.iter().enumerate() {
        let mut cell_splits: Vec<Vec<&str>> = vec![];
        for field in line.split(cell_separator) {
            let mut v = Vec::<&str>::new();
            for ss in UnicodeSegmentation::graphemes(field, true) {
                v.push(ss);
            }
            cell_splits.push(v);
        }
        assert!(cell_splits.len() == column_count);
        let items_all_digits: Vec<bool> = cell_splits.iter().map(|items| all_digits(items)).collect();

        let mut dones: Vec<usize> = vec![0; column_count];
        while (0..column_count).any(|ci| dones[ci] < cell_splits[ci].len()) {
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
                print!("{}", if li % 2 == 0 { "\u{1b}[32m" } else { "\u{1b}[33m" });
                print_cell_in_column(&csc[dones[ci]..todos[ci]], cwc, iadc);
                if ci < column_count - 1 {
                    print!("\u{1b}[90m\u{2595}\u{1b}[22m");
                }
            }
            println!("\u{1b}[0m");

            dones = todos;
        }
    }
}
