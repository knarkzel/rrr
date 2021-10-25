use tui::style::{Color, Modifier, Style};

pub fn file(highlight: bool) -> Style {
    match highlight {
        true => Style::default().fg(Color::Black).bg(Color::White),
        false => Style::default().fg(Color::White),
    }
}

pub fn directory(highlight: bool) -> Style {
    match highlight {
        true => Style::default().fg(Color::Black).bg(Color::Blue),
        false => Style::default().fg(Color::Blue),
    }
    .add_modifier(Modifier::BOLD)
}

pub fn reset() -> Style {
    Style::default().fg(Color::Reset)
}
