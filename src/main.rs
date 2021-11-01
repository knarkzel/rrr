use rrr::{state::Mode, *};
use std::{
    fs::File,
    io::{prelude::*, stdin, stdout},
};
use termion::{event::Key, input::TermRead, raw::IntoRawMode, screen::AlternateScreen};
use tui::{
    backend::TermionBackend,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Paragraph},
    Terminal,
};

#[throws]
fn main() {
    let stdout = stdout().into_raw_mode()?;
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut views = state::Views::new()?;

    'update: loop {
        // Assign current context, immutable moves here
        let mode = views.mode;
        let index = views.index + 1;
        let command = if views.mode == Mode::Command {
            format!(":{}", views.command)
        } else {
            String::new()
        };

        // Mutable borrows start here
        let mut context = views.current_context();
        context.clamp_cursor()?;

        // Assign terminal size for paging
        context.terminal_size = terminal.size()?;

        // Create listing of files
        let listing = context.listing()?;

        terminal.draw(|frame| {
            let size = frame.size();

            let chunks = Layout::default()
                .constraints([Constraint::Min(1), Constraint::Max(1)].as_ref())
                .split(size);

            // Header and files pane
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
            let files = Paragraph::new(listing).block(outline);

            // Command pane
            let command = Paragraph::new(command);

            // Render
            frame.render_widget(files, chunks[0]);
            frame.render_widget(command, chunks[1]);
        })?;

        for key in stdin().keys() {
            if let Ok(key) = key {
                match mode {
                    Mode::Normal => match key {
                        Key::Char('q') | Key::Ctrl('c') | Key::Ctrl('z') => break 'update,
                        Key::Up | Key::Char('k') => {
                            context.cursor_up(1);
                        }
                        Key::Down | Key::Char('j') => {
                            context.cursor_down(1);
                        }
                        Key::Left | Key::Char('h') => {
                            context.save_buffer();
                            let backup = context.current_dir.clone();
                            context.current_dir.pop();
                            if context.read_directory().is_err() {
                                context.current_dir = backup;
                            }
                            context.restore_buffer();
                        }
                        Key::Right | Key::Char('l') => {
                            if let Ok(target) = context.target_dir() {
                                context.save_buffer();
                                let backup = context.current_dir.clone();
                                context.current_dir.push(target);
                                if context.read_directory().is_err() {
                                    context.current_dir = backup;
                                }
                                context.restore_buffer();
                            }
                        }
                        Key::Char('.') => {
                            context.save_buffer();
                            let buffer = context.current_dir.clone();
                            if let Some(state) = context.buffers.get_mut(&buffer) {
                                state.show_hidden = !state.show_hidden;
                                context.read_directory()?;
                                context.restore_buffer();
                            }
                        }
                        Key::Char('e') => {
                            if let Some(target) = context.target() {
                                if edit::file(&target.path()).is_err() {}
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
                                if open::that(&target.path()).is_err() {}
                            }
                        }
                        Key::Char(index) if ('1'..='4').any(|digit| digit == index) => {
                            if let Some(index) = index.to_digit(10) {
                                views.index(index.saturating_sub(1) as usize)?;
                            }
                        }
                        Key::Char('>') => {
                            views.index = (views.index + 1) % 4;
                        }
                        Key::Char('<') => {
                            match views.index > 0 {
                                true => views.index -= 1,
                                false => views.index = 3,
                            }
                        }
                        Key::Char(':') => views.mode = Mode::Command,
                        Key::Char(' ') => {
                            if let Some(entry) = context.target() {
                                let path = entry.path();
                                if let Some(buffer) = context.buffer_mut() {
                                    buffer.flip(path);
                                    context.cursor_down(1);
                                }
                            }
                        }
                        _ => {}
                    },
                    Mode::Command => match key {
                        Key::Char('\n') => {
                            views.execute_command()?;
                            views.exit_command();
                            terminal.clear()?;
                        },
                        Key::Esc => views.exit_command(),
                        Key::Ctrl('u') => views.command = String::new(),
                        Key::Char(c) => views.command.push(c),
                        Key::Backspace => {
                            views.command.pop();
                        }
                        _ => {}
                    },
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
}
