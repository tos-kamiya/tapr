use std::io;

use anyhow;
use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use super::constants::*;
use super::utils::*;

fn all_digits<S: AsRef<str>>(text: S) -> bool {
    text.as_ref().chars().find(|c| !('0'..='9').contains(c)) == None
}

fn format_str_bar<S: AsRef<str>>(s: S, count: usize) -> String {
    if count > 0 {
        (0..count).map(|_| s.as_ref()).collect()
    } else {
        "".to_string()
    }
}

pub fn print_horizontal_line(out: &mut dyn io::Write, crossing: &str, column_widths: &[usize], linenum_width: usize) -> anyhow::Result<()> {
    use super::constants::ansi_escape::*;

    assert!(*frame::CHAR_WIDTH == 1);

    let column_count = column_widths.len();

    write!(out, "{}", FRAME_COLOR)?;
    if linenum_width > 0 {
        write!(out, "{}{}", format_str_bar(frame::HORIZONTAL, linenum_width), crossing)?;
    }
    for (ci, cwc) in column_widths.iter().enumerate() {
        write!(out, "{}", format_str_bar(frame::HORIZONTAL, *cwc))?;
        if ci != column_count - 1 {
            write!(out, "{}", crossing)?;
        }
    }
    writeln!(out, "{}", RESET_COLOR)?;

    Ok(())
}

fn print_cell<S: AsRef<str>>(out: &mut dyn io::Write, subcells: &[S], column_width: usize, right_align: bool) -> anyhow::Result<()> {
    let w: usize = subcells
        .iter()
        .map(|s| UnicodeWidthStr::width(s.as_ref()))
        .sum();
    let s: String = subcells.iter().map(|s| s.as_ref()).collect();
    let padding = format_str_bar(" ", column_width - w);
    if right_align {
        write!(out, "{}{}", padding, s.nfc())?;
    } else {
        write!(out, "{}{}", s.nfc(), padding)?;
    }

    Ok(())
}

pub fn print_line<S: AsRef<str>>(
    out: &mut dyn io::Write, 
    line_number: usize,
    cells: &[S],
    column_widths: &[usize],
    linenum_width: usize,
) -> anyhow::Result<()> {
    use super::constants::ansi_escape::*;

    let column_count = column_widths.len();

    // split each cells into unicode chars
    let mut subcells: Vec<Vec<&str>> = cells.iter().map(|cell| UnicodeSegmentation::graphemes(cell.as_ref(), true).collect()).collect();
    if subcells.is_empty() {
        subcells.push(vec![" "]);
    }
    subcells.resize(column_count, vec![]);

    let cell_right_aligns: Vec<bool> = cells.iter().map(|s| all_digits(s.as_ref())).collect();

    // print cells
    let mut first_physical_line = true;
    let mut dones: Vec<usize> = vec![0; column_count]; // the indices of subcells already printed
    while (0..column_count).any(|ci| dones[ci] < subcells[ci].len()) {
        // print line number
        if linenum_width > 0 {
            let c = TEXT_COLORS[(line_number == 0).q(0, 1 + line_number % 2)];
            write!(out, "{}", c)?;
            if line_number != 0 && first_physical_line {
                let linenum_str = line_number.to_string();
                write!(out, "{}{}", format_str_bar(" ", linenum_width - linenum_str.len()), linenum_str)?;
            } else {
                write!(out, "{}", format_str_bar(" ", linenum_width))?;
            }
            write!(out, "{}{}", FRAME_COLOR, frame::VERTICAL)?;
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
            write!(out, "{}", c)?;
            print_cell(out, &csc[dones[ci]..todos[ci]], cwc, crac)?;
            if ci == column_count - 1 {
                break; // for ci
            }
            write!(out, "{}{}", FRAME_COLOR, frame::VERTICAL)?;
        }
        writeln!(out, "{}", RESET_COLOR)?;

        // update indices to mark the subcells "printed"
        dones = todos;

        first_physical_line = false;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::constants::ansi_escape::*;
    use super::super::constants::frame::*;
    use super::*;

    #[test]
    fn test_print_line_for_empty() -> anyhow::Result<()> {
        let mut out = io::Cursor::new(vec![]);
        let cells: Vec<&str> = vec![];
        print_line(&mut out, 0, &cells, &vec![], 0)?;
        let value = String::from_utf8(out.into_inner()).unwrap();
        assert!(value.is_empty());

        Ok(())
    }

    #[test]
    fn test_print_line_for_empty_cell() -> anyhow::Result<()> {
        let mut out = io::Cursor::new(vec![]);
        let cells: Vec<&str> = vec![];
        print_line(&mut out, 0, &cells, &vec![1], 0)?;
        let value = String::from_utf8(out.into_inner()).unwrap();
        assert_eq!(value, format!("{} {}\n", TEXT_COLORS[0], RESET_COLOR));

        Ok(())
    }

    #[test]
    fn test_print_line_for_missing_columns() -> anyhow::Result<()> {
        let mut out = io::Cursor::new(vec![]);
        let cells: Vec<&str> = vec!["1"];
        print_line(&mut out, 0, &cells, &vec![1, 2, 3], 0)?;
        let value = String::from_utf8(out.into_inner()).unwrap();
        assert_eq!(value, format!("{}1{}{}{}  {}{}{}   {}\n", 
            TEXT_COLORS[0], 
            FRAME_COLOR, VERTICAL, 
            TEXT_COLORS[0], 
            FRAME_COLOR, VERTICAL, 
            TEXT_COLORS[0], 
            RESET_COLOR));

        Ok(())
    }
}
