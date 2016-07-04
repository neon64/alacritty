// Copyright 2016 Joe Wilm, The Alacritty Project Contributors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
//! Exports the `Term` type which is a high-level API for the Grid
use std::ops::Range;
use std::fmt;

use ansi::{self, Attr};
use grid::{Grid, ClearRegion};
use tty;
use ::Rgb;

/// coerce val to be between min and max
fn limit<T: PartialOrd>(val: T, min: T, max: T) -> T {
    if val < min {
        min
    } else if val > max {
        max
    } else {
        val
    }
}

pub mod cell {
    use super::{DEFAULT_FG, DEFAULT_BG};
    use ::Rgb;

    bitflags! {
        pub flags Flags: u32 {
            const INVERSE   = 0b00000001,
            const BOLD      = 0b00000010,
            const ITALIC    = 0b00000100,
            const UNDERLINE = 0b00001000,
        }
    }

    #[derive(Clone, Debug)]
    pub struct Cell {
        pub c: char,
        pub fg: Rgb,
        pub bg: Rgb,
        pub flags: Flags,
    }

    impl Cell {
        pub fn new(c: char) -> Cell {
            Cell {
                c: c.into(),
                bg: Default::default(),
                fg: Default::default(),
                flags: Flags::empty(),
            }
        }

        pub fn reset(&mut self) {
            self.c = ' ';
            self.flags = Flags::empty();

            self.bg = DEFAULT_BG;
            self.fg = DEFAULT_FG;
        }
    }
}

pub use self::cell::Cell;

/// tomorrow night bright
///
/// because contrast
pub static COLORS: &'static [Rgb] = &[
    Rgb {r: 0x00, g: 0x00, b: 0x00}, // Black
    Rgb {r: 0xd5, g: 0x4e, b: 0x53}, // Red
    Rgb {r: 0xb9, g: 0xca, b: 0x4a}, // Green
    Rgb {r: 0xe6, g: 0xc5, b: 0x47}, // Yellow
    Rgb {r: 0x7a, g: 0xa6, b: 0xda}, // Blue
    Rgb {r: 0xc3, g: 0x97, b: 0xd8}, // Magenta
    Rgb {r: 0x70, g: 0xc0, b: 0xba}, // Cyan
    Rgb {r: 0x42, g: 0x42, b: 0x42}, // White
    Rgb {r: 0x66, g: 0x66, b: 0x66}, // Bright black
    Rgb {r: 0xff, g: 0x33, b: 0x34}, // Bright red
    Rgb {r: 0x9e, g: 0xc4, b: 0x00}, // Bright green
    Rgb {r: 0xe7, g: 0xc5, b: 0x47}, // Bright yellow
    Rgb {r: 0x7a, g: 0xa6, b: 0xda}, // Bright blue
    Rgb {r: 0xb7, g: 0x7e, b: 0xe0}, // Bright magenta
    Rgb {r: 0x54, g: 0xce, b: 0xd6}, // Bright cyan
    Rgb {r: 0x2a, g: 0x2a, b: 0x2a}, // Bright white
];

pub mod mode {
    bitflags! {
        pub flags TermMode: u8 {
            const SHOW_CURSOR = 0b00000001,
            const APP_CURSOR  = 0b00000010,
            const ANY         = 0b11111111,
            const NONE        = 0b00000000,
        }
    }

    impl Default for TermMode {
        fn default() -> TermMode {
            SHOW_CURSOR
        }
    }
}

pub use self::mode::TermMode;

pub const CURSOR_SHAPE: char = '█';

pub const DEFAULT_FG: Rgb = Rgb { r: 0xea, g: 0xea, b: 0xea};
pub const DEFAULT_BG: Rgb = Rgb { r: 0, g: 0, b: 0};
pub const TAB_SPACES: usize = 8;

use grid::index::{Cursor, Column, Line};

pub struct Term {
    /// The grid
    grid: Grid<Cell>,

    /// Alternate grid
    alt_grid: Grid<Cell>,

    /// Alt is active
    alt: bool,

    /// Reference to the underlying tty
    tty: tty::Tty,

    /// The cursor
    cursor: Cursor,

    /// Alt cursor
    alt_cursor: Cursor,

    /// Active foreground color
    fg: Rgb,

    /// Active background color
    bg: Rgb,

    /// Tabstops
    tabs: Vec<bool>,

    /// Cell attributes
    attr: cell::Flags,

    /// Mode flags
    mode: TermMode,

    /// Scroll region
    scroll_region: Range<Line>,

    /// Size
    size_info: SizeInfo,
}

/// Terminal size info
#[derive(Debug)]
pub struct SizeInfo {
    /// Terminal window width
    pub width: f32,

    /// Terminal window height
    pub height: f32,

    /// Width of individual cell
    pub cell_width: f32,

    /// Height of individual cell
    pub cell_height: f32,
}

impl SizeInfo {
    #[inline]
    pub fn lines(&self) -> Line {
        Line((self.height / self.cell_height) as usize)
    }

    #[inline]
    pub fn cols(&self) -> Column {
        Column((self.width / self.cell_width) as usize)
    }
}

impl Term {
    pub fn new(width: f32, height: f32, cell_width: f32, cell_height: f32) -> Term {
        let size = SizeInfo {
            width: width as f32,
            height: height as f32,
            cell_width: cell_width as f32,
            cell_height: cell_height as f32,
        };

        let num_cols = size.cols();
        let num_lines = size.lines();

        println!("num_cols, num_lines = {}, {}", num_cols, num_lines);

        let grid = Grid::new(num_lines, num_cols, &Cell::new(' '));

        let tty = tty::new(*num_lines as u8, *num_cols as u8);
        tty.resize(*num_lines as usize, *num_cols as usize, size.width as usize, size.height as usize);

        let mut tabs = (Column(0)..grid.num_cols()).map(|i| (*i as usize) % TAB_SPACES == 0)
                                                   .collect::<Vec<bool>>();
        tabs[0] = false;

        let alt = grid.clone();
        let scroll_region = Line(0)..grid.num_lines();

        Term {
            grid: grid,
            alt_grid: alt,
            alt: false,
            cursor: Cursor::default(),
            alt_cursor: Cursor::default(),
            fg: DEFAULT_FG,
            bg: DEFAULT_BG,
            tty: tty,
            tabs: tabs,
            attr: cell::Flags::empty(),
            mode: Default::default(),
            scroll_region: scroll_region,
            size_info: size
        }
    }

    /// Resize terminal to new dimensions
    pub fn resize(&mut self, width: f32, height: f32) {
        let size = SizeInfo {
            width: width,
            height: height,
            cell_width: self.size_info.cell_width,
            cell_height: self.size_info.cell_height,
        };

        let old_cols = self.size_info.cols();
        let old_lines = self.size_info.lines();
        let num_cols = size.cols();
        let num_lines = size.lines();

        self.size_info = size;

        if old_cols == num_cols && old_lines == num_lines {
            return;
        }

        // Scroll up to keep cursor and as much context as possible in grid. This only runs when the
        // lines decreases.
        self.scroll_region = Line(0)..self.grid.num_lines();

        // Scroll up to keep cursor in terminal
        if self.cursor.line >= num_lines {
            let lines = self.cursor.line - num_lines + 1;
            self.scroll(lines, ScrollDirection::Down);
            self.cursor.line -= lines;
        }

        println!("num_cols, num_lines = {}, {}", num_cols, num_lines);

        // Resize grids to new size
        self.grid.resize(num_lines, num_cols, &Cell::new(' '));
        self.alt_grid.resize(num_lines, num_cols, &Cell::new(' '));

        // Ensure cursor is in-bounds
        self.cursor.line = limit(self.cursor.line, Line(0), num_lines);
        self.cursor.col = limit(self.cursor.col, Column(0), num_cols);

        // Recreate tabs list
        self.tabs = (Column(0)..self.grid.num_cols()).map(|i| (*i as usize) % TAB_SPACES == 0)
                                                     .collect::<Vec<bool>>();

        // Make sure bottom of terminal is clear
        self.grid.clear_region((self.cursor.line).., |c| c.reset());
        self.alt_grid.clear_region((self.cursor.line).., |c| c.reset());

        // Reset scrolling region to new size
        self.scroll_region = Line(0)..self.grid.num_lines();

        // Inform tty of new dimensions
        self.tty.resize(*num_lines as _,
                        *num_cols as _,
                        self.size_info.width as usize,
                        self.size_info.height as usize);

    }

    #[inline]
    pub fn tty(&self) -> &tty::Tty {
        &self.tty
    }

    #[inline]
    pub fn size_info(&self) -> &SizeInfo {
        &self.size_info
    }

    #[inline]
    pub fn grid(&self) -> &Grid<Cell> {
        &self.grid
    }

    #[inline]
    pub fn mode(&self) -> &TermMode {
        &self.mode
    }

    pub fn swap_alt(&mut self) {
        self.alt = !self.alt;
        ::std::mem::swap(&mut self.grid, &mut self.alt_grid);
        ::std::mem::swap(&mut self.cursor, &mut self.alt_cursor);

        if self.alt {
            self.grid.clear(|c| c.reset());
        }
    }

    #[inline]
    pub fn cursor(&self) -> &Cursor {
        &self.cursor
    }

    /// Set character in current cursor position
    fn set_char(&mut self, c: char) {
        if self.cursor.col == self.grid.num_cols() {
            println!("wrapping");
            self.cursor.line += 1;
            self.cursor.col = Column(0);
        }

        if self.cursor.line == self.grid.num_lines() {
            panic!("cursor fell off grid");
        }

        let cell = &mut self.grid[&self.cursor];
        cell.c = c;
        cell.fg = self.fg;
        cell.bg = self.bg;
        cell.flags = self.attr;
    }

    /// Convenience function for scrolling
    fn scroll(&mut self, lines: Line, direction: ScrollDirection) {
        println!("[TERM] scrolling {} {} lines", direction, lines);
        match direction {
            ScrollDirection::Down => {
                // Scrolled down, so need to clear from bottom
                self.grid.scroll(self.scroll_region.clone(), *lines as isize);
                let start = self.scroll_region.end - lines;
                self.grid.clear_region(start..self.scroll_region.end, |c| c.reset());
            },
            ScrollDirection::Up => {
                // Scrolled up, clear from top
                self.grid.scroll(self.scroll_region.clone(), -(*lines as isize));
                let end = self.scroll_region.start + lines;
                self.grid.clear_region(self.scroll_region.start..end, |c| c.reset());
            }
        }
    }
}

/// Which direction to scroll
#[derive(Debug)]
enum ScrollDirection {
    /// Scroll up
    ///
    /// Lines move down
    Up,

    /// Scroll down
    ///
    /// Lines move up
    Down,
}

impl fmt::Display for ScrollDirection {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ScrollDirection::Up => write!(f, "up"),
            ScrollDirection::Down => write!(f, "down"),
        }
    }
}

impl ansi::TermInfo for Term {
    #[inline]
    fn rows(&self) -> usize {
        *self.grid.num_lines() as usize
    }

    #[inline]
    fn cols(&self) -> usize {
        *self.grid.num_cols() as usize
    }
}

impl ansi::Handler for Term {
    /// A character to be displayed
    #[inline]
    fn input(&mut self, c: char) {
        self.set_char(c);
        self.cursor.col += 1;
    }

    #[inline]
    fn goto(&mut self, x: i64, y: i64) {
        println!("goto: x={}, y={}", x, y);
        self.cursor.line = Line(y as usize);
        self.cursor.col = Column(x as usize);
    }

    #[inline]
    fn goto_row(&mut self, row: i64) {
        println!("goto_row: {}", row);
        self.cursor.line = Line(row as usize);
    }

    #[inline]
    fn goto_col(&mut self, col: i64) {
        println!("goto_col: {}", col);
        self.cursor.col = Column(col as usize);
    }

    #[inline]
    fn insert_blank(&mut self, num: i64) { println!("insert_blank: {}", num); }

    #[inline]
    fn move_up(&mut self, rows: i64) {
        println!("move_up: {}", rows);
        self.cursor.line -= Line(rows as usize);
    }

    #[inline]
    fn move_down(&mut self, rows: i64) {
        println!("move_down: {}", rows);
        self.cursor.line += Line(rows as usize);
    }

    #[inline]
    fn move_forward(&mut self, cols: i64) {
        println!("move_forward: {}", cols);
        self.cursor.col += Column(cols as usize);
    }

    #[inline]
    fn move_backward(&mut self, spaces: i64) {
        println!("move_backward: {}", spaces);
        self.cursor.col -= Column(spaces as usize);
    }

    #[inline]
    fn identify_terminal(&mut self) { println!("identify_terminal"); }

    #[inline]
    fn move_down_and_cr(&mut self, rows: i64) { println!("move_down_and_cr: {}", rows); }

    #[inline]
    fn move_up_and_cr(&mut self, rows: i64) { println!("move_up_and_cr: {}", rows); }

    #[inline]
    fn put_tab(&mut self, mut count: i64) {
        println!("put_tab: {}", count);

        let mut col = self.cursor.col;
        while col < self.grid.num_cols() && count != 0 {
            count -= 1;
            loop {
                if col == self.grid.num_cols() || self.tabs[*col as usize] {
                    break;
                }
                col += 1;
            }
        }

        self.cursor.col = col;
    }

    /// Backspace `count` characters
    #[inline]
    fn backspace(&mut self) {
        println!("backspace");
        self.cursor.col -= 1;
    }

    /// Carriage return
    #[inline]
    fn carriage_return(&mut self) {
        println!("carriage_return");
        self.cursor.col = Column(0);
    }

    /// Linefeed
    #[inline]
    fn linefeed(&mut self) {
        println!("linefeed");
        if self.cursor.line + 1 >= self.scroll_region.end {
            self.scroll(Line(1), ScrollDirection::Down);
            self.clear_line(ansi::LineClearMode::Right);
        } else {
            self.cursor.line += 1;
        }
    }

    /// Set current position as a tabstop
    #[inline]
    fn bell(&mut self) { println!("bell"); }

    #[inline]
    fn substitute(&mut self) { println!("substitute"); }

    #[inline]
    fn newline(&mut self) { println!("newline"); }

    #[inline]
    fn set_horizontal_tabstop(&mut self) { println!("set_horizontal_tabstop"); }

    #[inline]
    fn scroll_up(&mut self, rows: i64) {
        println!("scroll_up: {}", rows);
        self.scroll(Line(rows as usize), ScrollDirection::Up);
    }

    #[inline]
    fn scroll_down(&mut self, rows: i64) {
        println!("scroll_down: {}", rows);
        self.scroll(Line(rows as usize), ScrollDirection::Down);
    }

    #[inline]
    fn insert_blank_lines(&mut self, count: i64) {
        println!("insert_blank_lines: {}", count);
        if self.scroll_region.contains(self.cursor.line) {
            self.scroll(Line(count as usize), ScrollDirection::Down);
        }
    }

    #[inline]
    fn delete_lines(&mut self, count: i64) {
        if self.scroll_region.contains(self.cursor.line) {
            self.scroll(Line(count as usize), ScrollDirection::Up);
        }
    }

    #[inline]
    fn erase_chars(&mut self, count: i64) {
        println!("erase_chars: {}", count);
        let col_index = self.cursor.col;
        let count = count as usize;

        let row = &mut self.grid[self.cursor.line];
        for c in &mut row[self.cursor.col..(col_index + count)] {
            c.reset();
        }
    }

    #[inline]
    fn delete_chars(&mut self, count: i64) { println!("delete_chars: {}", count); }

    #[inline]
    fn move_backward_tabs(&mut self, count: i64) { println!("move_backward_tabs: {}", count); }

    #[inline]
    fn move_forward_tabs(&mut self, count: i64) { println!("move_forward_tabs: {}", count); }

    #[inline]
    fn save_cursor_position(&mut self) { println!("save_cursor_position"); }

    #[inline]
    fn restore_cursor_position(&mut self) { println!("restore_cursor_position"); }

    #[inline]
    fn clear_line(&mut self, mode: ansi::LineClearMode) {
        println!("clear_line: {:?}", mode);
        match mode {
            ansi::LineClearMode::Right => {
                let row = &mut self.grid[self.cursor.line];
                for cell in &mut row[self.cursor.col..] {
                    cell.reset();
                }
            },
            _ => (),
        }
    }

    #[inline]
    fn clear_screen(&mut self, mode: ansi::ClearMode) {
        println!("clear_screen: {:?}", mode);
        match mode {
            ansi::ClearMode::Below => {
                let start = self.cursor.line;
                let end = self.grid.num_lines();

                for row in &mut self.grid[start..end] {
                    for cell in row {
                        cell.reset();
                    }
                }
            },
            ansi::ClearMode::All => {
                self.grid.clear(|c| c.reset());
            },
            _ => {
                panic!("ansi::ClearMode::Above not implemented");
            }
        }
    }

    #[inline]
    fn clear_tabs(&mut self, mode: ansi::TabulationClearMode) { println!("clear_tabs: {:?}", mode); }

    #[inline]
    fn reset_state(&mut self) { println!("reset_state"); }

    #[inline]
    fn reverse_index(&mut self) {
        println!("reverse_index");
        // if cursor is at the top
        if self.cursor.col == Column(0) {
            self.scroll(Line(1), ScrollDirection::Up);
        } else {
            self.cursor.col -= 1;
        }
    }

    /// set a terminal attribute
    #[inline]
    fn terminal_attribute(&mut self, attr: Attr) {
        match attr {
            Attr::DefaultForeground => {
                self.fg = DEFAULT_FG;
            },
            Attr::DefaultBackground => {
                self.bg = DEFAULT_BG;
            },
            Attr::Foreground(named_color) => {
                self.fg = COLORS[named_color as usize];
            },
            Attr::Background(named_color) => {
                self.bg = COLORS[named_color as usize];
            },
            Attr::ForegroundSpec(rgb) => {
                self.fg = rgb;
            },
            Attr::BackgroundSpec(rgb) => {
                self.bg = rgb;
            },
            Attr::Reset => {
                self.fg = DEFAULT_FG;
                self.bg = DEFAULT_BG;
                self.attr = cell::Flags::empty();
            },
            Attr::Reverse => self.attr.insert(cell::INVERSE),
            Attr::CancelReverse => self.attr.remove(cell::INVERSE),
            Attr::Bold => self.attr.insert(cell::BOLD),
            Attr::CancelBoldDim => self.attr.remove(cell::BOLD),
            Attr::Italic => self.attr.insert(cell::ITALIC),
            Attr::CancelItalic => self.attr.remove(cell::ITALIC),
            Attr::Underscore => self.attr.insert(cell::UNDERLINE),
            Attr::CancelUnderline => self.attr.remove(cell::UNDERLINE),
            _ => {
                println!("Term got unhandled attr: {:?}", attr);
            }
        }
    }

    #[inline]
    fn set_mode(&mut self, mode: ansi::Mode) {
        println!("set_mode: {:?}", mode);
        match mode {
            ansi::Mode::SwapScreenAndSetRestoreCursor => self.swap_alt(),
            ansi::Mode::ShowCursor => self.mode.insert(mode::SHOW_CURSOR),
            ansi::Mode::CursorKeys => self.mode.insert(mode::APP_CURSOR),
            _ => {
                println!(".. ignoring set_mode");
            }
        }
    }

    #[inline]
    fn unset_mode(&mut self,mode: ansi::Mode) {
        println!("unset_mode: {:?}", mode);
        match mode {
            ansi::Mode::SwapScreenAndSetRestoreCursor => self.swap_alt(),
            ansi::Mode::ShowCursor => self.mode.remove(mode::SHOW_CURSOR),
            ansi::Mode::CursorKeys => self.mode.remove(mode::APP_CURSOR),
            _ => {
                println!(".. ignoring unset_mode");
            }
        }
    }

    #[inline]
    fn set_scrolling_region(&mut self, top: i64, bot: i64) {
        println!("set scroll region: {:?} - {:?}", top, bot);
        // 1 is added to bottom for inclusive range
        self.scroll_region = Line(top as usize)..Line((bot as usize) + 1);
    }
}