use anyhow::{bail, Result};
use itertools::Itertools;
use std::{
    ffi::OsString,
    fs::{read_dir, DirEntry},
    io::{stdin, stdout},
    path::PathBuf,
};
use termion::{
    event::Key,
    input::{MouseTerminal, TermRead},
    raw::IntoRawMode,
    screen::AlternateScreen,
};
use tui::{
    backend::TermionBackend,
    style::{Color, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};

#[derive(Default)]
struct Context {
    cursor: usize,
    current_dir: PathBuf,
}

impl Context {
    fn new() -> Result<Self> {
        let context = Self {
            current_dir: std::env::current_dir()?,
            ..Self::default()
        };
        Ok(context)
    }

    fn read_directory(&self) -> Result<impl Iterator<Item = DirEntry>> {
        Ok(read_dir(&self.current_dir)?
            .flat_map(|e| e)
            .sorted_by_key(|e| !e.path().file_name().unwrap().to_str().unwrap().starts_with("."))
            .sorted_by_key(|e| !e.path().is_dir()))
    }

    fn current_dir(&self) -> Option<&str> {
        self.current_dir.as_os_str().to_str()
    }

    fn amount_dir(&self) -> Result<usize> {
        Ok(self.read_directory()?.count())
    }

    /// Returns name of directory if target is a directory, otherwise returns error
    fn target_dir(&self) -> Result<OsString> {
        let target = self.read_directory()?.skip(self.cursor).next();
        match target {
            Some(target) if target.path().is_dir() => Ok(target.file_name()),
            _ => bail!("Error occured when trying to get current target"),
        }
    }

    fn listing(&self) -> Result<Text<'_>> {
        let mut text = Text::default();
        for (line, path) in self.read_directory()?.enumerate() {
            if let Some(input) = path.file_name().to_str() {
                let input = input.to_string();
                let mut spans = Spans::default();
                if self.cursor == line {
                    spans.0.push(Span::styled(
                        input,
                        Style::default().fg(Color::Black).bg(Color::White),
                    ));
                } else {
                    spans.0.push(Span::raw(input));
                }
                if path.path().is_dir() {
                    spans.0.push(Span::raw("/"));
                }
                text.lines.push(spans);
            }
        }
        Ok(text)
    }
}

fn main() -> Result<()> {
    let stdout = stdout().into_raw_mode()?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut context = Context::new()?;

    'draw: loop {
        let listing = context.listing()?;
        terminal.draw(|frame| {
            let size = frame.size();

            // Files pane
            let directory = Span::from(context.current_dir().unwrap_or("Invalid directory"));
            let outline = Block::default().borders(Borders::ALL).title(directory);
            let files = Paragraph::new(listing).block(outline);
            frame.render_widget(files, size);
        })?;

        for key in stdin().keys() {
            match key? {
                Key::Char('q') | Key::Ctrl('c') | Key::Ctrl('z') => {
                    std::env::set_current_dir(context.current_dir)?;
                    return Ok(());
                }
                Key::Up => {
                    context.cursor = context.cursor.saturating_sub(1);
                    continue 'draw;
                }
                Key::Down => {
                    if context.cursor < context.amount_dir()?.saturating_sub(1) {
                        context.cursor = context.cursor.saturating_add(1);
                        continue 'draw;
                    }
                }
                Key::Left => {
                    context.current_dir.pop();
                    context.cursor = 0;
                    continue 'draw;
                }
                Key::Right => {
                    if let Ok(target) = context.target_dir() {
                        context.current_dir.push(target);
                        context.cursor = 0;
                        continue 'draw;
                    }
                }
                _ => {}
            }
        }
    }
}
