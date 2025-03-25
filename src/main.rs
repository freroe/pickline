mod picker;

use crate::picker::commands::Command;
use crate::picker::modes::Mode;
use crate::picker::options::Options;
use crate::picker::picker::Picker;
use crate::picker::ui::Ui;
use clap::{crate_authors, crate_version, Arg};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind};
use std::error::Error;
use std::io;

fn main() {
    let matches = clap::Command::new("pickline")
        .version(crate_version!())
        .author(crate_authors!())
        .about("pickline: a tool to pick lines")
        .arg(
            Arg::new("page_size")
                .long("page-size")
                .help("a number or 'auto' to have the pages fit the terminal size")
                .default_value("auto")
        )
        .arg(
            Arg::new("alphabet")
                .long("alphabet")
                .short('a')
                .default_value("asdfhjkl")
                .help("the alphabet used for hinting")
        )
        .arg(
            Arg::new("delimiter")
                .long("delimiter")
                .short('d')
                .help("split on delimiter and treat lines as columnar data")
        )
        .arg(
            Arg::new("columns")
                .long("cols")
                .short('c')
                .help("the columns to display (requires -d)")
                .requires("delimiter")
        )
        .arg(
            Arg::new("output-columns")
                .long("output-cols")
                .help("the columns to output - will be joined by delimiter (requires -d)")
                .requires("delimiter")
        )
        .arg(
            Arg::new("selection-column")
                .long("selection-col")
                .help("the column used to determine initial selection (requires -d)")
                .requires("delimiter")
        )
        .arg(
            Arg::new("selection-regex")
                .long("selection-regex")
                .help("regex used to determine initial selection (requires --selection-col)")
                .default_value("\\S")
                .requires("selection-column")
        )
        .get_matches();

    let opts = Options::from_matches(&matches);

    match run(opts.unwrap()) {
        Err(_) => panic!("error oh no real bad"),
        Ok(None) => (),
        Ok(Some(lines)) => {
            for l in lines {
                println!("{}", l);
            }
        },
    }
}

fn run(opts: Options) -> Result<Option<Vec<String>>, Box<dyn Error>> {
    let lines = io::stdin().lines();

    let mut w = io::stderr();
    let mut picker = Picker::new(lines.map(|l| l.unwrap()).collect(), opts.clone());
    let mut ui = Ui::new(&picker, opts.clone());

    ui.setup(&mut w)?;

    loop {
        ui.draw(&mut w, &picker)?;

        let key_code = next_keycode()?;

        let command = match ui.mode() {
            Mode::Normal => {
               match key_code {
                   KeyCode::Enter => Some(Command::SelectAndExit),
                   KeyCode::Char(' ') => Some(Command::ToggleSelection),
                   KeyCode::Char('j') | KeyCode::Down => Some(Command::MoveDown),
                   KeyCode::Char('k') | KeyCode::Up => Some(Command::MoveUp),
                   KeyCode::Char('[') => Some(Command::PreviousPage),
                   KeyCode::Char(']') => Some(Command::NextPage),
                   KeyCode::Char('s') => Some(Command::ShowSelection),
                   KeyCode::Char('f') => Some(Command::EnterMode(Mode::Hint)),
                   KeyCode::Char('/') => Some(Command::EnterMode(Mode::Filter)),
                   KeyCode::Char('q') | KeyCode::Esc => Some(Command::Exit),
                   _ => None,
               }
            }
            Mode::Hint => {
                let mut tape = ui.tape();

                match key_code {
                    KeyCode::Esc => Some(Command::EnterMode(Mode::Normal)),
                    KeyCode::Backspace => {
                        tape.pop();
                        Some(Command::Hint(tape))
                    }
                    KeyCode::Char(c) => {
                        tape.push(c);
                        Some(Command::Hint(tape))
                    },
                    _ => None,
                }
            }
            Mode::Filter => {
                let mut filter = picker.filter_text();

                // todo: think of a way to cancel the filtering operation and restore the previous filter
                match key_code {
                    KeyCode::Enter => Some(Command::EnterMode(Mode::Normal)),
                    KeyCode::Esc => Some(Command::EnterMode(Mode::Normal)),
                    KeyCode::Backspace => {
                        filter.pop();
                        Some(Command::Filter(filter))
                    }
                    KeyCode::Char(c) => {
                        filter.push(c);
                        Some(Command::Filter(filter))
                    },
                    _ => None,
                }
            }
        };

        if let Some(command) = command {
            match command {
                Command::EnterMode(mode) => ui.change_mode(mode),
                Command::MoveUp => ui.move_cursor_up(),
                Command::MoveDown => ui.move_cursor_down(),
                Command::PreviousPage => ui.previous_page(),
                Command::NextPage => ui.next_page(),
                Command::Filter(s) => {
                    let visible = picker.apply_filter(s);
                    ui.paginate(visible);
                },
                Command::Hint(s) => {
                    if let Some(m) = ui.match_hint(s) {
                        ui.set_cursor(m);

                        if let Some(choice) = ui.line_under_cursor() {
                            picker.toggle_selection(choice);
                            break;
                        }

                    }
                },
                Command::ToggleSelection => {
                    if let Some(choice) = ui.line_under_cursor() {
                        picker.toggle_selection(choice)
                    }
                },
                Command::ShowSelection => {}
                Command::SelectAndExit => {
                    if let Some(choice) = ui.line_under_cursor() {
                        picker.toggle_selection(choice)
                    }

                    break;
                },
                Command::Exit => break,
            }
        }
    }

    ui.cleanup(&mut w)?;

    Ok(picker.result())
}

fn next_keycode() -> std::io::Result<KeyCode> {
    loop {
        if let Ok(Event::Key(KeyEvent {
                                 code,
                                 kind: KeyEventKind::Press,
                                 modifiers: _,
                                 state: _,
                             })) = crossterm::event::read()
        {
            return Ok(code);
        }
    }
}
