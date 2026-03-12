//! VTE-based ANSI escape sequence parser

use crate::color::{Color, NamedColor};
use crate::grid::CellAttributes;
use crate::terminal::SharedTerminal;
use vte::{Params, Perform};

/// VTE-based ANSI escape sequence parser
pub struct Parser {
    terminal: SharedTerminal,
    vte_parser: vte::Parser,
}

impl Parser {
    pub fn new(terminal: SharedTerminal) -> Self {
        Self {
            terminal,
            vte_parser: vte::Parser::new(),
        }
    }

    pub fn process(&mut self, data: &[u8]) {
        let mut performer = Performer {
            terminal: self.terminal.clone(),
        };
        self.vte_parser.advance(&mut performer, data);
    }
}

struct Performer {
    terminal: SharedTerminal,
}

impl Perform for Performer {
    fn print(&mut self, c: char) {
        let mut term = self.terminal.lock();
        if term.grid.is_scrolled() {
            term.grid.scroll_to_bottom();
        }
        term.selection = None;
        term.put_char(c);
    }

    fn execute(&mut self, byte: u8) {
        let mut term = self.terminal.lock();
        match byte {
            0x07 => term.ring_bell(),
            0x08 => term.backspace(),
            0x09 => term.tab(),
            0x0a..=0x0c => term.linefeed(),
            0x0d => term.carriage_return(),
            _ => {}
        }
    }

    fn hook(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, _action: char) {}

    fn put(&mut self, _byte: u8) {}

    fn unhook(&mut self) {}

    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        if params.is_empty() {
            return;
        }

        let cmd = params[0];
        let is_title_cmd = cmd == b"0" || cmd == b"1" || cmd == b"2";
        if is_title_cmd && params.len() > 1 {
            if let Ok(title) = std::str::from_utf8(params[1]) {
                let mut term = self.terminal.lock();
                term.title = title.to_string();
            }
        }
    }

    fn csi_dispatch(&mut self, params: &Params, intermediates: &[u8], _ignore: bool, action: char) {
        let mut term = self.terminal.lock();
        let params: Vec<u16> = params.iter().flat_map(|s| s.iter().copied()).collect();

        match action {
            '@' => {
                let count = params.first().copied().unwrap_or(1).max(1) as usize;
                term.insert_blank(count);
            }
            'A' => {
                let count = params.first().copied().unwrap_or(1).max(1) as usize;
                term.move_cursor_up(count);
            }
            'B' | 'e' => {
                let count = params.first().copied().unwrap_or(1).max(1) as usize;
                term.move_cursor_down(count);
            }
            'C' | 'a' => {
                let count = params.first().copied().unwrap_or(1).max(1) as usize;
                term.move_cursor_forward(count);
            }
            'D' => {
                let count = params.first().copied().unwrap_or(1).max(1) as usize;
                term.move_cursor_backward(count);
            }
            'E' => {
                let count = params.first().copied().unwrap_or(1).max(1) as usize;
                term.move_cursor_down(count);
                term.carriage_return();
            }
            'F' => {
                let count = params.first().copied().unwrap_or(1).max(1) as usize;
                term.move_cursor_up(count);
                term.carriage_return();
            }
            'G' | '`' => {
                let col = params.first().copied().unwrap_or(1).max(1) as usize - 1;
                term.cursor_x = col.min(term.cols.saturating_sub(1));
            }
            'H' | 'f' => {
                let row = params.first().copied().unwrap_or(1).max(1) as usize - 1;
                let col = params.get(1).copied().unwrap_or(1).max(1) as usize - 1;
                term.move_cursor_to(col, row);
            }
            'J' => {
                let mode = params.first().copied().unwrap_or(0);
                term.erase_in_display(mode);
            }
            'K' => {
                let mode = params.first().copied().unwrap_or(0);
                term.erase_in_line(mode);
            }
            'L' => {
                let count = params.first().copied().unwrap_or(1).max(1) as usize;
                term.insert_lines(count);
            }
            'M' => {
                let count = params.first().copied().unwrap_or(1).max(1) as usize;
                term.delete_lines(count);
            }
            'P' => {
                let count = params.first().copied().unwrap_or(1).max(1) as usize;
                term.delete_chars(count);
            }
            'S' => {
                let count = params.first().copied().unwrap_or(1).max(1) as usize;
                term.scroll_up(count);
            }
            'T' => {
                let count = params.first().copied().unwrap_or(1).max(1) as usize;
                term.scroll_down(count);
            }
            'X' => {
                let count = params.first().copied().unwrap_or(1).max(1) as usize;
                let cursor_x = term.cursor_x;
                let cursor_y = term.cursor_y;
                let cols = term.cols;
                for i in 0..count {
                    if cursor_x + i < cols {
                        term.grid.clear_cell(cursor_x + i, cursor_y);
                    }
                }
                term.mark_dirty();
            }
            'd' => {
                let row = params.first().copied().unwrap_or(1).max(1) as usize - 1;
                term.cursor_y = row.min(term.rows.saturating_sub(1));
            }
            'h' => {
                if intermediates.contains(&b'?') {
                    for p in &params {
                        match *p {
                            7 => term.mode_autowrap = true,
                            25 => term.cursor_visible = true,
                            1049 => {
                                term.save_cursor();
                                term.erase_in_display(2);
                            }
                            _ => {}
                        }
                    }
                }
            }
            'l' => {
                if intermediates.contains(&b'?') {
                    for p in &params {
                        match *p {
                            7 => term.mode_autowrap = false,
                            25 => term.cursor_visible = false,
                            1049 => {
                                term.erase_in_display(2);
                                term.restore_cursor();
                            }
                            _ => {}
                        }
                    }
                }
            }
            'm' => {
                if params.is_empty() {
                    term.current_attr.reset();
                } else {
                    parse_sgr(&params, &mut term.current_attr);
                }
            }
            'r' => {
                let top = params.first().copied().unwrap_or(1) as usize;
                let bottom = params.get(1).copied().unwrap_or(term.rows as u16) as usize;
                term.set_scrolling_region(top, bottom);
            }
            's' => term.save_cursor(),
            'u' => term.restore_cursor(),
            _ => {
                log::trace!("Unhandled CSI: {:?} {} {:?}", params, action, intermediates);
            }
        }
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], _ignore: bool, byte: u8) {
        let mut term = self.terminal.lock();
        match (intermediates, byte) {
            ([], b'7') => term.save_cursor(),
            ([], b'8') => term.restore_cursor(),
            ([], b'D') => term.linefeed(),
            ([], b'E') => {
                term.carriage_return();
                term.linefeed();
            }
            ([], b'M') => {
                if term.cursor_y == term.scroll_top {
                    term.scroll_down(1);
                } else if term.cursor_y > 0 {
                    term.cursor_y -= 1;
                }
            }
            ([], b'c') => term.reset(),
            ([b'#'], b'8') => {
                for y in 0..term.rows {
                    for x in 0..term.cols {
                        term.grid.set(x, y, 'E', CellAttributes::default());
                    }
                }
                term.mark_dirty();
            }
            _ => {
                log::trace!("Unhandled ESC: {:?} {}", intermediates, byte as char);
            }
        }
    }
}

fn parse_sgr(params: &[u16], attr: &mut CellAttributes) {
    let mut i = 0;
    while i < params.len() {
        let p = params[i];
        match p {
            0 => attr.reset(),
            1 => attr.bold = true,
            2 => attr.dim = true,
            3 => attr.italic = true,
            4 => attr.underline = true,
            7 => attr.inverse = true,
            8 => attr.hidden = true,
            9 => attr.strikethrough = true,
            21 => attr.bold = false,
            22 => {
                attr.bold = false;
                attr.dim = false;
            }
            23 => attr.italic = false,
            24 => attr.underline = false,
            27 => attr.inverse = false,
            28 => attr.hidden = false,
            29 => attr.strikethrough = false,
            30..=37 => attr.fg = Color::Named(NamedColor::from_index((p - 30) as u8).unwrap()),
            38 => {
                if i + 2 < params.len() && params[i + 1] == 5 {
                    attr.fg = Color::Indexed(params[i + 2] as u8);
                    i += 2;
                } else if i + 4 < params.len() && params[i + 1] == 2 {
                    attr.fg = Color::Rgb(
                        params[i + 2] as u8,
                        params[i + 3] as u8,
                        params[i + 4] as u8,
                    );
                    i += 4;
                }
            }
            39 => attr.fg = Color::Default,
            40..=47 => attr.bg = Color::Named(NamedColor::from_index((p - 40) as u8).unwrap()),
            48 => {
                if i + 2 < params.len() && params[i + 1] == 5 {
                    attr.bg = Color::Indexed(params[i + 2] as u8);
                    i += 2;
                } else if i + 4 < params.len() && params[i + 1] == 2 {
                    attr.bg = Color::Rgb(
                        params[i + 2] as u8,
                        params[i + 3] as u8,
                        params[i + 4] as u8,
                    );
                    i += 4;
                }
            }
            49 => attr.bg = Color::Default,
            90..=97 => attr.fg = Color::Named(NamedColor::from_index((p - 90 + 8) as u8).unwrap()),
            100..=107 => {
                attr.bg = Color::Named(NamedColor::from_index((p - 100 + 8) as u8).unwrap())
            }
            _ => {}
        }
        i += 1;
    }
}
