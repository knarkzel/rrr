use anyhow::Result;
use std::{
    fs::read_dir,
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
    text::Span,
    widgets::{Block, Borders, Paragraph},
    Terminal,
};

struct Context {
    current_dir: PathBuf,
}

impl Context {
    fn new() -> Result<Self> {
        let context = Self {
            current_dir: std::env::current_dir()?,
        };
        Ok(context)
    }

    fn current_dir(&self) -> Option<&str> {
        self.current_dir.as_os_str().to_str()
    }

    fn listing(&self) -> Result<String> {
        let mut output = String::new();
        for path in read_dir(&self.current_dir)? {
            if let Some(input) = path?.file_name().to_str() {
                output.push_str(input);
                output.push('\n');
            }
        }
        Ok(output)
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
            let outline = Block::default().borders(Borders::ALL).title(directory);
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
