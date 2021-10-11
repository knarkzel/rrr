use anyhow::Result;
use std::{
    fmt::Write,
    fs::read_dir,
    io::{stdin, stdout},
    path::PathBuf,
};
use termion::{
    color,
    event::Key,
    input::{MouseTerminal, TermRead},
    raw::IntoRawMode,
    screen::AlternateScreen,
};
use tui::{
    backend::TermionBackend,
    style::{Color, Style},
    text::{Span, Spans, Text},
    widgets::{Block, BorderType, Borders, Paragraph},
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

    fn current_dir(&self) -> Option<&str> {
        self.current_dir.as_os_str().to_str()
    }

    fn listing(&self) -> Result<Text<'_>> {
        let mut text = Text::default();
        for (line, path) in read_dir(&self.current_dir)?.enumerate() {
            let path = path?;
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
    let context = Context::new()?;
    let stdout = stdout().into_raw_mode()?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    loop {
        let listing = context.listing()?;
        terminal.draw(|frame| {
            let size = frame.size();

            // Files pane
            let directory = Span::from(context.current_dir().unwrap_or("Invalid directory"));
            let outline = Block::default()
                .borders(Borders::ALL)
                .title(directory);
            let files = Paragraph::new(listing).block(outline);
            frame.render_widget(files, size);
        })?;

        for key in stdin().keys() {
            match key? {
                Key::Char('q') => {
                    std::env::set_current_dir(context.current_dir)?;
                    return Ok(());
                }
                _ => {}
            }
        }
    }
}
