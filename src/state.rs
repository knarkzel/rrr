use crate::*;
use jwalk::WalkDir;
use std::{collections::HashMap, path::PathBuf};
use tui::{
    layout::Rect,
    text::{Span, Spans, Text},
};

pub struct Entry {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
}

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
    pub directory: Vec<Entry>,
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
            let amount = self.view()?.count().saturating_sub(1);
            self.cursor = amount;
        }
    }

    pub fn target(&self) -> Option<&Entry> {
        match self.view() {
            Ok(iter) => iter.skip(self.cursor).next(),
            _ => None,
        }
    }

    #[throws]
    pub fn target_dir(&self) -> String {
        let target = self.target();
        match target {
            Some(target) if target.is_dir => target.name.clone(),
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
        let amount = self.view().unwrap().count().saturating_sub(1);
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

    // #[throws]
    // pub fn read(&self) -> impl Iterator<Item = DirEntry> {
    //     read_dir(&self.current_dir)?
    //         .flatten()
    //         .filter(|entry| entry_not_hidden(entry) || self.show_hidden())
    //         .sorted_unstable_by(|first, second| first.file_name().cmp(&second.file_name()))
    //         .sorted_unstable_by_key(entry_not_hidden)
    //         .sorted_unstable_by_key(|entry| !entry.path().is_dir())
    // }

    #[throws]
    pub fn read(&self) -> Vec<Entry> {
        let mut entries = Vec::with_capacity(1000);
        for entry in WalkDir::new(&self.current_dir)
            .max_depth(1)
            .skip_hidden(self.show_hidden())
            .sort(true)
            .into_iter()
            .skip(1)
        {
            let entry = entry?;
            let path = entry.path();
            let is_dir = entry.path().is_dir();
            let name = entry.file_name().to_str().unwrap().to_string();
            entries.push(Entry { path, name, is_dir });
        }
        entries
    }

    #[throws]
    pub fn view(&self) -> impl Iterator<Item = &Entry> {
        self.directory
            .iter()
            .skip(self.scroll)
            .take(self.height() + 1)
    }

    #[throws]
    pub fn read_directory(&mut self) {
        self.directory = self.read()?;
    }

    #[throws]
    pub fn listing(&self) -> Text {
        let mut text = Text::default();
        for (line, entry) in self.view()?.enumerate() {
            let highlight = self.cursor == line;
            let mut spans = Spans::default();
            let items = &mut spans.0;
            if entry.is_dir {
                items.push(Span::styled(&entry.name, style::directory(highlight)));
                items.push(Span::styled("/", style::reset()));
            } else {
                items.push(Span::styled(&entry.name, style::file(highlight)));
            }
            text.lines.push(spans);
        }
        text
    }
}
