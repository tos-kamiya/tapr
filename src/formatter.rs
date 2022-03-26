use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use super::constants::*;

fn all_digits(subcells: &[&str]) -> bool {
    for item in subcells {
        for c in item.chars() {
            if !('0'..='9').contains(&c) {
                return false;
            }
        }
    }
    true
}

fn print_str_bar(s: &str, repeat: usize) {
    if repeat > 0 {
        print!("{}", (0..repeat).map(|_| s).collect::<String>());
    }
}

#[derive(Copy, Clone, Debug)]
pub enum TMB {
    Top,
    Middle,
    Bottom,
}

pub fn print_horizontal_line(tmb: TMB, column_widths: &[usize], linenum_width: usize) {
    assert!(*FRAME_CHAR_WIDTH == 1);

    let column_count = column_widths.len();
    let cross = match tmb {
        TMB::Top => FRAME_CROSSING_TOP,
        TMB::Middle => FRAME_CROSSING_MIDDLE,
        TMB::Bottom => FRAME_CROSSING_BOTTOM,
    };

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

fn print_cell<S: AsRef<str>>(subcells: &[S], column_width: usize, subcell_right_aligns: bool) {
    let w: usize = subcells
        .iter()
        .map(|s| UnicodeWidthStr::width(s.as_ref()))
        .sum();
    let s: String = subcells.iter().map(|s| s.as_ref()).collect();
    let ns: String = s.nfc().to_string();
    if subcell_right_aligns {
        print_str_bar(" ", column_width - w);
        print!("{}", ns);
    } else {
        print!("{}", ns);
        print_str_bar(" ", column_width - w);
    }
}

fn to_subcells<S: AsRef<str>>(cells: &[S]) -> Vec<Vec<&str>> {
    let mut subcells: Vec<Vec<&str>> = vec![]; // split each cell into substrings
    for cell in cells {
        let v: Vec<&str> = UnicodeSegmentation::graphemes(cell.as_ref(), true).collect();
        subcells.push(v);
    }
    subcells
}

pub fn print_line<S: AsRef<str>>(
    line_number: usize,
    cells: &[S],
    column_widths: &[usize],
    linenum_width: usize,
) {
    if cells.len() == 0 {
        println!();
        return;
    }

    let column_count = column_widths.len();

    // split each cells into unicode chars
    let mut subcells = to_subcells(cells);
    while subcells.len() < column_count {
        subcells.push(vec![]);
    }
    let subcell_right_aligns: Vec<bool> = subcells.iter().map(|ss| all_digits(ss)).collect();

    // print cells
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
            let srac = subcell_right_aligns[ci];
            let ac = if line_number == 0 {
                ANSI_ESCAPE_HEADER_COLOR
            } else {
                ANSI_ESCAPE_TEXT_COLOR
            };
            print!("{}", ac[line_number % 2]);
            print_cell(&csc[dones[ci]..todos[ci]], cwc, srac);
            if ci == column_count - 1 {
                break; // for ci
            }
            print!(
                "{}{}{}",
                ANSI_ESCAPE_RESET_COLOR, ANSI_ESCAPE_FRAME_COLOR, FRAME_VERTICAL
            );
        }
        println!("{}", ANSI_ESCAPE_RESET_COLOR);

        // update indices to mark the subcells "printed"
        dones = todos;

        first_physical_line = false;
    }
}

fn str_width(s: &str) -> usize {
    let mut w: usize = 0;
    for ss in UnicodeSegmentation::graphemes(s, true) {
        w += UnicodeWidthStr::width(ss);
    }
    w
}

#[derive(Copy, Clone, Debug)]
pub struct MinMedMax(usize, usize, usize);

pub fn get_raw_column_widths<S: AsRef<str>>(line_cells: &[&[S]]) -> Vec<MinMedMax> {
    let line_count = line_cells.len();

    let mut column_width_lists: Vec<Vec<usize>> = vec![];
    for (ri, cells) in line_cells.iter().enumerate() {
        for (ci, cell) in cells.iter().enumerate() {
            if ci >= column_width_lists.len() {
                column_width_lists.push(vec![0; line_count]);
            }
            let w = str_width(cell.as_ref());
            column_width_lists[ci][ri] = w;
        }
    }
    for cwl in &mut column_width_lists {
        cwl.sort_unstable();
    }

    let column_count: usize = column_width_lists.len();
    let median_index = (line_count + 1) / 2;

    let mut column_widths: Vec<MinMedMax> = vec![MinMedMax(0, 0, 0); column_count];
    for ci in 0..column_count {
        let cwlc: &[usize] = &column_width_lists[ci];
        column_widths[ci] = MinMedMax(cwlc[0], cwlc[median_index], cwlc[cwlc.len() - 1]);
    }

    column_widths
}

pub fn det_print_column_widths(
    column_width_minmedmaxs: &[MinMedMax],
    terminal_width: usize,
) -> Option<Vec<usize>> {
    let mid_max = |mmm: &MinMedMax| (mmm.1 + mmm.2) / 2;

    let column_count: usize = column_width_minmedmaxs.len();

    // check if each column need to more than min width (MAX_UNFOLDED_COLUMN_WIDTH)
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

    // calculate how many chars are allocable to columns need more width
    let allocable: isize = (terminal_width + extra_allocable) as isize
        - (column_count * MAX_UNFOLDED_COLUMN_WIDTH + (column_count - 1) * *FRAME_CHAR_WIDTH)
            as isize;
    if allocable < 0 {
        return None;
    }

    // allocate widths to each column
    let allocable = allocable as usize;
    let mut column_allocations: Vec<usize> = vec![MAX_UNFOLDED_COLUMN_WIDTH; column_count];
    for ci in 0..column_count {
        let mmm = column_width_minmedmaxs[ci];
        if mid_max(&mmm) > MAX_UNFOLDED_COLUMN_WIDTH {
            column_allocations[ci] += std::cmp::min(
                mmm.2 - MAX_UNFOLDED_COLUMN_WIDTH,
                (mid_max(&mmm) - MAX_UNFOLDED_COLUMN_WIDTH) * allocable / need_to_alloc,
            );
        } else if mmm.2 < MAX_UNFOLDED_COLUMN_WIDTH {
            column_allocations[ci] -= MAX_UNFOLDED_COLUMN_WIDTH - mmm.2;
        }
    }

    Some(column_allocations)
}
