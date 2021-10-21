use anyhow::{bail, Result};
use itertools::Itertools;
use std::{
    collections::HashMap,
    ffi::OsString,
    fs::{read_dir, DirEntry},
    path::PathBuf,
};
use tui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
};

#[derive(Default)]
pub struct Views {
    pub index: usize,
    pub contexts: [Context; 4],
}

impl Views {
    pub fn new() -> Result<Self> {
        let mut views = Self::default();
        for context in &mut views.contexts {
            *context = Context::new()?;
        }
        Ok(views)
    }

    pub fn current_context(&mut self) -> &mut Context {
        &mut self.contexts[self.index]
    }
}

enum Target {
    File,
    Directory,
}

impl Target {
    pub fn style(self, highlight: bool) -> Style {
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
pub struct Buffer {
    pub path: PathBuf,
}

#[derive(Debug)]
pub struct State {
    pub cursor: usize,
    pub scroll: usize,
    pub show_hidden: bool,
}

#[derive(Default)]
pub struct Context {
    pub cursor: usize,
    pub scroll: usize,
    pub terminal_size: Rect,
    pub current_dir: PathBuf,
    pub directory: Vec<DirEntry>,
    pub buffers: HashMap<Buffer, State>,
}

impl Context {
    pub fn new() -> Result<Self> {
        let mut context = Self {
            current_dir: std::env::current_dir()?,
            ..Self::default()
        };
        context.read_directory()?;
        Ok(context)
    }

    pub fn current_dir(&self) -> Option<&str> {
        self.current_dir.as_os_str().to_str()
    }

    pub fn height(&self) -> usize {
        (self.terminal_size.height.saturating_sub(3)).into()
    }

    pub fn current_buffer(&self) -> Buffer {
        Buffer {
            path: self.current_dir.clone(),
        }
    }

    pub fn save_buffer(&mut self) {
        let state = State {
            cursor: self.cursor,
            scroll: self.scroll,
            show_hidden: self.show_hidden(),
        };
        self.buffers.insert(self.current_buffer(), state);
    }

    pub fn restore_buffer(&mut self) {
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

    pub fn clamp_cursor(&mut self) -> Result<()> {
        // TODO: Make this target last file/directory instead
        if self.target().is_none() {
            self.scroll = 0;
            let amount = self.walk_directory()?.count().saturating_sub(1);
            self.cursor = amount;
        }
        Ok(())
    }

    pub fn target(&self) -> Option<&DirEntry> {
        match self.walk_directory() {
            Ok(iter) => iter.skip(self.cursor).next(),
            _ => None,
        }
    }

    pub fn target_dir(&self) -> Result<OsString> {
        let target = self.target();
        match target {
            Some(target) if target.path().is_dir() => Ok(target.file_name()),
            _ => bail!("Error occured when trying to get current target"),
        }
    }

    pub fn cursor_up(&mut self, amount: usize) {
        if self.cursor < amount && self.scroll > 0 {
            self.scroll = self.scroll.saturating_sub(10);
            self.cursor += 10;
        } else {
            self.cursor = self.cursor.saturating_sub(amount);
        }
    }

    pub fn cursor_down(&mut self, amount: usize) {
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

    pub fn show_hidden(&self) -> bool {
        match self.buffers.get(&self.current_buffer()) {
            Some(state) => state.show_hidden,
            _ => false,
        }
    }

    pub fn directory_iter(&self) -> Result<impl Iterator<Item = DirEntry>> {
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

    pub fn read_directory(&mut self) -> Result<()> {
        self.directory = self.directory_iter()?.collect();
        Ok(())
    }

    pub fn walk_directory(&self) -> Result<impl Iterator<Item = &DirEntry>> {
        Ok(self
            .directory
            .iter()
            .skip(self.scroll)
            .take(self.height() + 1))
    }

    pub fn listing(&self) -> Result<Text<'_>> {
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
