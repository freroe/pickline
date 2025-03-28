use anyhow::Result;
use crossterm::terminal::ClearType;
use crossterm::{cursor, style, terminal, QueueableCommand};
use std::cmp::{max, min};
use std::collections::HashMap;

use crate::picker::modes::Mode;
use crate::picker::options::{Options, PageSizeOption};
use crate::picker::picker::Picker;
use crossterm::style::Stylize;
use regex::Regex;
use std::io::Write;
use std::str::FromStr;

pub struct Ui {
    cursor: usize,
    mode: Mode,
    input_buffer: String,

    // pagination
    page: usize,
    page_size: usize,
    pages: Vec<Vec<usize>>,

    // hinting
    hints: Option<HashMap<usize, String>>,

    // terminal window
    scroll_off: Option<u16>,
    top: u16,
    width: u16,
    bar: u16,
    col_widths: Vec<usize>,

    // options
    opts: Options,
}

impl Ui {
    pub fn new(picker: &Picker, opts: Options) -> Self {
        let term_size = terminal::size().unwrap();
        let position = cursor::position().unwrap();

        let page_size = match opts.page_size {
            PageSizeOption::Auto => {
                if picker.lines().len() <= term_size.1 as usize {
                    picker.lines().len()
                } else {
                    term_size.1 as usize - 4
                }
            },
            PageSizeOption::Value(n) => n
        };

        let win_size = page_size + 2;
        let scroll_off = (win_size as u16).checked_sub(term_size.1 - position.1);

        let position = cursor::position().unwrap();

        let mut col_widths = vec![0; picker.lines().first().unwrap().display(&opts.display_columns).len()];
        for line in picker.lines() {
            for (i, col) in line.display(&opts.display_columns).iter().enumerate() {
                col_widths[i] = max(col_widths[i], col.len());
            }
        }

        let mut initial_index = 0;
        if let Some(selection) = &opts.selection_column {
            let selected_index = picker.lines().iter().enumerate().find_map(|(i, l)| {
                match l.matches_regex(&Regex::new(selection.1.as_str()).unwrap(), selection.0) {
                    true => Some(i),
                    false => None
                }
            });

            if selected_index.is_some() {
                initial_index = selected_index.unwrap();
            }
        }


        Ui {
            mode: Mode::Normal,
            cursor: initial_index,
            input_buffer: String::new(),

            // pagination
            page: 0,
            page_size,
            pages: Self::calc_pages((0..picker.lines().len()).collect(), page_size),

            // hinting
            hints: None,

            scroll_off,
            width: term_size.0,
            top: position.1 - scroll_off.unwrap_or(0),
            bar: 1 + position.1 - scroll_off.unwrap_or(0) + page_size as u16,
            col_widths,
            opts
        }
    }

    pub fn setup(&mut self, w: &mut impl Write) -> Result<()> {
        terminal::enable_raw_mode()?;

        w.queue(style::ResetColor)?.queue(cursor::Hide)?;

        if let Some(scroll_off) = self.scroll_off {
            w.queue(terminal::ScrollUp(scroll_off))?
                .queue(cursor::MoveUp(scroll_off))?;
        };

        Ok(())
    }

    pub fn cleanup(&mut self, w: &mut impl Write) -> Result<()> {
        terminal::disable_raw_mode()?;

        w.queue(cursor::MoveTo(0, self.top))?.queue(terminal::Clear(ClearType::FromCursorDown))?;

        w.queue(terminal::Clear(ClearType::FromCursorDown))?
            .queue(cursor::SetCursorStyle::DefaultUserShape)?
            .queue(style::ResetColor)?
            .queue(cursor::Show)?;

        Ok(())
    }

    pub fn show_selections(&mut self, w: &mut impl Write, picker: &Picker) -> Result<()> {
        w.queue(style::Print("Current selection:"))?
            .queue(cursor::MoveToNextLine(1))?;

        for l in picker.selected().iter() {
            w.queue(style::Print(l.output(&self.opts.output_columns, self.opts.delimiter.clone())))?
                .queue(cursor::MoveToNextLine(1))?;
        }

        w.flush()?;

        Ok(())
    }

    pub fn draw(&mut self, w: &mut impl Write, picker: &Picker) -> Result<()> {
        w.queue(cursor::MoveTo(0, self.top))?
            .queue(terminal::Clear(ClearType::FromCursorDown))?;

        if self.mode == Mode::DisplaySelection {
            return self.show_selections(w, picker);
        }

        if let Some(page) = self.pages.get(self.page) {
            for (page_lines_idx, all_lines_idx) in page.iter().enumerate() {
                self.render_line(page_lines_idx, *all_lines_idx, w, picker)?;
            }
        }

        w.queue(cursor::MoveTo(0, self.bar))?
            .queue(terminal::Clear(ClearType::CurrentLine))?;

        if self.mode() == Mode::Filter || picker.filter_text().len() > 0 {
            let filter_text = format!("filter:{}", picker.filter_text());
            let style_attr = match self.mode() {
                Mode::Filter => style::Attribute::Reset,
                _ => style::Attribute::Dim,
            };

            w.queue(style::PrintStyledContent(filter_text.attribute(style_attr)))?;
        }

        if self.num_pages() > 1 {
            let pagination_text = format!("({}/{})", self.current_page() + 1, self.num_pages());
            let pagination_len = pagination_text.len() as u16;

            let styled = pagination_text.attribute(style::Attribute::Dim);
            w.queue(cursor::MoveToColumn(self.width - pagination_len))?
               .queue(style::PrintStyledContent(styled))?;
        }

        w.flush()?;

        Ok(())
    }

    fn render_line(&self, page_lines_idx: usize, all_lines_idx: usize, w: &mut impl Write, picker: &Picker) -> Result<()> {
        let cols = picker.lines().get(all_lines_idx).unwrap().display(&self.opts.output_columns);
        let selected = picker.is_selected(all_lines_idx);

        // todo: maybe only clear lines that need to change
        w.queue(terminal::Clear(ClearType::CurrentLine))?;

        match self.mode() {
            Mode::Hint(_) => {
                match self.get_hint(page_lines_idx) {
                    Some(hint) => self.render_hinted_line(cols, hint, selected, w)?,
                    None => self.render_normal_line(cols, false, selected, w)?
                }
            },
            _ => {
                let current = page_lines_idx == self.cursor;
                self.render_normal_line(cols, current, selected, w)?;
            },
        };

        w.queue(cursor::MoveToNextLine(1))?;

        Ok(())
    }

    fn render_normal_line(&self, cols: Vec<String>, current: bool, selected: bool, w: &mut impl Write) -> Result<()> {
        if current {
            w.queue(style::SetForegroundColor(
                style::Color::from_str("green").unwrap(),
            ))?;

            w.queue(style::Print('>'))?;
        }

        if selected {
            w.queue(style::Print('+'))?;
        }

        self.print_text(cols, w)?;

        if current {
            w.queue(style::SetForegroundColor(style::Color::Reset))?;
        }

        Ok(())
    }

    fn render_hinted_line(&self, cols: Vec<String>, hint: String, selected: bool, w: &mut impl Write) -> Result<()> {
        if selected {
            w.queue(style::Print('+'))?;
        }

        // first print the whole line
        self.print_text(cols, w)?;

        // then print the hint, overwriting the beginning of the printed line (excluding marker)
        w.queue(cursor::MoveToColumn(2))?
            .queue(style::SetForegroundColor(style::Color::DarkGrey))?
            .queue(style::Print(hint))?
            .queue(style::SetForegroundColor(style::Color::Reset))?;

        Ok(())
    }

    fn print_text(&self, cols: Vec<String>, w: &mut impl Write) -> Result<()> {
        let mut position = 2;
        for (i, col) in cols.iter().enumerate() {
            w.queue(cursor::MoveToColumn(position))?
                .queue(style::Print(col))?;

            position += (self.col_widths[i] + 2) as u16;
        }

        Ok(())
    }

    // todo: use this more consistently, to align states between modes
    pub fn change_mode(&mut self, mode: Mode) {
        match (self.mode.clone(), mode.clone()) {
            (Mode::Normal, Mode::Hint(_)) => {
                self.hints = self.calculate_hints(self.opts.hint_alphabet.chars().collect());
            }
            (Mode::Hint(_), Mode::Normal) => {
                self.hints = None;
                self.clear_input_buffer();
            }
            _ => {}
        }

        self.mode = mode
    }

    pub fn mode(&self) -> Mode {
        self.mode.clone()
    }

    pub fn set_cursor(&mut self, i: usize) {
        self.cursor = i;
    }

    fn align_cursor(&mut self) {
        if let Some(page) = self.page() {
            self.cursor = min(page.len() - 1, self.cursor);
        } else {
            self.cursor = 0;
        }
    }

    pub fn line_under_cursor(&self) -> Option<usize> {
        if let Some(page) = self.page() {
            return page.get(self.cursor).map(|i| i.clone())
        };

        None
    }

    pub fn move_cursor_up(&mut self) {
        self.cursor = Self::saturating_decrement(self.cursor);
    }

    pub fn move_cursor_down(&mut self) {
        if let Some(page) = self.page() {
            self.cursor = Self::increment_to_max(self.cursor, min(page.len() - 1, self.page_size - 1));
        }
    }

    pub fn paginate(&mut self, indexes: Vec<usize>) {
        self.pages = Self::calc_pages(indexes, self.page_size);

        self.page = 0;
        self.align_cursor();
    }

    pub fn previous_page(&mut self) {
        self.page = Self::saturating_decrement(self.page);
        self.align_cursor()
    }

    pub fn next_page(&mut self) {
        self.page = Self::increment_to_max(self.page, self.num_pages().saturating_sub(1));
        self.align_cursor()
    }

    pub fn page(&self) -> Option<&Vec<usize>> {
        self.pages.get(self.page)
    }

    pub fn num_pages(&self) -> usize {
        self.pages.len()
    }

    pub fn current_page(&self) -> usize {
        self.page
    }

    fn calc_pages(indexes: Vec<usize>, page_size: usize) -> Vec<Vec<usize>> {
        indexes.chunks(page_size).map(|c| c.to_vec()).collect()
    }

    pub fn clear_input_buffer(&mut self) {
        self.input_buffer.clear();
    }

    pub fn push_to_input_buffer(&mut self, c: char) {
        self.input_buffer.push(c);
    }

    pub fn pop_from_input_buffer(&mut self) {
        self.input_buffer.pop();
    }

    pub fn match_hint(&mut self) -> (Option<usize>, bool) {
        let Some(map) = &self.hints else {
            return (None, false)
        };

        let mut valid = false;
        for (idx, hint) in map {
            if hint.contains(&self.input_buffer) {
                valid = true;

                if *hint == self.input_buffer {
                    return (Some(*idx), true);
                }
            }
        }

        (None, valid)
    }

    pub fn get_hint(&self, i: usize) -> Option<String> {
        let Some(map) = &self.hints else {
            return None
        };

        map.get(&i).cloned()
    }

    // todo: think about making hints non deterministic to prevent growing reliant on order
    fn calculate_hints(&mut self, alphabet: Vec<char>) -> Option<HashMap<usize, String>> {
        let Some(indexes) = self.page() else {
            return None
        };

        let mut map = HashMap::with_capacity(indexes.len());

        // minimum length required for uniqueness
        let min_length = (indexes.len() as f64).log(alphabet.len() as f64).ceil() as usize;

        // total possible combinations at this length
        let total_combinations = alphabet.len().pow(min_length as u32);

        // ideal spacing between selected hints
        let spacing = total_combinations / indexes.len();

        for i in 0..indexes.len() {
            let index = (i * spacing) % total_combinations;
            let hint = Self::index_to_hint(index, alphabet.as_ref(), min_length);

            map.insert(i, hint);
        }

        Some(map)
    }

    fn index_to_hint(index: usize, alphabet: &[char], length: usize) -> String {
        let base = alphabet.len();
        let mut result = String::with_capacity(length);
        let mut remaining = index;

        // Convert index to a base-N number where N is alphabet length
        for _ in 0..length {
            let digit = remaining % base;
            remaining /= base;
            result.push(alphabet[digit]);
        }

        // Reverse since we built it in reverse order
        result.chars().rev().collect()
    }

    fn saturating_decrement(n: usize) -> usize {
        n.saturating_sub(1)
    }

    fn increment_to_max(n: usize, max: usize) -> usize {
        min(n + 1, max)
    }
}
