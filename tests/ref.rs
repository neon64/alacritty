extern crate alacritty;
extern crate serde_json as json;

use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use alacritty::{Grid, Term, ansi};
use alacritty::index::{Line, Column};
use alacritty::config::Config;
use alacritty::term::Cell;
use alacritty::term::SizeInfo;
use alacritty::util::fmt::{Red, Green};

macro_rules! ref_tests {
    ($($name:ident)*) => {
        $(
            #[test]
            fn $name() {
                let test_dir = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/ref"));
                let test_path = test_dir.join(stringify!($name));
                ref_test(&test_path);
            }
        )*
    }
}

ref_tests! {
    csi_rep
    fish_cc
    indexed_256_colors
    issue_855
    ll
    ll_scrollback
    newline_with_cursor_beyond_scroll_region
    tab_rendering
    tmux_git_log
    tmux_git_log_scrollback
    tmux_htop
    tmux_htop_scrollback
    vim_24bitcolors_bce
    vim_large_window_scroll
    vim_simple_edit
    vim_scrollback_scrolling_disabled
    vttest_cursor_movement_1
    vttest_insert
    vttest_origin_mode_1
    vttest_origin_mode_2
    vttest_scroll
    vttest_tab_clear_set
    zsh_tab_completion
    alternate_screen_buffer_enter
    alternate_screen_buffer_exit
}

fn read_u8<P>(path: P) -> Vec<u8>
    where P: AsRef<Path>
{
    let mut res = Vec::new();
    File::open(path.as_ref()).unwrap()
        .read_to_end(&mut res).unwrap();

    res
}

fn read_string<P>(path: P) -> String
    where P: AsRef<Path>
{
    let mut res = String::new();
    File::open(path.as_ref()).unwrap()
        .read_to_string(&mut res).unwrap();

    res
}

fn ref_test(dir: &Path) {
    let recording = read_u8(dir.join("alacritty.recording"));
    let serialized_size = read_string(dir.join("size.json"));
    let serialized_grid = read_string(dir.join("grid.json"));

    let size: SizeInfo = json::from_str(&serialized_size).unwrap();
    let grid: Grid<Cell> = json::from_str(&serialized_grid).unwrap();

    let mut config = Config::default();
    // Disable scrollback if the ref test has it disabled
    if !grid.scrollback_enabled() {
        config.disable_scrollback();
    }

    // Create new terminal
    let mut terminal = Term::new(&config, size);
    let mut parser = ansi::Processor::new();

    for byte in recording {
        parser.advance(&mut terminal, byte, &mut io::sink());
    }

    // Compare complete grid with scrollback
    if grid != *terminal.grid() {
        for (i, row) in terminal.grid().lines().enumerate() {
            for (j, cell) in row.iter().enumerate() {
                let original_cell = &grid[Line(i)][Column(j)];
                if *original_cell != *cell {
                    println!("[{i}][{j}] {original:?} => {now:?}",
                             i=i, j=j, original=Green(original_cell), now=Red(cell));
                }
            }
        }

        panic!("Ref test failed; grid doesn't match");
    }

    assert_eq!(grid, *terminal.grid());
}
