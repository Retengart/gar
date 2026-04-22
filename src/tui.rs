use crate::dump::{CHUNK, format_line};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};

/// Interactive viewer over `data`, with `base_offset` added to every line's
/// displayed offset so it matches byte position in the original file.
pub fn run(data: &[u8], base_offset: u64) -> Result<()> {
    let total_lines = data.len().div_ceil(CHUNK);
    let max_scroll = total_lines.saturating_sub(1);
    let mut scroll: usize = 0;
    let mut last_view_rows: usize = 1;

    ratatui::run(|terminal| -> Result<()> {
        loop {
            terminal.draw(|frame| {
                let [body_area, status_area] =
                    Layout::vertical([Constraint::Min(1), Constraint::Length(1)])
                        .areas(frame.area());

                // Subtract the two border rows of the bordered Block.
                let view_rows = (body_area.height as usize).saturating_sub(2).max(1);
                last_view_rows = view_rows;

                let visible_end = scroll.saturating_add(view_rows).min(total_lines);

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
                frame.render_widget(body, body_area);

                let status = if total_lines == 0 {
                    " empty input ".to_string()
                } else {
                    format!(
                        " lines {}-{} / {}   bytes {}-{} ",
                        scroll + 1,
                        visible_end,
                        total_lines,
                        base_offset + (scroll * CHUNK) as u64,
                        base_offset + (visible_end * CHUNK) as u64,
                    )
                };
                frame.render_widget(
                    Paragraph::new(status)
                        .style(Style::default().add_modifier(Modifier::REVERSED)),
                    status_area,
                );
            })?;

            let Event::Key(key) = event::read()? else {
                continue;
            };
            if key.kind != KeyEventKind::Press {
                continue;
            }
            let half = (last_view_rows / 2).max(1);
            let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => break Ok(()),
                KeyCode::Char('j') | KeyCode::Down => {
                    scroll = (scroll + 1).min(max_scroll);
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    scroll = scroll.saturating_sub(1);
                }
                KeyCode::Char('d') if ctrl => {
                    scroll = scroll.saturating_add(half).min(max_scroll);
                }
                KeyCode::PageDown => {
                    scroll = scroll.saturating_add(last_view_rows).min(max_scroll);
                }
                KeyCode::Char('u') if ctrl => {
                    scroll = scroll.saturating_sub(half);
                }
                KeyCode::PageUp => {
                    scroll = scroll.saturating_sub(last_view_rows);
                }
                KeyCode::Char('g') | KeyCode::Home => scroll = 0,
                KeyCode::Char('G') | KeyCode::End => scroll = max_scroll,
                _ => {}
            }
        }
    })?;
    Ok(())
}
