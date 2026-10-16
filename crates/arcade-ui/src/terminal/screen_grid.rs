//! High-performance 2D virtual terminal screen grid and ANSI/VT100 escape sequence parser.
//!
//! Maintains an $R \times C$ cell matrix with color styling, cursor positioning, and in-place
//! screen refreshes for live interactive TUI applications (such as `ir pmon`, `ir nettop`, `ir dua`, `ir fm`).

use gpui::{div, prelude::*, px, rgb, rgba, IntoElement, Rgba};
use vte::{Params, Parser, Perform};

use crate::theme::SolarizedTheme;

/// Maps standard 256-color ANSI index to a corresponding RGBA color.
pub fn ansi_256_to_rgba(code: u8) -> Rgba {
    match code {
        // Standard 16 ANSI colors (Solarized mappings)
        0 => rgb(0x073642),  // base02
        1 => rgb(0xdc322f),  // red
        2 => rgb(0x859900),  // green
        3 => rgb(0xb58900),  // yellow
        4 => rgb(0x268bd2),  // blue
        5 => rgb(0xd33682),  // magenta
        6 => rgb(0x2aa198),  // cyan
        7 => rgb(0xeee8d5),  // base2
        8 => rgb(0x002b36),  // base03
        9 => rgb(0xcb4b16),  // orange
        10 => rgb(0x586e75), // base01
        11 => rgb(0x657b83), // base00
        12 => rgb(0x839496), // base0
        13 => rgb(0x6c71c4), // violet
        14 => rgb(0x93a1a1), // base1
        15 => rgb(0xfdf6e3), // base3

        // 16..=231: 6x6x6 color cube
        16..=231 => {
            let n = code - 16;
            let b_val = n % 6;
            let g_val = (n / 6) % 6;
            let r_val = n / 36;
            let to_comp = |v: u8| if v == 0 { 0u8 } else { 55 + 40 * v };
            let r = to_comp(r_val) as u32;
            let g = to_comp(g_val) as u32;
            let b = to_comp(b_val) as u32;
            rgb((r << 16) | (g << 8) | b)
        }

        // 232..=255: grayscale ramp from black to white
        232..=255 => {
            let gray = (8 + (code - 232) as u32 * 10) & 0xFF;
            rgb((gray << 16) | (gray << 8) | gray)
        }
    }
}

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
    /// Underlined text decoration.
    pub underline: bool,
    /// Inverted / reverse video (swaps fg and bg).
    pub reverse: bool,
}

impl Default for GridCell {
    fn default() -> Self {
        Self {
            c: ' ',
            fg: None,
            bg: None,
            bold: false,
            dim: false,
            underline: false,
            reverse: false,
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

    /// Clears all cells in this row with a given background color.
    pub fn clear_with_bg(&mut self, bg: Option<Rgba>) {
        self.cells.fill(GridCell {
            c: ' ',
            fg: None,
            bg,
            bold: false,
            dim: false,
            underline: false,
            reverse: false,
        });
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
    /// Whether the cursor is visibly rendered.
    pub cursor_visible: bool,

    // Active ANSI styling attributes:
    current_fg: Option<Rgba>,
    current_bg: Option<Rgba>,
    current_bold: bool,
    current_dim: bool,
    current_underline: bool,
    current_reverse: bool,
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
            cursor_visible: true,
            current_fg: None,
            current_bg: None,
            current_bold: false,
            current_dim: false,
            current_underline: false,
            current_reverse: false,
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
            .enumerate()
            .map(|(r_idx, row)| {
                // Group contiguous cells with identical formatting into single text segments for high render speed
                let mut segments: Vec<(String, Rgba, Option<Rgba>, bool, bool)> = Vec::new();

                for (c_idx, cell) in row.cells.iter().enumerate() {
                    let is_cursor = self.cursor_visible && r_idx == self.cursor_row && c_idx == self.cursor_col;

                    let mut fg = if cell.dim {
                        theme.text_muted
                    } else if cell.bold {
                        cell.fg.unwrap_or(theme.text_bright)
                    } else {
                        cell.fg.unwrap_or(default_fg)
                    };
                    let mut bg = cell.bg;
                    let bold = cell.bold;
                    let underline = cell.underline;

                    // Reverse video swap
                    if cell.reverse {
                        let orig_fg = fg;
                        fg = bg.unwrap_or(theme.bg_canvas);
                        bg = Some(orig_fg);
                    }

                    // Render cursor cell as prominent cyan block with inverted text
                    if is_cursor {
                        fg = theme.bg_canvas;
                        bg = Some(theme.syntax_cyan);
                    }

                    if let Some((last_text, last_fg, last_bg, last_bold, last_ul)) = segments.last_mut() {
                        if !is_cursor && *last_fg == fg && *last_bg == bg && *last_bold == bold && *last_ul == underline {
                            last_text.push(cell.c);
                            continue;
                        }
                    }

                    let mut s = String::with_capacity(16);
                    s.push(cell.c);
                    segments.push((s, fg, bg, bold, underline));
                }

                div()
                    .flex()
                    .items_center()
                    .h(px(16.0))
                    .font_family("Consolas, 'Cascadia Code', 'Fira Code', 'Courier New', monospace")
                    .text_size(px(12.0))
                    .line_height(px(16.0))
                    .children(segments.into_iter().map(|(text, fg, bg, bold, underline)| {
                        div()
                            .text_color(fg)
                            .when(bold, |p| p.font_weight(gpui::FontWeight::BOLD))
                            .when(underline, |p| p.underline())
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
                cell.underline = self.current_underline;
                cell.reverse = self.current_reverse;
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

    fn csi_dispatch(&mut self, params: &Params, intermediates: &[u8], _ignore: bool, action: char) {
        // DEC Private Mode handling (e.g. `\x1b[?25h`, `\x1b[?25l`, `\x1b[?1049h`)
        if intermediates.contains(&b'?') {
            for param in params.iter() {
                let code = param.first().copied().unwrap_or(0);
                match (code, action) {
                    // Show / Hide Cursor
                    (25, 'h') => self.cursor_visible = true,
                    (25, 'l') => self.cursor_visible = false,
                    // Alternate Screen Buffer Enter
                    (1049, 'h') | (47, 'h') => {
                        self.clear();
                    }
                    // Alternate Screen Buffer Exit
                    (1049, 'l') | (47, 'l') => {
                        self.clear();
                    }
                    _ => {}
                }
            }
            return;
        }

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

            // Cursor Horizontal Absolute: CSI [n] G or `
            'G' | '`' => {
                let c = params.iter().next().and_then(|p| p.first().copied()).unwrap_or(1).max(1) as usize;
                self.cursor_col = (c.saturating_sub(1)).min(self.cols.saturating_sub(1));
            }

            // Cursor Vertical Absolute: CSI [n] d
            'd' => {
                let r = params.iter().next().and_then(|p| p.first().copied()).unwrap_or(1).max(1) as usize;
                self.cursor_row = (r.saturating_sub(1)).min(self.rows.saturating_sub(1));
            }

            // Cursor Next Line: CSI [n] E
            'E' => {
                let n = params.iter().next().and_then(|p| p.first().copied()).unwrap_or(1).max(1) as usize;
                self.cursor_col = 0;
                self.cursor_row = (self.cursor_row + n).min(self.rows.saturating_sub(1));
            }

            // Cursor Previous Line: CSI [n] F
            'F' => {
                let n = params.iter().next().and_then(|p| p.first().copied()).unwrap_or(1).max(1) as usize;
                self.cursor_col = 0;
                self.cursor_row = self.cursor_row.saturating_sub(n);
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
                                    *cell = GridCell {
                                        bg: self.current_bg,
                                        ..GridCell::default()
                                    };
                                }
                            }
                        }
                        for r_idx in (self.cursor_row + 1)..self.rows {
                            if let Some(r) = self.rows_data.get_mut(r_idx) {
                                r.clear_with_bg(self.current_bg);
                            }
                        }
                    }
                    // 1: clear from start of screen to cursor
                    1 => {
                        for r_idx in 0..self.cursor_row {
                            if let Some(r) = self.rows_data.get_mut(r_idx) {
                                r.clear_with_bg(self.current_bg);
                            }
                        }
                        if let Some(r) = self.rows_data.get_mut(self.cursor_row) {
                            for col in 0..=self.cursor_col.min(self.cols.saturating_sub(1)) {
                                if let Some(cell) = r.cells.get_mut(col) {
                                    *cell = GridCell {
                                        bg: self.current_bg,
                                        ..GridCell::default()
                                    };
                                }
                            }
                        }
                    }
                    // 2 or 3: clear entire display
                    2 | 3 => {
                        for row in &mut self.rows_data {
                            row.clear_with_bg(self.current_bg);
                        }
                        self.cursor_col = 0;
                        self.cursor_row = 0;
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
                                    *cell = GridCell {
                                        bg: self.current_bg,
                                        ..GridCell::default()
                                    };
                                }
                            }
                        }
                        // 1: clear from start to cursor
                        1 => {
                            for col in 0..=self.cursor_col.min(self.cols.saturating_sub(1)) {
                                if let Some(cell) = r.cells.get_mut(col) {
                                    *cell = GridCell {
                                        bg: self.current_bg,
                                        ..GridCell::default()
                                    };
                                }
                            }
                        }
                        // 2: clear entire line
                        2 => {
                            r.clear_with_bg(self.current_bg);
                        }
                        _ => {}
                    }
                }
            }

            // Erase Characters: CSI [n] X
            'X' => {
                let n = params.iter().next().and_then(|p| p.first().copied()).unwrap_or(1).max(1) as usize;
                if let Some(r) = self.rows_data.get_mut(self.cursor_row) {
                    for col in self.cursor_col..(self.cursor_col + n).min(self.cols) {
                        if let Some(cell) = r.cells.get_mut(col) {
                            *cell = GridCell {
                                bg: self.current_bg,
                                ..GridCell::default()
                            };
                        }
                    }
                }
            }

            // Delete Characters: CSI [n] P
            'P' => {
                let n = params.iter().next().and_then(|p| p.first().copied()).unwrap_or(1).max(1) as usize;
                if let Some(r) = self.rows_data.get_mut(self.cursor_row) {
                    for _ in 0..n {
                        if self.cursor_col < r.cells.len() {
                            r.cells.remove(self.cursor_col);
                            r.cells.push(GridCell::default());
                        }
                    }
                }
            }

            // Insert Blank Characters: CSI [n] @
            '@' => {
                let n = params.iter().next().and_then(|p| p.first().copied()).unwrap_or(1).max(1) as usize;
                if let Some(r) = self.rows_data.get_mut(self.cursor_row) {
                    for _ in 0..n {
                        if self.cursor_col <= r.cells.len() {
                            r.cells.insert(self.cursor_col, GridCell::default());
                            r.cells.truncate(self.cols);
                        }
                    }
                }
            }

            // Select Graphic Rendition (SGR): CSI [params] m
            'm' => {
                let flat: Vec<u16> = params.iter().flat_map(|sub| sub.iter().copied()).collect();
                if flat.is_empty() {
                    // Reset all
                    self.current_fg = None;
                    self.current_bg = None;
                    self.current_bold = false;
                    self.current_dim = false;
                    self.current_underline = false;
                    self.current_reverse = false;
                    return;
                }

                let mut i = 0;
                while i < flat.len() {
                    match flat[i] {
                        0 => {
                            // Normal / Reset
                            self.current_fg = None;
                            self.current_bg = None;
                            self.current_bold = false;
                            self.current_dim = false;
                            self.current_underline = false;
                            self.current_reverse = false;
                        }
                        1 => self.current_bold = true,
                        2 => self.current_dim = true,
                        4 => self.current_underline = true,
                        7 => self.current_reverse = true,
                        22 => {
                            self.current_bold = false;
                            self.current_dim = false;
                        }
                        24 => self.current_underline = false,
                        27 => self.current_reverse = false,

                        // Standard foreground colors (Solarized mappings)
                        30 => self.current_fg = Some(rgb(0x073642)), // Black
                        31 => self.current_fg = Some(rgb(0xdc322f)), // Red
                        32 => self.current_fg = Some(rgb(0x859900)), // Green
                        33 => self.current_fg = Some(rgb(0xb58900)), // Yellow
                        34 => self.current_fg = Some(rgb(0x268bd2)), // Blue
                        35 => self.current_fg = Some(rgb(0xd33682)), // Magenta
                        36 => self.current_fg = Some(rgb(0x2aa198)), // Cyan
                        37 => self.current_fg = Some(rgb(0xeee8d5)), // White
                        38 => {
                            // Extended foreground: 38;5;n (256-color) or 38;2;r;g;b (24-bit RGB)
                            if i + 2 < flat.len() && flat[i + 1] == 5 {
                                let color_idx = (flat[i + 2] & 0xFF) as u8;
                                self.current_fg = Some(ansi_256_to_rgba(color_idx));
                                i += 2;
                            } else if i + 4 < flat.len() && flat[i + 1] == 2 {
                                let r = (flat[i + 2] & 0xFF) as u32;
                                let g = (flat[i + 3] & 0xFF) as u32;
                                let b = (flat[i + 4] & 0xFF) as u32;
                                self.current_fg = Some(rgb((r << 16) | (g << 8) | b));
                                i += 4;
                            }
                        }
                        39 => self.current_fg = None, // Default fg

                        // Standard background colors
                        40 => self.current_bg = Some(rgba(0x07364280)),
                        41 => self.current_bg = Some(rgba(0xdc322f40)),
                        42 => self.current_bg = Some(rgba(0x85990040)),
                        43 => self.current_bg = Some(rgba(0xb5890040)),
                        44 => self.current_bg = Some(rgba(0x268bd240)),
                        45 => self.current_bg = Some(rgba(0xd3368240)),
                        46 => self.current_bg = Some(rgba(0x2aa19840)),
                        47 => self.current_bg = Some(rgba(0xeee8d540)),
                        48 => {
                            // Extended background: 48;5;n (256-color) or 48;2;r;g;b (24-bit RGB)
                            if i + 2 < flat.len() && flat[i + 1] == 5 {
                                let color_idx = (flat[i + 2] & 0xFF) as u8;
                                self.current_bg = Some(ansi_256_to_rgba(color_idx));
                                i += 2;
                            } else if i + 4 < flat.len() && flat[i + 1] == 2 {
                                let r = (flat[i + 2] & 0xFF) as u32;
                                let g = (flat[i + 3] & 0xFF) as u32;
                                let b = (flat[i + 4] & 0xFF) as u32;
                                self.current_bg = Some(rgb((r << 16) | (g << 8) | b));
                                i += 4;
                            }
                        }
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

                        // Bright background colors
                        100 => self.current_bg = Some(rgba(0x586e7540)),
                        101 => self.current_bg = Some(rgba(0xcb4b1640)),
                        102 => self.current_bg = Some(rgba(0x85990040)),
                        103 => self.current_bg = Some(rgba(0xb5890040)),
                        104 => self.current_bg = Some(rgba(0x268bd240)),
                        105 => self.current_bg = Some(rgba(0x6c71c440)),
                        106 => self.current_bg = Some(rgba(0x2aa19840)),
                        107 => self.current_bg = Some(rgba(0xfdf6e340)),

                        _ => {}
                    }
                    i += 1;
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

    #[test]
    fn handles_256_colors_and_truecolor() {
        let mut grid = TerminalScreenGrid::new(40, 5);
        // 256-color fg (color 196 = red in 6x6x6 cube)
        grid.advance_bytes(b"\x1b[38;5;1mRED\x1b[0m");
        assert_eq!(grid.rows_data[0].cells[0].fg, Some(rgb(0xdc322f)));

        // 24-bit Truecolor RGB fg: 255, 128, 64
        grid.advance_bytes(b"\x1b[38;2;255;128;64mTRUECOLOR\x1b[0m");
        assert_eq!(grid.rows_data[0].cells[3].fg, Some(rgb(0xFF8040)));

        // Reverse video
        grid.advance_bytes(b"\x1b[7mREVERSED\x1b[27mNORMAL");
        assert!(grid.rows_data[0].cells[12].reverse);
        assert!(!grid.rows_data[0].cells[20].reverse);
    }

    #[test]
    fn handles_cursor_visibility_and_erase_chars() {
        let mut grid = TerminalScreenGrid::new(40, 5);
        assert!(grid.cursor_visible);

        // Hide cursor
        grid.advance_bytes(b"\x1b[?25l");
        assert!(!grid.cursor_visible);

        // Show cursor
        grid.advance_bytes(b"\x1b[?25h");
        assert!(grid.cursor_visible);

        // Erase character (CSI X)
        grid.advance_bytes(b"ABCDEFGH");
        grid.advance_bytes(b"\x1b[1;3H\x1b[2X"); // At col 3 ('C'), erase 2 characters
        assert_eq!(grid.rows_data[0].cells[2].c, ' ');
        assert_eq!(grid.rows_data[0].cells[3].c, ' ');
        assert_eq!(grid.rows_data[0].cells[4].c, 'E');
    }
}
