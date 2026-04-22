use crate::dump::{CHUNK, format_line};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};

/// Interactive viewer over `data`, with `base_offset` added to every line's
/// displayed offset so it matches byte position in the original file.
pub fn run(data: &[u8], base_offset: u64) -> Result<()> {
    let total_lines = data.len().div_ceil(CHUNK);
    let mut scroll: usize = 0;

    ratatui::run(|terminal| -> Result<()> {
        loop {
            terminal.draw(|frame| {
                let area = frame.area();
                let layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(1), Constraint::Length(1)])
                    .split(area);

                let view_rows = layout[0].height.saturating_sub(2) as usize; // borders
                let visible_end = (scroll + view_rows).min(total_lines);

                let lines: Vec<Line> = (scroll..visible_end)
                    .map(|row| {
                        let start = row * CHUNK;
                        let end = (start + CHUNK).min(data.len());
                        let offset = base_offset + start as u64;
                        Line::from(format_line(offset, &data[start..end]))
                    })
                    .collect();

                let body = Paragraph::new(lines).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" base60 — q: quit  j/k: line  Ctrl-d/u: half page  g/G: top/bot "),
                );
                frame.render_widget(body, layout[0]);

                let status = format!(
                    " lines {}-{} / {}   bytes {}-{} ",
                    scroll + 1,
                    visible_end,
                    total_lines,
                    base_offset + (scroll * CHUNK) as u64,
                    base_offset + (visible_end * CHUNK) as u64
                );
                frame.render_widget(
                    Paragraph::new(status)
                        .style(Style::default().add_modifier(Modifier::REVERSED)),
                    layout[1],
                );
            })?;

            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                let page = (terminal.size()?.height as usize).saturating_sub(3);
                let half = page / 2;
                let max_scroll = total_lines.saturating_sub(1);
                match (key.code, key.modifiers) {
                    (KeyCode::Char('q'), _) | (KeyCode::Esc, _) => break Ok(()),
                    (KeyCode::Char('j'), _) | (KeyCode::Down, _) => {
                        scroll = (scroll + 1).min(max_scroll);
                    }
                    (KeyCode::Char('k'), _) | (KeyCode::Up, _) => {
                        scroll = scroll.saturating_sub(1);
                    }
                    (KeyCode::Char('d'), KeyModifiers::CONTROL)
                    | (KeyCode::PageDown, _) => {
                        scroll = (scroll + half.max(1)).min(max_scroll);
                    }
                    (KeyCode::Char('u'), KeyModifiers::CONTROL) | (KeyCode::PageUp, _) => {
                        scroll = scroll.saturating_sub(half.max(1));
                    }
                    (KeyCode::Char('g'), _) | (KeyCode::Home, _) => scroll = 0,
                    (KeyCode::Char('G'), _) | (KeyCode::End, _) => scroll = max_scroll,
                    _ => {}
                }
            }
        }
    })?;
    Ok(())
}
