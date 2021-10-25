use crate::{target::Target, utils::entry_not_hidden, *};
use itertools::Itertools;
use std::{
    collections::HashMap,
    ffi::OsString,
    fs::{read_dir, DirEntry},
    path::PathBuf,
};
use tui::{
    layout::Rect,
    style::{Color, Style},
    text::{Span, Spans, Text},
};

#[derive(Default)]
pub struct Views {
    pub index: usize,
    pub contexts: [Context; 4],
}

impl Views {
    #[throws]
    pub fn new() -> Self {
        let mut views = Self::default();
        for context in &mut views.contexts {
            *context = Context::new()?;
        }
        views
    }

    pub fn current_context(&mut self) -> &mut Context {
        &mut self.contexts[self.index]
    }
}

#[derive(Debug)]
pub struct Buffer {
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
    pub buffers: HashMap<PathBuf, Buffer>,
}

impl Context {
    #[throws]
    pub fn new() -> Self {
        let mut context = Self {
            current_dir: std::env::current_dir()?,
            ..Self::default()
        };
        context.read_directory()?;
        context
    }

    pub fn current_dir(&self) -> Option<&str> {
        self.current_dir.as_os_str().to_str()
    }

    pub fn height(&self) -> usize {
        (self.terminal_size.height.saturating_sub(3)).into()
    }

    pub fn save_buffer(&mut self) {
        let state = Buffer {
            cursor: self.cursor,
            scroll: self.scroll,
            show_hidden: self.show_hidden(),
        };
        self.buffers.insert(self.current_dir.clone(), state);
    }

    pub fn restore_buffer(&mut self) {
        match self.buffers.get(&self.current_dir) {
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

    #[throws]
    pub fn clamp_cursor(&mut self) {
        if self.target().is_none() {
            self.scroll = 0;
            let amount = self.walk_directory()?.count().saturating_sub(1);
            self.cursor = amount;
        }
    }

    pub fn target(&self) -> Option<&DirEntry> {
        match self.walk_directory() {
            Ok(iter) => iter.skip(self.cursor).next(),
            _ => None,
        }
    }

    #[throws]
    pub fn target_dir(&self) -> OsString {
        let target = self.target();
        match target {
            Some(target) if target.path().is_dir() => target.file_name(),
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
        self.buffers
            .get(&self.current_dir)
            .map(|state| state.show_hidden)
            .unwrap_or(false)
    }

    #[throws]
    pub fn directory_iter(&self) -> impl Iterator<Item = DirEntry> {
        read_dir(&self.current_dir)?
            .flatten()
            .filter(|entry| entry_not_hidden(entry) || self.show_hidden())
            .sorted_unstable_by(|first, second| first.file_name().cmp(&second.file_name()))
            .sorted_unstable_by_key(entry_not_hidden)
            .sorted_unstable_by_key(|entry| !entry.path().is_dir())
    }

    #[throws]
    pub fn read_directory(&mut self) {
        self.directory = self.directory_iter()?.collect();
    }

    #[throws]
    pub fn walk_directory(&self) -> impl Iterator<Item = &DirEntry> {
        self.directory
            .iter()
            .skip(self.scroll)
            .take(self.height() + 1)
    }

    #[throws]
    pub fn listing(&self) -> Text<'_> {
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
        text
    }
}
