//! High-performance 2D virtual terminal screen grid and ANSI/VT100 escape sequence parser.
//!
//! Maintains an $R \times C$ cell matrix with color styling, cursor positioning, and in-place
//! screen refreshes for live interactive TUI applications (such as `ir pmon`, `ir nettop`, `ir dua`, `ir fm`).

use gpui::{div, prelude::*, px, rgb, rgba, IntoElement, Rgba};
use vte::{Params, Parser, Perform};

use crate::theme::SolarizedTheme;

/// A single rendered character cell inside the terminal grid.
#[derive(Clone, Debug, PartialEq)]
pub struct GridCell {
    /// Character rendered at this cell.
    pub c: char,
    /// Custom foreground color override (if set by ANSI sequence).
    pub fg: Option<Rgba>,
    /// Custom background color override (if set by ANSI sequence).
    pub bg: Option<Rgba>,
    /// Bold text weight.
    pub bold: bool,
    /// Dimmed text weight.
    pub dim: bool,
}

impl Default for GridCell {
    fn default() -> Self {
        Self {
            c: ' ',
            fg: None,
            bg: None,
            bold: false,
            dim: false,
        }
    }
}

/// A contiguous line of character cells.
#[derive(Clone, Debug)]
pub struct GridRow {
    /// Ordered cells in this row.
    pub cells: Vec<GridCell>,
}

impl GridRow {
    /// Creates a blank row of length `cols`.
    pub fn new(cols: usize) -> Self {
        Self {
            cells: vec![GridCell::default(); cols],
        }
    }

    /// Clears all cells in this row to default spaces.
    pub fn clear(&mut self) {
        for cell in &mut self.cells {
            *cell = GridCell::default();
        }
    }

    /// Returns the plain text content of this row with trailing whitespace trimmed.
    pub fn text(&self) -> String {
        let s: String = self.cells.iter().map(|c| c.c).collect();
        s.trim_end().to_string()
    }
}

/// Virtual terminal screen grid maintaining a 2D matrix of cells and cursor tracking.
#[derive(Clone, Debug)]
pub struct TerminalScreenGrid {
    /// Number of visible columns.
    pub cols: usize,
    /// Number of visible rows.
    pub rows: usize,
    /// 2D row grid.
    pub rows_data: Vec<GridRow>,
    /// Active 0-based cursor column.
    pub cursor_col: usize,
    /// Active 0-based cursor row.
    pub cursor_row: usize,

    // Active ANSI styling attributes:
    current_fg: Option<Rgba>,
    current_bg: Option<Rgba>,
    current_bold: bool,
    current_dim: bool,
}

impl TerminalScreenGrid {
    /// Creates a new screen grid with the specified column and row dimensions.
    pub fn new(cols: usize, rows: usize) -> Self {
        let cols = cols.max(20);
        let rows = rows.max(5);
        let rows_data = (0..rows).map(|_| GridRow::new(cols)).collect();
        Self {
            cols,
            rows,
            rows_data,
            cursor_col: 0,
            cursor_row: 0,
            current_fg: None,
            current_bg: None,
            current_bold: false,
            current_dim: false,
        }
    }

    /// Resizes the grid to new dimensions, preserving existing content where possible.
    pub fn resize(&mut self, cols: usize, rows: usize) {
        let cols = cols.max(20);
        let rows = rows.max(5);
        if cols == self.cols && rows == self.rows {
            return;
        }

        self.cols = cols;
        self.rows = rows;
        self.rows_data.resize_with(rows, || GridRow::new(cols));
        for r in &mut self.rows_data {
            r.cells.resize_with(cols, GridCell::default);
        }
        self.cursor_col = self.cursor_col.min(cols.saturating_sub(1));
        self.cursor_row = self.cursor_row.min(rows.saturating_sub(1));
    }

    /// Completely clears the grid to blank spaces and returns cursor to (0, 0).
    pub fn clear(&mut self) {
        for row in &mut self.rows_data {
            row.clear();
        }
        self.cursor_col = 0;
        self.cursor_row = 0;
    }

    /// Scrolls the screen content up by one line, discarding the top row and appending a blank row at the bottom.
    pub fn scroll_up(&mut self) {
        if !self.rows_data.is_empty() {
            self.rows_data.remove(0);
            self.rows_data.push(GridRow::new(self.cols));
        }
    }

    /// Feeds raw incoming bytes from a process stdout into the ANSI VT parser and updates the grid.
    pub fn advance_bytes(&mut self, bytes: &[u8]) {
        let mut parser = Parser::new();
        parser.advance(self, bytes);
    }

    /// Renders the 2D grid into styled GPUI div rows for the docked terminal panel.
    pub fn render_rows(&self, theme: &SolarizedTheme) -> Vec<impl IntoElement> {
        let default_fg = theme.text_primary;

        self.rows_data
            .iter()
            .map(|row| {
                // Group contiguous cells with identical formatting into single text segments for high render speed
                let mut segments: Vec<(String, Rgba, Option<Rgba>, bool)> = Vec::new();

                for cell in &row.cells {
                    let fg = if cell.dim {
                        theme.text_muted
                    } else if cell.bold {
                        cell.fg.unwrap_or(theme.text_bright)
                    } else {
                        cell.fg.unwrap_or(default_fg)
                    };
                    let bg = cell.bg;
                    let bold = cell.bold;

                    if let Some((last_text, last_fg, last_bg, last_bold)) = segments.last_mut() {
                        if *last_fg == fg && *last_bg == bg && *last_bold == bold {
                            last_text.push(cell.c);
                            continue;
                        }
                    }

                    let mut s = String::with_capacity(16);
                    s.push(cell.c);
                    segments.push((s, fg, bg, bold));
                }

                div()
                    .flex()
                    .items_center()
                    .h(px(16.0))
                    .font_family("Consolas, 'Cascadia Code', monospace")
                    .text_size(px(12.0))
                    .line_height(px(16.0))
                    .children(segments.into_iter().map(|(text, fg, bg, bold)| {
                        div()
                            .text_color(fg)
                            .when(bold, |p| p.font_weight(gpui::FontWeight::BOLD))
                            .when_some(bg, |p, bg_col| p.bg(bg_col))
                            .child(text)
                    }))
            })
            .collect()
    }
}

// Implement the `vte::Perform` trait to interpret VT100/ANSI terminal sequences
impl Perform for TerminalScreenGrid {
    fn print(&mut self, c: char) {
        if self.cursor_row >= self.rows {
            self.scroll_up();
            self.cursor_row = self.rows.saturating_sub(1);
        }

        if self.cursor_col >= self.cols {
            self.cursor_col = 0;
            self.cursor_row += 1;
            if self.cursor_row >= self.rows {
                self.scroll_up();
                self.cursor_row = self.rows.saturating_sub(1);
            }
        }

        if let Some(row) = self.rows_data.get_mut(self.cursor_row) {
            if let Some(cell) = row.cells.get_mut(self.cursor_col) {
                cell.c = c;
                cell.fg = self.current_fg;
                cell.bg = self.current_bg;
                cell.bold = self.current_bold;
                cell.dim = self.current_dim;
            }
        }

        self.cursor_col += 1;
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            // Carriage return (\r): return cursor to beginning of line
            b'\r' => {
                self.cursor_col = 0;
            }
            // Line feed (\n): move cursor down, scroll if at bottom
            b'\n' => {
                self.cursor_row += 1;
                if self.cursor_row >= self.rows {
                    self.scroll_up();
                    self.cursor_row = self.rows.saturating_sub(1);
                }
            }
            // Backspace (\x08): move cursor left
            b'\x08' => {
                self.cursor_col = self.cursor_col.saturating_sub(1);
            }
            // Tab (\t): advance cursor to next 8-column tab stop
            b'\t' => {
                let next_tab = ((self.cursor_col / 8) + 1) * 8;
                self.cursor_col = next_tab.min(self.cols.saturating_sub(1));
            }
            // Form feed / Clear (\x0c): clear entire display
            b'\x0c' => {
                self.clear();
            }
            _ => {}
        }
    }

    fn hook(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, _action: char) {}
    fn put(&mut self, _byte: u8) {}
    fn unhook(&mut self) {}
    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {}

    fn csi_dispatch(&mut self, params: &Params, _intermediates: &[u8], _ignore: bool, action: char) {
        match action {
            // Cursor Position: CSI [row] ; [col] H (or f)
            'H' | 'f' => {
                let mut iter = params.iter();
                let r = iter.next().and_then(|p| p.first().copied()).unwrap_or(1).max(1);
                let c = iter.next().and_then(|p| p.first().copied()).unwrap_or(1).max(1);
                self.cursor_row = (r as usize - 1).min(self.rows.saturating_sub(1));
                self.cursor_col = (c as usize - 1).min(self.cols.saturating_sub(1));
            }

            // Cursor Up: CSI [n] A
            'A' => {
                let n = params.iter().next().and_then(|p| p.first().copied()).unwrap_or(1).max(1) as usize;
                self.cursor_row = self.cursor_row.saturating_sub(n);
            }

            // Cursor Down: CSI [n] B
            'B' => {
                let n = params.iter().next().and_then(|p| p.first().copied()).unwrap_or(1).max(1) as usize;
                self.cursor_row = (self.cursor_row + n).min(self.rows.saturating_sub(1));
            }

            // Cursor Forward: CSI [n] C
            'C' => {
                let n = params.iter().next().and_then(|p| p.first().copied()).unwrap_or(1).max(1) as usize;
                self.cursor_col = (self.cursor_col + n).min(self.cols.saturating_sub(1));
            }

            // Cursor Back: CSI [n] D
            'D' => {
                let n = params.iter().next().and_then(|p| p.first().copied()).unwrap_or(1).max(1) as usize;
                self.cursor_col = self.cursor_col.saturating_sub(n);
            }

            // Erase in Display: CSI [n] J
            'J' => {
                let n = params.iter().next().and_then(|p| p.first().copied()).unwrap_or(0);
                match n {
                    // 0: clear from cursor to end of screen
                    0 => {
                        if let Some(r) = self.rows_data.get_mut(self.cursor_row) {
                            for col in self.cursor_col..self.cols {
                                if let Some(cell) = r.cells.get_mut(col) {
                                    *cell = GridCell::default();
                                }
                            }
                        }
                        for r_idx in (self.cursor_row + 1)..self.rows {
                            if let Some(r) = self.rows_data.get_mut(r_idx) {
                                r.clear();
                            }
                        }
                    }
                    // 1: clear from start of screen to cursor
                    1 => {
                        for r_idx in 0..self.cursor_row {
                            if let Some(r) = self.rows_data.get_mut(r_idx) {
                                r.clear();
                            }
                        }
                        if let Some(r) = self.rows_data.get_mut(self.cursor_row) {
                            for col in 0..=self.cursor_col.min(self.cols.saturating_sub(1)) {
                                if let Some(cell) = r.cells.get_mut(col) {
                                    *cell = GridCell::default();
                                }
                            }
                        }
                    }
                    // 2 or 3: clear entire display
                    2 | 3 => {
                        self.clear();
                    }
                    _ => {}
                }
            }

            // Erase in Line: CSI [n] K
            'K' => {
                let n = params.iter().next().and_then(|p| p.first().copied()).unwrap_or(0);
                if let Some(r) = self.rows_data.get_mut(self.cursor_row) {
                    match n {
                        // 0: clear from cursor to end of line
                        0 => {
                            for col in self.cursor_col..self.cols {
                                if let Some(cell) = r.cells.get_mut(col) {
                                    *cell = GridCell::default();
                                }
                            }
                        }
                        // 1: clear from start to cursor
                        1 => {
                            for col in 0..=self.cursor_col.min(self.cols.saturating_sub(1)) {
                                if let Some(cell) = r.cells.get_mut(col) {
                                    *cell = GridCell::default();
                                }
                            }
                        }
                        // 2: clear entire line
                        2 => {
                            r.clear();
                        }
                        _ => {}
                    }
                }
            }

            // Select Graphic Rendition (SGR): CSI [params] m
            'm' => {
                if params.is_empty() {
                    // Reset all
                    self.current_fg = None;
                    self.current_bg = None;
                    self.current_bold = false;
                    self.current_dim = false;
                    return;
                }

                for param in params.iter() {
                    let code = param.first().copied().unwrap_or(0);
                    match code {
                        0 => {
                            // Normal / Reset
                            self.current_fg = None;
                            self.current_bg = None;
                            self.current_bold = false;
                            self.current_dim = false;
                        }
                        1 => self.current_bold = true,
                        2 => self.current_dim = true,
                        22 => {
                            self.current_bold = false;
                            self.current_dim = false;
                        }

                        // Standard foreground colors (Solarized mappings)
                        30 => self.current_fg = Some(rgb(0x073642)), // Black
                        31 => self.current_fg = Some(rgb(0xdc322f)), // Red
                        32 => self.current_fg = Some(rgb(0x859900)), // Green
                        33 => self.current_fg = Some(rgb(0xb58900)), // Yellow
                        34 => self.current_fg = Some(rgb(0x268bd2)), // Blue
                        35 => self.current_fg = Some(rgb(0xd33682)), // Magenta
                        36 => self.current_fg = Some(rgb(0x2aa198)), // Cyan
                        37 => self.current_fg = Some(rgb(0xeee8d5)), // White
                        39 => self.current_fg = None,               // Default fg

                        // Standard background colors
                        40 => self.current_bg = Some(rgba(0x07364280)),
                        41 => self.current_bg = Some(rgba(0xdc322f40)),
                        42 => self.current_bg = Some(rgba(0x85990040)),
                        43 => self.current_bg = Some(rgba(0xb5890040)),
                        44 => self.current_bg = Some(rgba(0x268bd240)),
                        45 => self.current_bg = Some(rgba(0xd3368240)),
                        46 => self.current_bg = Some(rgba(0x2aa19840)),
                        47 => self.current_bg = Some(rgba(0xeee8d540)),
                        49 => self.current_bg = None, // Default bg

                        // Bright foreground colors
                        90 => self.current_fg = Some(rgb(0x586e75)),  // Bright Black (base01)
                        91 => self.current_fg = Some(rgb(0xcb4b16)),  // Bright Red (orange)
                        92 => self.current_fg = Some(rgb(0x859900)),  // Bright Green
                        93 => self.current_fg = Some(rgb(0xb58900)),  // Bright Yellow
                        94 => self.current_fg = Some(rgb(0x268bd2)),  // Bright Blue
                        95 => self.current_fg = Some(rgb(0x6c71c4)),  // Bright Magenta (violet)
                        96 => self.current_fg = Some(rgb(0x2aa198)),  // Bright Cyan
                        97 => self.current_fg = Some(rgb(0xfdf6e3)),  // Bright White (base3)

                        _ => {}
                    }
                }
            }

            _ => {}
        }
    }

    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_text_and_line_breaks() {
        let mut grid = TerminalScreenGrid::new(40, 10);
        grid.advance_bytes(b"Hello World\r\nSecond Line");

        assert_eq!(grid.rows_data[0].text(), "Hello World");
        assert_eq!(grid.rows_data[1].text(), "Second Line");
    }

    #[test]
    fn handles_cursor_positioning_and_overwrite() {
        let mut grid = TerminalScreenGrid::new(40, 10);
        grid.advance_bytes(b"Line One\r\nLine Two");
        // Move to row 1, col 1 and overwrite
        grid.advance_bytes(b"\x1b[1;1HAAAA");
        assert_eq!(grid.rows_data[0].text(), "AAAA One");
    }

    #[test]
    fn handles_clear_screen_and_in_place_dashboard() {
        let mut grid = TerminalScreenGrid::new(40, 10);
        grid.advance_bytes(b"Frame 1\r\nData");
        assert_eq!(grid.rows_data[0].text(), "Frame 1");

        // Clear screen and redraw Frame 2
        grid.advance_bytes(b"\x1b[2J\x1b[HFrame 2\r\nUpdated Data");
        assert_eq!(grid.rows_data[0].text(), "Frame 2");
        assert_eq!(grid.rows_data[1].text(), "Updated Data");
    }

    #[test]
    fn handles_sgr_colors_and_bold() {
        let mut grid = TerminalScreenGrid::new(40, 5);
        // Cyan and bold
        grid.advance_bytes(b"\x1b[1;36mCYAN BOLD\x1b[0m PLAIN");

        assert!(grid.rows_data[0].cells[0].bold);
        assert_eq!(grid.rows_data[0].cells[0].fg, Some(rgb(0x2aa198)));
        assert!(!grid.rows_data[0].cells[10].bold);
        assert_eq!(grid.rows_data[0].cells[10].fg, None);
    }
}

