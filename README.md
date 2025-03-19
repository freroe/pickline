# pickline

Ever wanted to pick a line from a bunch of lines? Look no further - `pickline` is the tool for you! `pickline` reads lines from `stdin`, lets you pick one out, and outputs your pick to `stdout`.

## Installation
`pickline` is installed via `cargo`. Simply clone this repo, and do a `cargo install`.

## Usage
By default, `pickline` treats the input as regular lines, displaying and outputting them whole. If `--delimiter` is passed, `pickline` treats the input as rows of columnar data, which it splits using the provided delimiter. When dealing with columnar data, you can specify which columns `pickline` should display - and which columns should be included in the output printed to `stdout`.

```
pickline: a tool to pick lines

Usage: pickline [OPTIONS]

Options:
      --page-size <page_size>
          a number or 'auto' to have the pages fit the terminal size [default: 8]
  -a, --alphabet <alphabet>
          the alphabet used for hinting [default: asdfhjkl]
  -d, --delimiter <delimiter>
          split on delimiter and treat lines as columnar data
  -c, --cols <columns>
          the columns to display (requires -d)
      --output-cols <output-columns>
          the columns to output - will be joined by delimiter (requires -d)
      --selection-col <selection-column>
          the column used to determine initial selection (requires -d)
      --selection-regex <selection-regex>
          regex used to determine initial selection (requires --selection-col) [default: \S]
  -h, --help
          Print help
  -V, --version
          Print version
```

You can navigate `pickline` using the following keys:
```
k or <up>: move cursor up
j or <down>: move cursor down
/: enter filter mode
f: enter hint mode
]: next page
[: previous page
<enter>: pick the current line
<esc> or q: quit
```

## Motivation
`pickline` is mostly written as an exercise in Rust. I do - however - use the tool on a daily basis for all my line-picking needs.

## Acknowledgement
`pickline` would not be possible without the great work of all people involved with the following projects:
* [anyhow](https://docs.rs/anyhow/latest/anyhow/)
* [crossterm](https://docs.rs/crossterm/latest/crossterm/index.html)
* [clap](https://docs.rs/clap/latest/clap/)
* [regex](https://docs.rs/regex/latest/regex/)

Additionally [flirt](https://git.sr.ht/~hadronized/flirt) has been an invaluable inspiration on how to structure a Rust TUI in a no frills manner. 

## License
This project is licensed under MIT (https://opensource.org/licenses/MIT)

