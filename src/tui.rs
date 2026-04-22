//! Interactive terminal viewer (optional `--interactive` flag).

use crate::dump::{CHUNK, border_style, status_style, styled_line, title_style};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

const TITLE: &str = " base60 — q: quit  j/k: line  Ctrl-d/u: half page  g/G: top/bot ";

/// Run the interactive viewer over `data`, offsetting every displayed row
/// by `base_offset` so it matches the position in the original file.
///
/// # Errors
///
/// Propagates any I/O error returned by [`ratatui`] while initializing or
/// rendering the terminal, or by [`crossterm::event::read`] while polling
/// keyboard input.
pub(crate) fn run(data: &[u8], base_offset: u64) -> Result<()> {
    let mut state = ViewState::new(data.len());

    ratatui::run(|terminal| -> Result<()> {
        loop {
            terminal.draw(|frame| state.draw(frame, data, base_offset))?;

            let Event::Key(key) = event::read()? else {
                continue;
            };
            if key.kind != KeyEventKind::Press {
                continue;
            }
            if state.handle_key(key.code, key.modifiers).is_break() {
                break Ok(());
            }
        }
    })
}

/// Scroll state + derived layout sizes shared between `draw` and
/// `handle_key`.
struct ViewState {
    total_lines: usize,
    max_scroll: usize,
    scroll: usize,
    /// Rows inside the bordered body area in the most recent frame.
    /// Used to compute half-page / full-page jumps without re-querying the
    /// terminal on every keypress.
    view_rows: usize,
}

impl ViewState {
    const fn new(data_len: usize) -> Self {
        let total_lines = data_len.div_ceil(CHUNK);
        Self {
            total_lines,
            max_scroll: total_lines.saturating_sub(1),
            scroll: 0,
            view_rows: 1,
        }
    }

    fn draw(&mut self, frame: &mut ratatui::Frame<'_>, data: &[u8], base_offset: u64) {
        let [body_area, status_area] =
            Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(frame.area());

        // Subtract the two border rows.
        let rows = usize::from(body_area.height).saturating_sub(2).max(1);
        self.view_rows = rows;

        let visible_end = self.scroll.saturating_add(rows).min(self.total_lines);

        let lines: Vec<Line<'_>> = (self.scroll..visible_end)
            .map(|row| {
                let start = row * CHUNK;
                let end = (start + CHUNK).min(data.len());
                let offset = base_offset.saturating_add(start as u64);
                styled_line(offset, &data[start..end])
            })
            .collect();

        let title = Line::from(Span::styled(TITLE, title_style()));
        let body = Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style())
                .title(title),
        );
        frame.render_widget(body, body_area);

        let status_line = self.status_line(base_offset, visible_end);
        frame.render_widget(
            Paragraph::new(status_line).style(Style::default()),
            status_area,
        );
    }

    /// Build the bottom status bar as styled spans: pale fixed labels + a
    /// highlight for the live numbers so the eye can track them as the
    /// viewport moves.
    fn status_line(&self, base_offset: u64, visible_end: usize) -> Line<'static> {
        let label = status_style();
        let dim = Style::default();
        if self.total_lines == 0 {
            return Line::from(Span::styled(" empty input ", label));
        }

        let start_byte = base_offset.saturating_add((self.scroll * CHUNK) as u64);
        let end_byte = base_offset.saturating_add((visible_end * CHUNK) as u64);

        Line::from(vec![
            Span::styled(" lines ", label),
            Span::styled(
                format!("{}-{}", self.scroll + 1, visible_end),
                Style::default(),
            ),
            Span::styled(" / ", dim),
            Span::styled(self.total_lines.to_string(), Style::default()),
            Span::styled("   bytes ", label),
            Span::styled(format!("{start_byte}-{end_byte}"), Style::default()),
            Span::raw(" "),
        ])
    }

    fn handle_key(&mut self, code: KeyCode, mods: KeyModifiers) -> std::ops::ControlFlow<()> {
        let half = (self.view_rows / 2).max(1);
        let page = self.view_rows.max(1);
        let ctrl = mods.contains(KeyModifiers::CONTROL);

        match code {
            KeyCode::Char('q') | KeyCode::Esc => return std::ops::ControlFlow::Break(()),
            KeyCode::Char('j') | KeyCode::Down => {
                self.scroll = self.scroll.saturating_add(1).min(self.max_scroll);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.scroll = self.scroll.saturating_sub(1);
            }
            KeyCode::Char('d') if ctrl => {
                self.scroll = self.scroll.saturating_add(half).min(self.max_scroll);
            }
            KeyCode::Char('u') if ctrl => {
                self.scroll = self.scroll.saturating_sub(half);
            }
            KeyCode::PageDown => {
                self.scroll = self.scroll.saturating_add(page).min(self.max_scroll);
            }
            KeyCode::PageUp => {
                self.scroll = self.scroll.saturating_sub(page);
            }
            KeyCode::Char('g') | KeyCode::Home => self.scroll = 0,
            KeyCode::Char('G') | KeyCode::End => self.scroll = self.max_scroll,
            _ => {}
        }
        std::ops::ControlFlow::Continue(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_empty_input_is_safe() {
        let s = ViewState::new(0);
        assert_eq!(s.total_lines, 0);
        assert_eq!(s.max_scroll, 0);
        assert_eq!(s.scroll, 0);
    }

    #[test]
    fn handle_key_navigates_within_bounds() {
        let mut s = ViewState::new(8 * 100); // 100 lines
        s.view_rows = 10;

        // Down once.
        let _ = s.handle_key(KeyCode::Char('j'), KeyModifiers::NONE);
        assert_eq!(s.scroll, 1);

        // Up past zero saturates.
        let _ = s.handle_key(KeyCode::Char('k'), KeyModifiers::NONE);
        let _ = s.handle_key(KeyCode::Char('k'), KeyModifiers::NONE);
        assert_eq!(s.scroll, 0);

        // G jumps to last line.
        let _ = s.handle_key(KeyCode::Char('G'), KeyModifiers::NONE);
        assert_eq!(s.scroll, 99);

        // Down clamps to max.
        let _ = s.handle_key(KeyCode::Char('j'), KeyModifiers::NONE);
        assert_eq!(s.scroll, 99);

        // Ctrl+u half-page up.
        let _ = s.handle_key(KeyCode::Char('u'), KeyModifiers::CONTROL);
        assert_eq!(s.scroll, 99 - 5);
    }

    #[test]
    fn quit_returns_break() {
        let mut s = ViewState::new(80);
        let flow = s.handle_key(KeyCode::Char('q'), KeyModifiers::NONE);
        assert!(flow.is_break());
    }

    #[test]
    fn non_ctrl_d_does_not_scroll_half_page() {
        let mut s = ViewState::new(8 * 100);
        s.view_rows = 20;
        let _ = s.handle_key(KeyCode::Char('d'), KeyModifiers::NONE);
        assert_eq!(s.scroll, 0);
    }

    #[test]
    fn status_line_empty_input() {
        let s = ViewState::new(0);
        let line = s.status_line(0, 0);
        let joined: String = line
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect();
        assert_eq!(joined, " empty input ");
    }

    #[test]
    fn status_line_populated_mentions_counts() {
        let mut s = ViewState::new(8 * 100);
        s.scroll = 5;
        let line = s.status_line(0x100, 20);
        let joined: String = line
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect();
        assert!(joined.contains(" lines 6-20 / 100"));
        assert!(joined.contains("bytes"));
    }
}
