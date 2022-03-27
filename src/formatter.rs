use thiserror::Error;
use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use super::constants::*;
use super::utils::*;

fn all_digits<S: AsRef<str>>(text: S) -> bool {
    text.as_ref().chars().find(|c| !('0'..='9').contains(&c)) == None
}

fn print_str_bar<S: AsRef<str>>(s: S, count: usize) {
    if count > 0 {
        let s = s.as_ref();
        print!("{}", (0..count).map(|_| s).collect::<String>());
    }
}

pub fn print_horizontal_line(crossing: &str, column_widths: &[usize], linenum_width: usize) {
    use super::constants::ansi_escape::*;

    assert!(*frame::CHAR_WIDTH == 1);

    let column_count = column_widths.len();

    print!("{}", FRAME_COLOR);

    if linenum_width > 0 {
        print_str_bar(frame::HORIZONTAL, linenum_width);
        print!("{}", crossing);
    }

    for (ci, cwc) in column_widths.iter().enumerate() {
        print_str_bar(frame::HORIZONTAL, *cwc);
        if ci != column_count - 1 {
            print!("{}", crossing);
        }
    }
    println!("{}", RESET_COLOR);
}

fn print_cell<S: AsRef<str>>(subcells: &[S], column_width: usize, right_align: bool) {
    let w: usize = subcells
        .iter()
        .map(|s| UnicodeWidthStr::width(s.as_ref()))
        .sum();
    let s: String = subcells.iter().map(|s| s.as_ref()).collect();
    if right_align {
        print_str_bar(" ", column_width - w);
        print!("{}", s.nfc());
    } else {
        print!("{}", s.nfc());
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
    use super::constants::ansi_escape::*;

    assert!(!cells.is_empty());
    let column_count = column_widths.len();

    // split each cells into unicode chars
    let mut subcells = to_subcells(cells);
    subcells.resize(column_count, vec![]);

    let cell_right_aligns: Vec<bool> = cells.iter().map(|s| all_digits(s.as_ref())).collect();

    // print cells
    let mut first_physical_line = true;
    let mut dones: Vec<usize> = vec![0; column_count]; // the indices of subcells already printed
    while (0..column_count).any(|ci| dones[ci] < subcells[ci].len()) {
        // print line number
        if linenum_width > 0 {
            let c = TEXT_COLORS[(line_number == 0).q(0, 1 + line_number % 2)];
            print!("{}", c);
            if line_number != 0 && first_physical_line {
                let linenum_str = line_number.to_string();
                print_str_bar(" ", linenum_width - linenum_str.len());
                print!("{}", linenum_str);
            } else {
                print_str_bar(" ", linenum_width);
            }
            print!("{}{}", FRAME_COLOR, frame::VERTICAL);
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
            let crac = ci < cell_right_aligns.len() && cell_right_aligns[ci];
            let c = TEXT_COLORS[(line_number == 0).q(0, 1 + line_number % 2)];
            print!("{}", c);
            print_cell(&csc[dones[ci]..todos[ci]], cwc, crac);
            if ci == column_count - 1 {
                break; // for ci
            }
            print!("{}{}{}", RESET_COLOR, FRAME_COLOR, frame::VERTICAL);
        }
        println!("{}", RESET_COLOR);

        // update indices to mark the subcells "printed"
        dones = todos;

        first_physical_line = false;
    }
}

fn str_width(s: &str) -> usize {
    UnicodeSegmentation::graphemes(s, true)
        .map(UnicodeWidthStr::width)
        .sum()
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
            column_width_lists[ci][ri] = str_width(cell.as_ref());
        }
    }
    for cwl in &mut column_width_lists {
        cwl.sort_unstable();
    }

    let column_count: usize = column_width_lists.len();
    let median_index = (line_count == 1).q(0, (line_count + 1) / 2);

    let mut column_widths: Vec<MinMedMax> = vec![MinMedMax(0, 0, 0); column_count];
    for ci in 0..column_count {
        let cwlc: &[usize] = &column_width_lists[ci];
        column_widths[ci] = MinMedMax(cwlc[0], cwlc[median_index], cwlc[cwlc.len() - 1]);
    }

    column_widths
}

#[derive(Error, Debug)]
pub enum DetColumnWidthError {
    #[error("too many columns: {}", .columns)]
    TooManyColumns { columns: usize },
}

pub fn det_print_column_widths(
    column_width_minmedmaxs: &[MinMedMax],
    terminal_width: usize,
) -> Result<Vec<usize>, DetColumnWidthError> {
    let mid_max = |mmm: &MinMedMax| (mmm.1 + mmm.2) / 2;

    let column_count: usize = column_width_minmedmaxs.len();
    if column_count == 0 {
        return Ok(vec![]);
    }

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
        - (column_count * MAX_UNFOLDED_COLUMN_WIDTH + (column_count - 1) * *frame::CHAR_WIDTH)
            as isize;
    if allocable < 0 {
        return Err(DetColumnWidthError::TooManyColumns {
            columns: column_count,
        });
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

    Ok(column_allocations)
}
