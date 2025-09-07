use std::collections::HashSet;
use crate::picker::options::{ColumnRange, Options};
use regex::Regex;

// todo: consider having two different types of lines, representing simple and columnar data
pub struct Line {
    data: Vec<String>,
    original: String,
}

impl Line {
    fn new(text: &String, delimiter: Option<String>) -> Self {
        let Some(delim) = delimiter else {
            return Self { data: vec![text.to_string()], original: text.to_string() };
        };

        let data = text.split(delim.as_str()).map(|s| s.to_string()).collect::<Vec<String>>();

        Self { data, original: text.to_string() }
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

    pub fn matches_regex(&self, regex: &Regex) -> bool {
        regex.is_match(self.original.as_str())
    }

    // todo: maybe try to avoid cloning this much
    fn filter_columns(data: &[String], columns: &ColumnRange) -> Vec<String> {
        match columns {
            ColumnRange::Closed(range) => {
                data.iter()
                    .enumerate()
                    .filter_map(|(i, s)| range.contains(&i).then(|| s.to_string()))
                    .collect()
            },
            ColumnRange::Open(range) => {
                if range.is_empty() {
                    return Vec::new();
                }

                let &last = range.last().unwrap();

                data.iter()
                    .enumerate()
                    .filter_map(|(i, s)| {
                        if i > last || range.contains(&i) {
                            Some(s.to_string())
                        } else {
                            None
                        }
                    })
                    .collect()
            }
        }
    }
}
pub struct Picker {
    lines: Vec<Line>,
    filter: Option<String>,
    selection: HashSet<usize>,
    opts: Options,
}

impl Picker {
    pub fn new(lines: &[String], opts: Options) -> Self {
        let lines = lines.iter().map(|l| Line::new(l, opts.delimiter.clone())).collect::<Vec<Line>>();

        Self {
            lines,
            filter: None,
            selection: HashSet::new(),
            opts,
        }
    }

    pub fn result(&self) -> Option<Vec<String>> {
        match &self.selection {
            s if s.len() > 0 => {
                let selected = s.iter().map(|i| {
                    self.lines.get(*i).unwrap().output(&self.opts.output_columns, self.opts.delimiter.clone())
                });

                Some(selected.collect())
            },
            _ => None,
        }
    }

    pub fn selected(&self) -> Vec<&Line> {
        self.selection.iter().map(|i| self.lines.get(*i).unwrap()).collect::<Vec<&Line>>()
    }

    pub fn lines(&self) -> &[Line] {
        &self.lines
    }

    pub fn toggle_selection(&mut self, index: usize) {
        let _ = match self.selection.contains(&index) {
            false => self.selection.insert(index),
            true => self.selection.remove(&index),
        };
    }

    pub fn is_selected(&self, index: usize) -> bool {
        self.selection.contains(&index)
    }

    pub fn filter_text(&self) -> String {
        self.filter.clone().unwrap_or_default()
    }

    pub fn persist_filter(&mut self, filter: String) {
        self.filter = Some(filter);
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

        indexes.clone()
    }
}
