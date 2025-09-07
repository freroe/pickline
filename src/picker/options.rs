use std::str::FromStr;
use clap::ArgMatches;

#[derive(Clone)]
pub enum ColumnRange {
    Closed(Vec<usize>),
    Open(Vec<usize>),
}

#[derive(Clone)]
pub enum PageSizeOption {
    Auto,
    Value(usize)
}

impl FromStr for PageSizeOption {
    type Err = ();

    fn from_str(input: &str) -> Result<PageSizeOption, ()> {
        match input.to_lowercase().as_str() {
            "auto" => Ok(PageSizeOption::Auto),
            _ => Ok(PageSizeOption::Value(input.parse().unwrap()))
        }
    }
}

#[derive(Clone)]
pub struct Options {
    pub page_size: PageSizeOption,
    pub hint_alphabet: String,
    pub delimiter: Option<String>,
    pub display_columns: Option<ColumnRange>,
    pub output_columns: Option<ColumnRange>,
    pub selection_regex: Option<String>,
}

impl Options {
    pub fn from_matches(matches: &ArgMatches) -> Result<Self, String> {
        let page_size = PageSizeOption::from_str(matches.get_one::<String>("page_size").unwrap());
        let hint_alphabet = matches.get_one::<String>("alphabet").map(String::from);

        let columnar = matches.contains_id("delimiter");
        if !columnar {
            return Ok(Self {
                hint_alphabet: hint_alphabet.unwrap(),
                page_size: page_size.unwrap(),
                delimiter: None,
                display_columns: None,
                output_columns: None,
                selection_regex: None,
            })
        }
        
        let delimiter = matches.get_one::<String>("delimiter").unwrap();

        let display_columns = matches.get_one::<String>("columns").map(Self::parse_column_ranges);
        let output_columns = matches.get_one::<String>("output-columns").map(Self::parse_column_ranges);
        let selection_regex = matches.get_one::<String>("selection-regex").unwrap();

        Ok(Self {
            hint_alphabet: hint_alphabet.unwrap(),
            page_size: page_size.unwrap(),
            delimiter: Some(delimiter.to_string()),
            display_columns,
            output_columns,
            selection_regex: Some(selection_regex.to_string()),
        })
    }

    fn parse_column_ranges(columns_list: &String) -> ColumnRange {
        let mut columns : Vec<usize> = Vec::new();
        for s in columns_list.split(',') {
            match s {
                range if s.contains("..") => {
                    let (start, end) = range.split_once("..").unwrap();

                    let start = match start {
                        start if start.is_empty() => 0,
                        _ => start.parse::<usize>().unwrap(),
                    };

                    if end.is_empty() {
                        columns.push(start);
                        return ColumnRange::Open(columns);
                    }

                    if end.contains("=") {
                        let end = end.replace("=", "").parse::<usize>().unwrap();
                        columns.extend(start..end + 1);
                    } else {
                        let end = end.parse::<usize>().unwrap();
                        columns.extend(start..end)
                    }

                },
                single if s.parse::<usize>().is_ok() => columns.push(single.parse::<usize>().unwrap()),
                _ => {}
            }
        }

        ColumnRange::Closed(columns)
    }
}