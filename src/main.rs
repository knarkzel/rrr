use anyhow::{bail, Result};
use itertools::Itertools;
use std::{
    collections::HashMap,
    ffi::OsString,
    fs::{read_dir, DirEntry, File},
    io::{prelude::*, stdin, stdout},
    path::PathBuf,
};
use termion::{event::Key, input::TermRead, raw::IntoRawMode, screen::AlternateScreen};
use tui::{Terminal, backend::TermionBackend, layout::{Constraint, Layout, Rect}, style::{Color, Modifier, Style}, text::{Span, Spans, Text}, widgets::{Block, Paragraph}};

#[derive(Default)]
struct Views {
    index: usize,
    contexts: [Context; 4],
}

impl Views {
    fn new() -> Result<Self> {
        let mut views = Self::default();
        for context in &mut views.contexts {
            *context = Context::new()?;
        }
        Ok(views)
    }

    fn current_context(&mut self) -> &mut Context {
        &mut self.contexts[self.index]
    }
}

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

#[derive(Debug, Eq, PartialEq, Hash)]
struct Buffer {
    path: PathBuf,
}

#[derive(Debug)]
struct State {
    cursor: usize,
    scroll: usize,
    show_hidden: bool,
}

#[derive(Default)]
struct Context {
    cursor: usize,
    scroll: usize,
    terminal_size: Rect,
    current_dir: PathBuf,
    directory: Vec<DirEntry>,
    buffers: HashMap<Buffer, State>,
}

impl Context {
    fn new() -> Result<Self> {
        let mut context = Self {
            current_dir: std::env::current_dir()?,
            ..Self::default()
        };
        context.read_directory()?;
        Ok(context)
    }

    fn current_dir(&self) -> Option<&str> {
        self.current_dir.as_os_str().to_str()
    }

    fn height(&self) -> usize {
        (self.terminal_size.height.saturating_sub(3)).into()
    }

    fn current_buffer(&self) -> Buffer {
        Buffer {
            path: self.current_dir.clone(),
        }
    }

    fn save_buffer(&mut self) {
        let state = State {
            cursor: self.cursor,
            scroll: self.scroll,
            show_hidden: self.show_hidden(),
        };
        self.buffers.insert(self.current_buffer(), state);
    }

    fn restore_buffer(&mut self) {
        match self.buffers.get(&self.current_buffer()) {
            Some(state) => {
                self.cursor = state.cursor;
                self.scroll = state.scroll;
            }
            _ => {
                self.cursor = 0;
                self.scroll = 0;
            }
        }
    }

    fn clamp_cursor(&mut self) -> Result<()> {
        // TODO: Make this target last file/directory instead
        if self.target().is_none() {
            self.scroll = 0;
            let amount = self.walk_directory()?.count().saturating_sub(1);
            self.cursor = amount;
        }
        Ok(())
    }

    fn target(&self) -> Option<&DirEntry> {
        match self.walk_directory() {
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

    fn cursor_up(&mut self, amount: usize) {
        if self.cursor < amount && self.scroll > 0 {
            self.scroll = self.scroll.saturating_sub(10);
            self.cursor += 10;
        } else {
            self.cursor = self.cursor.saturating_sub(amount);
        }
    }

    fn cursor_down(&mut self, amount: usize) {
        let height = self.height();
        if self.cursor + amount > height {
            self.cursor -= 10;
            self.scroll += 10;
        } else {
            self.cursor += amount;
        }
        let amount = self.walk_directory().unwrap().count().saturating_sub(1);
        if self.cursor > amount {
            self.cursor = amount;
        }
    }

    fn show_hidden(&self) -> bool {
        match self.buffers.get(&self.current_buffer()) {
            Some(state) => state.show_hidden,
            _ => false,
        }
    }

    fn directory_iter(&self) -> Result<impl Iterator<Item = DirEntry>> {
        Ok(read_dir(&self.current_dir)?
            .flatten()
            .filter(|e| {
                !e.path()
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .starts_with(".")
                    || self.show_hidden()
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
            .sorted_by_key(|e| !e.path().is_dir()))
    }

    fn read_directory(&mut self) -> Result<()> {
        self.directory = self.directory_iter()?.collect();
        Ok(())
    }

    fn walk_directory(&self) -> Result<impl Iterator<Item = &DirEntry>> {
        Ok(self
            .directory
            .iter()
            .skip(self.scroll)
            .take(self.height() + 1))
    }

    fn listing(&self) -> Result<Text<'_>> {
        let mut text = Text::default();
        for (line, path) in self.walk_directory()?.enumerate() {
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
    let mut views = Views::new()?;

    'update: loop {
        // Assign current context
        let index = views.index + 1;
        let mut context = views.current_context();
        context.clamp_cursor()?;

        // Assign terminal size for paging
        context.terminal_size = terminal.size()?;

        // Create listing of files
        let listing = context.listing()?;

        terminal.draw(|frame| {
            let size = frame.size();

            let chunks = Layout::default()
                .constraints(
                    [
                        Constraint::Min(1),
                        Constraint::Max(1),
                    ]
                    .as_ref(),
                )
                .split(size);

            // Header
            let mut header = Spans::default();
            let items = &mut header.0;
            for number in 1..=4 {
                if number == index {
                    items.push(Span::styled(
                        number.to_string(),
                        Style::default().add_modifier(Modifier::BOLD),
                    ));
                } else {
                    items.push(Span::raw(number.to_string()));
                }
                items.push(Span::raw(" "));
            }
            let directory = context.current_dir().unwrap_or("Invalid directory");
            items.push(Span::styled(directory, Style::default().fg(Color::Blue)));
            let outline = Block::default().title(header);

            // Files pane
            let files = Paragraph::new(listing).block(outline);

            // Log pane
            let log = Paragraph::new(":quit!");

            // Render
            frame.render_widget(files, chunks[0]);
            frame.render_widget(log, chunks[1]);
        })?;

        for key in stdin().keys() {
            if let Ok(key) = key {
                match key {
                    Key::Char('q') | Key::Ctrl('c') | Key::Ctrl('z') => break 'update,
                    Key::Up | Key::Char('k') => {
                        context.cursor_up(1);
                    }
                    Key::Down | Key::Char('j') => {
                        context.cursor_down(1);
                    }
                    Key::Left | Key::Char('h') => {
                        context.save_buffer();
                        context.current_dir.pop();
                        context.read_directory()?;
                        context.restore_buffer();
                    }
                    Key::Right | Key::Char('l') => {
                        if let Ok(target) = context.target_dir() {
                            context.save_buffer();
                            context.current_dir.push(target);
                            context.read_directory()?;
                            context.restore_buffer();
                        }
                    }
                    Key::Char('.') => {
                        context.save_buffer();
                        let buffer = context.current_buffer();
                        if let Some(state) = context.buffers.get_mut(&buffer) {
                            state.show_hidden = !state.show_hidden;
                            context.read_directory()?;
                            context.restore_buffer();
                        }
                    }
                    Key::Char('e') => {
                        if let Some(target) = context.target() {
                            if edit_this::file(target.path()).is_err() {}
                            terminal.clear()?;
                        }
                    }
                    Key::Ctrl('d') => {
                        context.cursor_down(10);
                    }
                    Key::Ctrl('u') => {
                        context.cursor_up(10);
                    }
                    Key::Char('o') => {
                        if let Some(target) = context.target() {
                            if open::that(target.path()).is_err() {}
                        }
                    }
                    Key::Char(index) if ('1'..='4').any(|digit| digit == index) => {
                        if let Some(index) = index.to_digit(10) {
                            views.index = index.saturating_sub(1) as usize;
                        }
                    }
                    Key::Char('>') => {
                        views.index = (views.index + 1) % 4;
                    }
                    Key::Char('<') => {
                        if views.index > 0 {
                            views.index -= 1;
                        } else {
                            views.index = 3;
                        }
                    }
                    _ => {}
                }
                continue 'update;
            }
        }
    }

    // Fix wonkyness
    terminal.clear()?;

    // Write last entered directory into temporary file
    if let Some(mut path) = dirs::cache_dir() {
        path.push(".rrr");
        let mut file = File::create(path)?;
        write!(
            &mut file,
            "{}",
            views.current_context().current_dir.display()
        )?;
    }

    Ok(())
}
