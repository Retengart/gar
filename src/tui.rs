//! Interactive terminal viewer (optional `--interactive` flag).

use crate::dump::{CHUNK, border_style, status_style, styled_line, title_style};
use crate::lens::Lens;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

const TITLE: &str =
    " base60 — q: quit  hjkl: cursor  Ctrl-d/u: ½ page  g/G: top/bot  ^/$: line ends ";

/// Run the interactive viewer over `data`, offsetting every displayed row
/// by `base_offset` so it matches the position in the original file.
///
/// # Errors
///
/// Propagates any I/O error returned by [`ratatui`] while initializing or
/// rendering the terminal, or by [`crossterm::event::read`] while polling
/// keyboard input.
pub(crate) fn run(data: &[u8], base_offset: u64, lens: Option<&dyn Lens>) -> Result<()> {
    let mut state = ViewState::new(data.len());

    ratatui::run(|terminal| -> Result<()> {
        loop {
            terminal.draw(|frame| state.draw(frame, data, base_offset, lens))?;

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
    data_len: usize,
    total_lines: usize,
    scroll: usize,
    /// Byte offset of the cursor inside the loaded slice. Clamped to
    /// `0..=data_len.saturating_sub(1)`. `None` when the slice is empty.
    cursor: Option<usize>,
    /// Rows inside the bordered body area in the most recent frame.
    /// Used to compute half-page / full-page jumps without re-querying the
    /// terminal on every keypress.
    view_rows: usize,
}

impl ViewState {
    const fn new(data_len: usize) -> Self {
        let total_lines = data_len.div_ceil(CHUNK);
        Self {
            data_len,
            total_lines,
            scroll: 0,
            cursor: if data_len == 0 { None } else { Some(0) },
            view_rows: 1,
        }
    }

    fn draw(
        &mut self,
        frame: &mut ratatui::Frame<'_>,
        data: &[u8],
        base_offset: u64,
        lens: Option<&dyn Lens>,
    ) {
        let [body_area, status_area] =
            Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(frame.area());

        // Subtract the two border rows.
        let rows = usize::from(body_area.height).saturating_sub(2).max(1);
        self.view_rows = rows;
        self.scroll_into_view();

        let visible_end = self.scroll.saturating_add(rows).min(self.total_lines);
        let cursor_row = self.cursor.map(|b| b / CHUNK);
        let cursor_col = self.cursor.map(|b| b % CHUNK);

        let lines: Vec<Line<'_>> = (self.scroll..visible_end)
            .map(|row| {
                let start = row * CHUNK;
                let end = (start + CHUNK).min(data.len());
                let offset = base_offset.saturating_add(start as u64);
                let cursor_here = if cursor_row == Some(row) {
                    cursor_col
                } else {
                    None
                };
                styled_line(offset, &data[start..end], lens, cursor_here)
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

    /// If the cursor drifted off-screen (e.g. via `g`/`G` or page jumps),
    /// pull the viewport back so it's visible again. Called once per frame
    /// so the cursor is always on-screen when any key handler returns.
    fn scroll_into_view(&mut self) {
        let Some(row) = self.cursor.map(|b| b / CHUNK) else {
            return;
        };
        if row < self.scroll {
            self.scroll = row;
        } else if row >= self.scroll + self.view_rows {
            self.scroll = row + 1 - self.view_rows;
        }
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

        let mut spans = vec![
            Span::styled(" lines ", label),
            Span::styled(
                format!("{}-{}", self.scroll + 1, visible_end),
                Style::default(),
            ),
            Span::styled(" / ", dim),
            Span::styled(self.total_lines.to_string(), Style::default()),
            Span::styled("   bytes ", label),
            Span::styled(format!("{start_byte}-{end_byte}"), Style::default()),
        ];

        if let Some(byte) = self.cursor {
            let abs = base_offset.saturating_add(byte as u64);
            spans.push(Span::styled("   cursor 0x", label));
            spans.push(Span::styled(format!("{abs:08x}"), Style::default()));
        }
        spans.push(Span::raw(" "));
        Line::from(spans)
    }

    fn handle_key(&mut self, code: KeyCode, mods: KeyModifiers) -> std::ops::ControlFlow<()> {
        let half = (self.view_rows / 2).max(1);
        let page = self.view_rows.max(1);
        let ctrl = mods.contains(KeyModifiers::CONTROL);

        match code {
            KeyCode::Char('q') | KeyCode::Esc => return std::ops::ControlFlow::Break(()),
            // Byte-granular motion (hjkl / arrow keys). Each moves the
            // cursor; `scroll_into_view` (called on every draw) keeps the
            // viewport tracking it.
            KeyCode::Char('h') | KeyCode::Left => self.cursor_back(1),
            KeyCode::Char('l') | KeyCode::Right => self.cursor_fwd(1),
            KeyCode::Char('j') | KeyCode::Down => self.cursor_fwd(CHUNK),
            KeyCode::Char('k') | KeyCode::Up => self.cursor_back(CHUNK),
            // Line-ends map naturally to start/end of the cursor's row.
            KeyCode::Char('0' | '^') => self.move_cursor_to_line_start(),
            KeyCode::Char('$') => self.move_cursor_to_line_end(),
            KeyCode::Char('d') if ctrl => self.cursor_fwd(half * CHUNK),
            KeyCode::Char('u') if ctrl => self.cursor_back(half * CHUNK),
            KeyCode::PageDown => self.cursor_fwd(page * CHUNK),
            KeyCode::PageUp => self.cursor_back(page * CHUNK),
            KeyCode::Char('g') | KeyCode::Home => self.cursor = self.cursor.map(|_| 0),
            KeyCode::Char('G') | KeyCode::End => {
                self.cursor = self.cursor.map(|_| self.data_len.saturating_sub(1));
            }
            _ => {}
        }
        std::ops::ControlFlow::Continue(())
    }

    const fn cursor_fwd(&mut self, n: usize) {
        if let Some(c) = self.cursor {
            let last = self.data_len.saturating_sub(1);
            self.cursor = Some(byte_min(c.saturating_add(n), last));
        }
    }

    const fn cursor_back(&mut self, n: usize) {
        if let Some(c) = self.cursor {
            self.cursor = Some(c.saturating_sub(n));
        }
    }

    const fn move_cursor_to_line_start(&mut self) {
        if let Some(c) = self.cursor {
            self.cursor = Some((c / CHUNK) * CHUNK);
        }
    }

    const fn move_cursor_to_line_end(&mut self) {
        if let Some(c) = self.cursor {
            let last = self.data_len.saturating_sub(1);
            self.cursor = Some(byte_min(c / CHUNK * CHUNK + CHUNK - 1, last));
        }
    }
}

const fn byte_min(a: usize, b: usize) -> usize {
    if a < b { a } else { b }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_empty_input_has_no_cursor() {
        let s = ViewState::new(0);
        assert_eq!(s.total_lines, 0);
        assert_eq!(s.scroll, 0);
        assert_eq!(s.cursor, None);
    }

    #[test]
    fn new_nonempty_input_starts_cursor_at_zero() {
        let s = ViewState::new(80);
        assert_eq!(s.cursor, Some(0));
    }

    #[test]
    fn hjkl_moves_cursor_not_scroll() {
        let mut s = ViewState::new(8 * 100); // 100 lines
        s.view_rows = 10;

        // Right: +1 byte.
        let _ = s.handle_key(KeyCode::Char('l'), KeyModifiers::NONE);
        assert_eq!(s.cursor, Some(1));

        // Left past zero saturates.
        let _ = s.handle_key(KeyCode::Char('h'), KeyModifiers::NONE);
        let _ = s.handle_key(KeyCode::Char('h'), KeyModifiers::NONE);
        assert_eq!(s.cursor, Some(0));

        // Down: +CHUNK bytes = +1 row.
        let _ = s.handle_key(KeyCode::Char('j'), KeyModifiers::NONE);
        assert_eq!(s.cursor, Some(CHUNK));

        // Up past zero saturates.
        let _ = s.handle_key(KeyCode::Char('k'), KeyModifiers::NONE);
        let _ = s.handle_key(KeyCode::Char('k'), KeyModifiers::NONE);
        assert_eq!(s.cursor, Some(0));
    }

    #[test]
    fn cursor_clamps_to_last_byte_on_g() {
        let mut s = ViewState::new(8 * 100);
        let _ = s.handle_key(KeyCode::Char('G'), KeyModifiers::NONE);
        assert_eq!(s.cursor, Some(8 * 100 - 1));
    }

    #[test]
    fn line_end_jumps_to_last_byte_of_row() {
        let mut s = ViewState::new(8 * 100);
        let _ = s.handle_key(KeyCode::Char('$'), KeyModifiers::NONE);
        assert_eq!(s.cursor, Some(CHUNK - 1));
    }

    #[test]
    fn line_start_returns_to_row_origin() {
        let mut s = ViewState::new(8 * 100);
        // Move cursor to middle of a line then jump to line start.
        s.cursor = Some(CHUNK * 3 + 5);
        let _ = s.handle_key(KeyCode::Char('0'), KeyModifiers::NONE);
        assert_eq!(s.cursor, Some(CHUNK * 3));
    }

    #[test]
    fn cursor_at_last_byte_clamps_instead_of_overflowing() {
        let mut s = ViewState::new(8 * 2); // 2 rows.
        // Force cursor to last valid byte then try to move past.
        s.cursor = Some(15);
        let _ = s.handle_key(KeyCode::Char('l'), KeyModifiers::NONE);
        assert_eq!(s.cursor, Some(15));
    }

    #[test]
    fn ctrl_d_moves_cursor_by_half_page_in_bytes() {
        let mut s = ViewState::new(8 * 100);
        s.view_rows = 10;
        // Half page = 5 rows = 40 bytes.
        let _ = s.handle_key(KeyCode::Char('d'), KeyModifiers::CONTROL);
        assert_eq!(s.cursor, Some(40));
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
        // `d` without Ctrl is unbound; cursor must not budge.
        let _ = s.handle_key(KeyCode::Char('d'), KeyModifiers::NONE);
        assert_eq!(s.cursor, Some(0));
    }

    #[test]
    fn scroll_into_view_pulls_viewport_when_cursor_drops_below() {
        let mut s = ViewState::new(8 * 100);
        s.view_rows = 10;
        s.cursor = Some(8 * 50);
        s.scroll_into_view();
        // Cursor sits at row 50; with viewport 10 rows, scroll must be
        // exactly 41 (so row 50 is the 10th visible row).
        assert_eq!(s.scroll, 41);
    }

    #[test]
    fn scroll_into_view_pulls_viewport_when_cursor_jumps_above() {
        let mut s = ViewState::new(8 * 100);
        s.view_rows = 10;
        s.scroll = 40;
        s.cursor = Some(8 * 5); // row 5 — above the viewport.
        s.scroll_into_view();
        assert_eq!(s.scroll, 5);
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
    fn status_line_populated_mentions_cursor_offset() {
        let mut s = ViewState::new(8 * 100);
        s.scroll = 5;
        s.cursor = Some(42);
        let line = s.status_line(0x100, 20);
        let joined: String = line
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect();
        assert!(joined.contains(" lines 6-20 / 100"));
        assert!(joined.contains("bytes"));
        assert!(joined.contains("cursor 0x0000012a"));
    }
}
