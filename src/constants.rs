pub const MAX_UNFOLDED_COLUMN_WIDTH: usize = 7;

pub mod ansi_escape {
    pub const TEXT_COLORS: &[&str] = &[
        "\u{1b}[37m", // header line
        "\u{1b}[34m", // even line
        "\u{1b}[32m", // odd line
    ];
    pub const FRAME_COLOR: &str = "\u{1b}[90m";
    pub const RESET_COLOR: &str = "\u{1b}[0m";
}

pub mod frame {
    use lazy_static;

    use unicode_width::UnicodeWidthStr;

    pub const VERTICAL: &str = "\u{2502}";
    pub const HORIZONTAL: &str = "\u{2500}";
    pub const CROSSING_TOP: &str = "\u{252c}";
    pub const CROSSING_BOTTOM: &str = "\u{2534}";
    pub const CROSSING_MIDDLE: &str = "\u{253c}";

    // pub const VERTICAL: &str = "\u{2503}";
    // pub const HORIZONTAL: &str = "\u{2501}";
    // pub const CROSSING_TOP: &str = "\u{2533}";
    // pub const CROSSING_BOTTOM: &str = "\u{253b}";
    // pub const CROSSING_MIDDLE: &str = "\u{254b}";
    lazy_static! {
        pub static ref CHAR_WIDTH: usize = UnicodeWidthStr::width(VERTICAL);
    }
}
