use anyhow::{bail, Result};
use edit::edit_file;
use itertools::Itertools;
use std::{
    collections::HashMap,
    ffi::OsString,
    fs::{read_dir, DirEntry, File},
    io::{prelude::*, stdin, stdout},
    path::PathBuf,
};
use termion::{event::Key, input::TermRead, raw::IntoRawMode, screen::AlternateScreen};
use tui::{
    backend::TermionBackend,
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Paragraph},
    Terminal,
};

enum Target {
    File,
    Directory,
}

impl Target {
    fn style(self, highlight: bool) -> Style {
        match self {
            Self::File => {
                if highlight {
                    Style::default().fg(Color::Black).bg(Color::White)
                } else {
                    Style::default().fg(Color::White)
                }
            }
            Self::Directory => if highlight {
                Style::default().fg(Color::Black).bg(Color::Blue)
            } else {
                Style::default().fg(Color::Blue)
            }
            .add_modifier(Modifier::BOLD),
        }
    }
}

#[derive(Default)]
struct Options {
    show_hidden: bool,
}

#[derive(Default)]
struct Context {
    cursor: usize,
    previous_locations: HashMap<PathBuf, usize>,
    current_dir: PathBuf,
    options: Options,
}

impl Context {
    fn new() -> Result<Self> {
        let context = Self {
            current_dir: std::env::current_dir()?,
            ..Self::default()
        };
        Ok(context)
    }

    fn current_dir(&self) -> Option<&str> {
        self.current_dir.as_os_str().to_str()
    }

    fn save_location(&mut self) {
        self.previous_locations
            .insert(self.current_dir.clone(), self.cursor);
    }

    fn restore_location(&mut self) {
        match self.previous_locations.get(&self.current_dir) {
            Some(location) => self.cursor = *location,
            None => self.cursor = 0,
        }
    }

    fn target(&self) -> Option<DirEntry> {
        match self.read_directory() {
            Ok(iter) => iter.skip(self.cursor).next(),
            _ => None,
        }
    }

    fn target_dir(&self) -> Result<OsString> {
        let target = self.target();
        match target {
            Some(target) if target.path().is_dir() => Ok(target.file_name()),
            _ => bail!("Error occured when trying to get current target"),
        }
    }

    fn read_directory(&self) -> Result<impl Iterator<Item = DirEntry>> {
        let iterator = read_dir(&self.current_dir)?
            .flat_map(|e| e)
            .filter(|e| {
                !e.path()
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .starts_with(".")
                    || self.options.show_hidden
            })
            .sorted_by(|a, b| a.file_name().cmp(&b.file_name()))
            .sorted_by_key(|e| {
                !e.path()
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .starts_with(".")
            })
            .sorted_by_key(|e| !e.path().is_dir());
        Ok(iterator)
    }

    fn listing(&self) -> Result<Text<'_>> {
        let mut text = Text::default();
        for (line, path) in self.read_directory()?.enumerate() {
            if let Some(input) = path.file_name().to_str() {
                let input = input.to_string();
                let is_dir = path.path().is_dir();
                let highlight = self.cursor == line;
                let mut spans = Spans::default();
                let items = &mut spans.0;
                if is_dir {
                    items.push(Span::styled(input, Target::Directory.style(highlight)));
                    items.push(Span::styled("/", Style::default().fg(Color::Reset)));
                } else {
                    items.push(Span::styled(input, Target::File.style(highlight)));
                }
                text.lines.push(spans);
            }
        }
        Ok(text)
    }
}

fn main() -> Result<()> {
    let stdout = stdout().into_raw_mode()?;
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut context = Context::new()?;

    'update: loop {
        // Clamp the cursor if it's out of bounds
        let amount_files = context.read_directory()?.count();
        if context.cursor >= amount_files {
            context.cursor = amount_files.saturating_sub(1);
        }

        let listing = context.listing()?;

        terminal.draw(|frame| {
            let size = frame.size();

            // Files pane
            let directory = Span::from(context.current_dir().unwrap_or("Invalid directory"));
            let outline = Block::default()
                .title(directory)
                .style(Style::default().fg(Color::LightGreen));
            let files = Paragraph::new(listing).block(outline);
            frame.render_widget(files, size);
        })?;

        for key in stdin().keys() {
            if let Ok(key) = key {
                match key {
                    Key::Char('q') | Key::Ctrl('c') | Key::Ctrl('z') => break 'update,
                    Key::Up | Key::Char('k') => {
                        context.cursor = context.cursor.saturating_sub(1);
                    }
                    Key::Down | Key::Char('j') => {
                        context.cursor = context.cursor.saturating_add(1);
                    }
                    Key::Left | Key::Char('h') => {
                        context.save_location();
                        context.current_dir.pop();
                        context.restore_location();
                    }
                    Key::Right | Key::Char('l') => {
                        if let Ok(target) = context.target_dir() {
                            context.save_location();
                            context.current_dir.push(target);
                            context.restore_location();
                        }
                    }
                    Key::Char('.') => {
                        context.options.show_hidden = !context.options.show_hidden;
                    }
                    Key::Char('e') => {
                        if let Some(target) = context.target() {
                            edit_file(target.path())?;
                            terminal.clear()?;
                        }
                    }
                    Key::Ctrl('d') => {
                        context.cursor = context.cursor.saturating_add(10);
                    }
                    Key::Ctrl('u') => {
                        context.cursor = context.cursor.saturating_sub(10);
                    }
                    Key::Char('o') => {
                        if let Some(target) = context.target() {
                            if open::that(target.path()).is_err() {
                                // Output error to log
                            }
                        }
                    }
                    _ => {}
                }
                continue 'update;
            }
        }
    }

    // Write last entered directory into temporary file
    if let Some(mut path) = dirs::cache_dir() {
        path.push(".rrr");
        let mut file = File::create(path)?;
        write!(&mut file, "{}", context.current_dir.display())?;
    }

    Ok(())
}
