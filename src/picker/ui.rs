use std::cmp::max;
use crossterm::terminal::ClearType;
use crossterm::{cursor, style, terminal, QueueableCommand};
use anyhow::Result;

use crate::picker::picker::Picker;
use crate::picker::options::Options;
use std::io::Write;
use std::str::FromStr;
use crossterm::style::Stylize;
use crate::picker::modes::Mode;
use crate::picker::modes::Mode::Filter;

pub struct Ui {
    scroll_off: Option<u16>,
    top: u16,
    width: u16,
    bar: u16,
    col_widths: Vec<usize>,
    opts: Options,
}

impl Ui {
    pub fn new(picker: &Picker, opts: Options) -> Self {
        let term_size = terminal::size().unwrap();
        let position = cursor::position().unwrap();

        let page_size = picker.page().unwrap().len();
        let win_size = page_size + 2;
        let scroll_off = (win_size as u16).checked_sub(term_size.1 - position.1);

        let position = cursor::position().unwrap();

        let mut col_widths = vec![0; picker.lines().first().unwrap().display(&opts.display_columns).len()];
        for line in picker.lines() {
            for (i, col) in line.display(&opts.display_columns).iter().enumerate() {
                col_widths[i] = max(col_widths[i], col.len());
            }
        }

        Ui {
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

    pub fn draw(&mut self, w: &mut impl Write, picker: &Picker) -> Result<()> {
        w.queue(cursor::MoveTo(0, self.top))?
            .queue(terminal::Clear(ClearType::FromCursorDown))?;

        if let Some(page) = picker.page() {
            for (page_lines_idx, all_lines_idx) in page.iter().enumerate() {
                self.render_line(page_lines_idx, *all_lines_idx, w, picker)?;
            }
        }

        w.queue(cursor::MoveTo(0, self.bar))?
            .queue(terminal::Clear(ClearType::CurrentLine))?;

        if picker.mode() == Filter || picker.filter_text().len() > 0 {
            let filter_text = format!("filter:{}", picker.filter_text());
            let style_attr = match picker.mode() {
                Filter => style::Attribute::Reset,
                _ => style::Attribute::Dim,
            };

            w.queue(style::PrintStyledContent(filter_text.attribute(style_attr)))?;
        }

        if picker.num_pages() > 1 {
            let pagination_text = format!("({}/{})", picker.current_page() + 1, picker.num_pages());
            let pagination_len = pagination_text.len() as u16;

            let styled = pagination_text.attribute(style::Attribute::Dim);
            w.queue(cursor::MoveToColumn(self.width - pagination_len))?
               .queue(style::PrintStyledContent(styled))?;
        }

        w.flush()?;

        Ok(())
    }

    fn render_line(&mut self, page_lines_idx: usize, all_lines_idx: usize, w: &mut impl Write, picker: &Picker) -> Result<()> {
        // todo: should this really be as a ref and should we not just pass the line struct
        let cols = picker.lines().get(all_lines_idx).unwrap().display(&self.opts.display_columns);

        // todo: maybe only clear lines that need to change
        w.queue(terminal::Clear(ClearType::CurrentLine))?;

        match picker.mode() {
            Mode::Hint => {
                match picker.get_hint(page_lines_idx) {
                    Some(hint) => self.render_hinted_line(cols, hint, w)?,
                    None => self.render_normal_line(cols, false, w)?
                }
            },
            _ => {
                let selected = page_lines_idx == picker.current_index();
                self.render_normal_line(cols, selected, w)?;
            },
        };

        w.queue(cursor::MoveToNextLine(1))?;

        Ok(())
    }

    fn render_normal_line(&mut self, cols: Vec<String>, selected: bool, w: &mut impl Write) -> Result<()> {
        if selected {
            w.queue(style::SetForegroundColor(
                style::Color::from_str("green").unwrap(),
            ))?;

            w.queue(style::Print('>'))?;
        }

        self.print_text(cols, w)?;

        if selected {
            w.queue(style::SetForegroundColor(style::Color::Reset))?;
        }

        Ok(())
    }

    fn render_hinted_line(&mut self, cols: Vec<String>, hint: String, w: &mut impl Write) -> Result<()> {
        // first print the whole line
        self.print_text(cols, w)?;

        // then print the hint, overwriting the beginning of the printed line (excluding marker)
        w.queue(cursor::MoveToColumn(2))?
            .queue(style::SetForegroundColor(style::Color::DarkGrey))?
            .queue(style::Print(hint))?
            .queue(style::SetForegroundColor(style::Color::Reset))?;

        Ok(())
    }

    fn print_text(&mut self, cols: Vec<String>, w: &mut impl Write) -> Result<()> {
        let mut position = 2;
        for (i, col) in cols.iter().enumerate() {
            w.queue(cursor::MoveToColumn(position))?
                .queue(style::Print(col))?;

            position += (self.col_widths[i] + 2) as u16;
        }

        Ok(())
    }
}
