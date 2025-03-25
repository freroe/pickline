use crate::picker::options::{ColumnRange, Options};
use regex::Regex;

// todo: consider having two different types of lines, representing simple and columnar data
pub struct Line {
    data: Vec<String>,
    selected: bool,
}

impl Line {
    fn new(text: &String, delimiter: Option<String>) -> Self {
        let Some(delim) = delimiter else {
            return Self { data: vec![text.to_string()], selected: false };
        };

        let data = text.split(delim.as_str()).map(|s| s.to_string()).collect::<Vec<String>>();

        Self { data, selected: false }
    }

    pub fn toggle_selected(&mut self) {
        let current = self.selected;
        self.selected = !current;
    }

    pub fn is_selected(&self) -> bool {
        self.selected
    }

    // todo: maybe this and the `output` method belongs in ui.rs
    pub fn display(&self, columns: &Option<ColumnRange>) -> Vec<String> {
        match columns {
            None => self.data.clone(),
            Some(range) => Self::filter_columns(&self.data, &range),
        }
    }

    // todo: fix issue where delimiter is cloned into this everywhere..
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
    lines: Vec<Line>,
    filter: Option<String>,
    opts: Options,
}

impl Picker {
    pub fn new(lines: Vec<String>, opts: Options) -> Self {
        let lines = lines.iter().map(|l| Line::new(l, opts.delimiter.clone())).collect::<Vec<Line>>();

        Self {
            lines,
            filter: None,
            opts,
        }
    }

    pub fn result(&self) -> Option<Vec<String>> {
        let selected = self.lines.iter().filter(|l| l.selected).collect::<Vec<&Line>>();
        if selected.len() == 0 {
            return None
        };

        let output = selected.iter().map(|l| l.output(&self.opts.output_columns, self.opts.delimiter.clone())).collect();

        Some(output)
    }

    pub fn lines(&self) -> &Vec<Line> {
        &self.lines
    }

    pub fn toggle_selection(&mut self, index: usize) {
        self.lines.get_mut(index).unwrap().toggle_selected();
    }

    pub fn filter_text(&self) -> String {
        self.filter.clone().unwrap_or_default()
    }

    // todo: look into not cloning here..
    pub fn apply_filter(&mut self, filter: String) -> Vec<usize> {
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

        self.filter = Some(filter);
        indexes.clone()
    }
}
