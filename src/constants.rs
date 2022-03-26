use lazy_static;

use unicode_width::UnicodeWidthStr;

pub const MAX_UNFOLDED_COLUMN_WIDTH: usize = 7;
pub const ANSI_ESCAPE_HEADER_COLOR: &[&str] = &["\u{1b}[37m", "\u{1b}[37m"];
pub const ANSI_ESCAPE_TEXT_COLOR: &[&str] = &["\u{1b}[34m", "\u{1b}[32m"];
pub const ANSI_ESCAPE_FRAME_COLOR: &str = "\u{1b}[90m";
pub const ANSI_ESCAPE_RESET_COLOR: &str = "\u{1b}[0m";

pub const FRAME_VERTICAL: &str = "\u{2502}";
pub const FRAME_HORIZONTAL: &str = "\u{2500}";
pub const FRAME_CROSSING_TOP: &str = "\u{252c}";
pub const FRAME_CROSSING_BOTTOM: &str = "\u{2534}";
pub const FRAME_CROSSING_MIDDLE: &str = "\u{253c}";

// pub const FRAME_VERTICAL: &str = "\u{2503}";
// pub const FRAME_HORIZONTAL: &str = "\u{2501}";
// pub const FRAME_CROSSING_TOP: &str = "\u{2533}";
// pub const FRAME_CROSSING_BOTTOM: &str = "\u{253b}";
// pub const FRAME_CROSSING_MIDDLE: &str = "\u{254b}";

lazy_static! {
    pub static ref FRAME_CHAR_WIDTH: usize = UnicodeWidthStr::width(FRAME_VERTICAL);
}

