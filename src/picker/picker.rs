use std::cmp::{max, min};
use std::collections::HashMap;
use regex::Regex;
use crate::picker::modes::Mode;
use crate::picker::options::{ColumnRange, Options, PageSizeOption};

// todo: consider having two different types of lines, representing simple and columnar data
pub struct Line {
    data: Vec<String>,
}

impl Line {
    fn new(text: &String, delimiter: Option<String>) -> Self {
        let Some(delim) = delimiter else {
            return Self { data: vec![text.to_string()] };
        };

        let data = text.split(delim.as_str()).map(|s| s.to_string()).collect::<Vec<String>>();

        Self { data }
    }

    pub fn display(&self, columns: &Option<ColumnRange>) -> Vec<String> {
        match columns {
            None => self.data.clone(),
            Some(range) => Self::filter_columns(&self.data, &range),
        }
    }

    pub fn output(&self, columns: &Option<ColumnRange>, delimiter: Option<String>) -> String {
        let cols = match columns {
            None => self.data.clone(),
            Some(range) => Self::filter_columns(&self.data, &range),
        };

        cols.join(delimiter.unwrap_or_default().as_str())
    }

    pub fn matches(&self, s: &String) -> bool {
        self.data.iter().any(|x| x.contains(s))
    }

    pub fn matches_regex(&self, regex: &Regex, col: usize) -> bool {
        match self.data.get(col) {
            None => false,
            Some(s) => regex.is_match(s),
        }
    }

    // todo: maybe try to avoid cloning this much
    fn filter_columns(data: &Vec<String>, columns: &ColumnRange) -> Vec<String> {
        let indexes = match columns {
            ColumnRange::Open(range) => {
                let mut range = range.clone();
                let column_cnt = data.len();
                if let Some(last) = range.last() {
                    range.extend(last + 1..column_cnt);
                }

                range
            },
            ColumnRange::Closed(range) => range.clone(),
        };

        data.iter().enumerate().filter_map(|(i, s)| indexes.contains(&i).then(|| s.to_string())).collect()
    }
}
pub struct Picker {
    mode: Mode,
    // selection
    lines: Vec<Line>,
    cursor_idx: usize,
    selection: Option<usize>,
    // pagination
    page_idx: usize,
    page_size: usize,
    pages: Vec<Vec<usize>>,
    // filtering
    filter: Option<String>,
    // hinting
    hints: Option<HashMap<usize, String>>,
    tape: Option<String>,
    // options
    opts: Options,
}

impl Picker {
    pub fn new(lines: Vec<String>, opts: Options) -> Self {
        let page_size = match opts.page_size {
            PageSizeOption::Auto => todo!(),
            PageSizeOption::Value(n) => n
        };

        let lines = lines.iter().map(|l| Line::new(l, opts.delimiter.clone())).collect::<Vec<Line>>();

        let mut initial_index = 0;
        if let Some(selection) = &opts.selection_column {
            let selected_index = lines.iter().enumerate().find_map(|(i, l)| {
                match l.matches_regex(&Regex::new(selection.1.as_str()).unwrap(), selection.0) {
                    true => Some(i),
                    false => None
                }
            });

            if selected_index.is_some() {
                initial_index = selected_index.unwrap();
            }
        }

        let pages = Self::paginate((0..lines.len()).collect(), page_size);
        Self {
            mode: Mode::Normal,
            lines,
            cursor_idx: initial_index,
            page_idx: 0,
            selection: None,
            page_size,
            pages,
            filter: None,
            hints: None,
            tape: None,
            opts,
        }
    }

    pub fn result(&self) -> Option<String> {
        match self.selection {
            None => None,
            Some(i) => self.lines().get(i).map(|l| l.output(&self.opts.output_columns, self.opts.delimiter.clone()))
        }
    }

    // todo: use this more consistently, to align states between modes
    pub fn change_mode(&mut self, mode: Mode) {
        match (self.mode.clone(), mode.clone()) {
            (Mode::Normal, Mode::Hint) => {
                self.hints = self.calculate_hints(self.opts.hint_alphabet.chars().collect());
                self.tape = Some(String::new());
            }
            (Mode::Hint, Mode::Normal) => {
                self.hints = None;
                self.tape = None;
            }
            _ => {}
        }

        self.mode = mode
    }

    pub fn mode(&self) -> Mode {
        self.mode.clone()
    }

    pub fn current_index(&self) -> usize {
        self.cursor_idx
    }

    pub fn lines(&self) -> &Vec<Line> {
        &self.lines
    }

    pub fn select(&mut self) {
       if let Some(page) = self.page() {
           self.selection = page.get(self.cursor_idx).cloned();
       }
    }

    pub fn move_cursor_up(&mut self) {
        self.cursor_idx = Self::saturating_decrement(self.cursor_idx);
    }

    pub fn move_cursor_down(&mut self) {
        if let Some(page) = self.page() {
            self.cursor_idx = Self::increment_to_max(self.cursor_idx, min(page.len() - 1, self.page_size - 1));
        }
    }

    pub fn previous_page(&mut self) {
        self.page_idx = Self::saturating_decrement(self.page_idx);
        self.align_cursor()
    }

    pub fn next_page(&mut self) {
        self.page_idx = Self::increment_to_max(self.page_idx, self.num_pages().saturating_sub(1));
        self.align_cursor()
    }

    pub fn page(&self) -> Option<&Vec<usize>> {
        self.pages.get(self.page_idx)
    }

    pub fn num_pages(&self) -> usize {
        self.pages.len()
    }

    pub fn current_page(&self) -> usize {
        self.page_idx
    }

    pub fn filter_text(&self) -> String {
        self.filter.clone().unwrap_or_default()
    }

    pub fn apply_filter(&mut self, filter: String) {
        let indexes = match &filter {
            f if f.len() == 0 => {
                (0..self.lines.len()).collect()
            },
            f => {
                self.lines.iter().enumerate().filter_map(|(i, l)| {
                    match l.matches(f) {
                        true => Some(i),
                        false => None,
                    }
                }).collect::<Vec<usize>>()
            }
        };

        self.pages = Self::paginate(indexes, self.page_size);
        self.page_idx = 0;
        self.filter = Some(filter);
        self.align_cursor()
    }

    pub fn tape(&self) -> String {
        self.tape.clone().unwrap_or_default()
    }

    pub fn match_hint(&mut self, tape: String) -> bool {
        let Some(map) = &self.hints else {
            return false
        };

        let mut valid_tape = false;
        for (idx, hint) in map {
            if hint.contains(&tape) {
                valid_tape = true;

                if *hint == tape {
                    self.cursor_idx = *idx;
                    return true;
                }
            }
        }

        if valid_tape {
            self.tape = Some(tape);
        }

        false
    }

    pub fn get_hint(&self, i: usize) -> Option<String> {
        let Some(map) = &self.hints else {
            return None
        };

        map.get(&i).cloned()
    }

    fn calculate_hints(&mut self, alphabet: Vec<char>) -> Option<HashMap<usize, String>> {
        let Some(indexes) = self.page() else {
            return None
        };

        let mut map = HashMap::with_capacity(indexes.len());
        let needed_chars = max(1, indexes.len().div_ceil(alphabet.len()));

        let mut hint_idxs = vec![0usize; needed_chars];
        let mut hint_cursor = needed_chars - 1;

        for i in 0..indexes.len() {
            let hint = hint_idxs.iter().map(|&idx| alphabet[idx]).collect();

            map.insert(i, hint);

            if hint_idxs[hint_cursor] == alphabet.len() - 1 && hint_cursor > 0 {
                hint_idxs[hint_cursor] = 0;
                hint_idxs[hint_cursor - 1] += 1;
                hint_cursor = 1;
            } else {
                hint_idxs[hint_cursor] += 1;
            }
        }

        Some(map)
    }

    fn paginate(indexes: Vec<usize>, page_size: usize) -> Vec<Vec<usize>> {
        indexes.chunks(page_size).map(|c| c.to_vec()).collect()
    }

    fn align_cursor(&mut self) {
        if let Some(page) = self.page() {
            self.cursor_idx = min(page.len() - 1, self.cursor_idx);
        } else {
            self.cursor_idx = 0;
        }
    }

    fn saturating_decrement(n: usize) -> usize {
        n.saturating_sub(1)
    }

    fn increment_to_max(n: usize, max: usize) -> usize {
        min(n + 1, max)
    }
}
