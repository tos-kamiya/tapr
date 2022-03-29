use thiserror::Error;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use super::constants::*;
use super::utils::*;

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

fn mid_max(mmm: &MinMedMax, weight_of_max: f32) -> isize {
    if weight_of_max == 1.0 {
        mmm.2 as isize
    } else {
        (mmm.1 as f32 * (1.0 - weight_of_max) + mmm.2 as f32 * weight_of_max) as isize
    }
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
    let column_count: usize = column_width_minmedmaxs.len();
    if column_count == 0 {
        return Ok(vec![]);
    }

    // determine width of each column
    let mut v = None;
    for p in (50..=105).rev().step_by(5) {
        let weight_of_max: f32 = p as f32 / 100.0;
        let mut need_to_alloc: usize = 0;
        let mut extra_allocable: usize = 0;

        // determine width requested by each column
        for mmm in column_width_minmedmaxs {
            let t = mid_max(mmm, weight_of_max) as usize;
            if t > MAX_UNFOLDED_COLUMN_WIDTH {
                need_to_alloc += t - MAX_UNFOLDED_COLUMN_WIDTH;
            } else if mmm.2 < MAX_UNFOLDED_COLUMN_WIDTH {
                extra_allocable += MAX_UNFOLDED_COLUMN_WIDTH - mmm.2;
            }
        }
        if need_to_alloc == 0 {
            need_to_alloc = 1;
        }

        // check terminal has enough width to the requests
        let allocable: isize = (terminal_width + extra_allocable) as isize
            - (column_count * MAX_UNFOLDED_COLUMN_WIDTH + (column_count - 1) * *frame::CHAR_WIDTH)
                as isize;
        if allocable >= 0 {
            v = Some((weight_of_max, need_to_alloc, allocable as usize));
            break;  // for p
        }
    }
    if v == None {
        return Err(DetColumnWidthError::TooManyColumns {
            columns: column_count,
        });
    }

    // allocate width to each column
    let (weight_of_max, need_to_alloc, allocable) = v.unwrap();
    let mut column_allocations: Vec<usize> = vec![MAX_UNFOLDED_COLUMN_WIDTH; column_count];
    for ci in 0..column_count {
        let mmm = column_width_minmedmaxs[ci];
        let t = mid_max(&mmm, weight_of_max) as usize;
        if t > MAX_UNFOLDED_COLUMN_WIDTH {
            column_allocations[ci] += std::cmp::min(
                mmm.2 - MAX_UNFOLDED_COLUMN_WIDTH,
                (t - MAX_UNFOLDED_COLUMN_WIDTH) * allocable / need_to_alloc,
            );
        } else if mmm.2 < MAX_UNFOLDED_COLUMN_WIDTH {
            column_allocations[ci] -= MAX_UNFOLDED_COLUMN_WIDTH - mmm.2;
        }
    }

    Ok(column_allocations)
}
