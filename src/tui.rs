//! Interactive terminal viewer (optional `--interactive` flag).

use crate::cli::{LensMode, TimeScale, build_lens};
use crate::cuneiform;
use crate::dump::{CHUNK, border_style, status_style, styled_line, title_style};
use crate::lens::Lens;
use crate::search::{self, Pattern};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

const TITLE: &str =
    " 𒁹 base60 — hjkl: cursor  L: lens  /: search  n/N: match  g/G: top/bot  q: quit 𒌋 ";

/// Modal state of the viewer. `Normal` is the cursor-driven default; the
/// other variants trap almost every keypress so the user can compose an
/// input without accidentally scrolling.
enum Mode {
    Normal,
    /// User is typing a search query into the status bar.
    SearchInput(String),
}

/// Run the interactive viewer over `data`, offsetting every displayed row
/// by `base_offset` so it matches the position in the original file.
///
/// `initial_mode`, `scale`, and `purist` seed the lens state; `L` cycles
/// through the five [`LensMode`] variants at runtime.
///
/// # Errors
///
/// Propagates any I/O error returned by [`ratatui`] while initializing or
/// rendering the terminal, or by [`crossterm::event::read`] while polling
/// keyboard input.
pub(crate) fn run(
    data: &[u8],
    base_offset: u64,
    initial_mode: LensMode,
    scale: TimeScale,
    purist: bool,
) -> Result<()> {
    let mut state = ViewState::new(data.len(), initial_mode, scale, purist);

    ratatui::run(|terminal| -> Result<()> {
        loop {
            terminal.draw(|frame| state.draw(frame, data, base_offset))?;

            let Event::Key(key) = event::read()? else {
                continue;
            };
            if key.kind != KeyEventKind::Press {
                continue;
            }
            if state.handle_key(key.code, key.modifiers, data).is_break() {
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
    /// Currently-active lens variant; cycled by the `L` key.
    lens_mode: LensMode,
    /// Pre-built trait object matching `lens_mode`. Rebuilt when the
    /// mode changes so the render hot path stays indirection-free.
    lens: Option<Box<dyn Lens>>,
    /// Frozen at construction — `--time-scale` and `--purist` carry over
    /// when the user cycles into [`LensMode::Time`] or [`LensMode::Tablet`].
    scale: TimeScale,
    purist: bool,
    /// Modal state driving keyboard dispatch.
    mode: Mode,
    /// Byte offsets of every hit from the last confirmed search, in
    /// ascending order. Empty when no search has run or the query
    /// produced no matches.
    matches: Vec<usize>,
    /// Index into `matches` — the one the cursor currently sits on.
    /// `usize::MAX` means "no match selected yet".
    match_idx: usize,
    /// Last known status message — set after a search confirms or errors,
    /// shown in the status bar for one render then cleared.
    status_message: Option<String>,
}

impl ViewState {
    fn new(data_len: usize, initial_mode: LensMode, scale: TimeScale, purist: bool) -> Self {
        let total_lines = data_len.div_ceil(CHUNK);
        Self {
            data_len,
            total_lines,
            scroll: 0,
            cursor: if data_len == 0 { None } else { Some(0) },
            view_rows: 1,
            lens_mode: initial_mode,
            lens: build_lens(initial_mode, scale, purist),
            scale,
            purist,
            mode: Mode::Normal,
            matches: Vec::new(),
            match_idx: usize::MAX,
            status_message: None,
        }
    }

    fn draw(&mut self, frame: &mut ratatui::Frame<'_>, data: &[u8], base_offset: u64) {
        let [body_area, status_area] =
            Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(frame.area());

        // Subtract the two border rows.
        let rows = usize::from(body_area.height).saturating_sub(2).max(1);
        self.view_rows = rows;
        self.scroll_into_view();

        let visible_end = self.scroll.saturating_add(rows).min(self.total_lines);
        let cursor_row = self.cursor.map(|b| b / CHUNK);
        let cursor_col = self.cursor.map(|b| b % CHUNK);
        let lens_ref: Option<&dyn Lens> = self.lens.as_deref();

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
                styled_line(offset, &data[start..end], lens_ref, cursor_here)
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

        // Modal inputs and transient messages take over the status line
        // entirely — there's no room to mix them with the live stats
        // without making the whole bar unreadable on narrow terminals.
        if let Mode::SearchInput(buf) = &self.mode {
            return Line::from(vec![
                Span::styled(" search: ", label),
                Span::styled(buf.clone(), Style::default()),
                // Trailing cursor block so the user sees where typing lands.
                Span::styled("_", Style::default()),
                Span::raw(" "),
            ]);
        }
        if let Some(msg) = &self.status_message {
            return Line::from(vec![
                Span::styled(" ", label),
                Span::styled(msg.clone(), Style::default()),
                Span::raw(" "),
            ]);
        }

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
            // When the cuneiform lens is active, also render the cursor
            // offset in Sumero-Babylonian wedges — the tool is on-theme
            // with itself.
            if self.lens_mode == LensMode::Cuneiform {
                spans.push(Span::styled(" ", dim));
                spans.push(Span::styled(cuneiform_offset(abs), Style::default()));
            }
        }

        spans.push(Span::styled("   lens ", label));
        spans.push(Span::styled(self.lens_mode.label(), Style::default()));
        spans.push(Span::raw(" "));
        Line::from(spans)
    }

    fn handle_key(
        &mut self,
        code: KeyCode,
        mods: KeyModifiers,
        data: &[u8],
    ) -> std::ops::ControlFlow<()> {
        // Transient status messages survive exactly one frame — clear on
        // the next keypress so the bar returns to showing live stats.
        self.status_message = None;

        // Modal inputs are handled first so accelerators like `q` don't
        // quit the TUI while the user is typing a search query.
        if let Mode::SearchInput(_) = &self.mode {
            return self.handle_search_input_key(code, data);
        }

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
            // Capital L cycles through the five lens modes: None → Time →
            // Angle → Tablet → Cuneiform → None. Lower-case `l` already
            // moves the cursor, hence the `Shift+l` choice.
            KeyCode::Char('L') => self.cycle_lens(),
            // Search: `/` opens the modal input; `n`/`N` navigate between
            // matches from the last confirmed query.
            KeyCode::Char('/') => self.mode = Mode::SearchInput(String::new()),
            KeyCode::Char('n') => self.jump_to_next_match(),
            KeyCode::Char('N') => self.jump_to_prev_match(),
            _ => {}
        }
        std::ops::ControlFlow::Continue(())
    }

    fn handle_search_input_key(&mut self, code: KeyCode, data: &[u8]) -> std::ops::ControlFlow<()> {
        match code {
            KeyCode::Esc => {
                // Cancel the search mid-input, leaving previous matches
                // intact so the user can keep using `n`/`N`.
                self.mode = Mode::Normal;
            }
            KeyCode::Enter => self.confirm_search(data),
            KeyCode::Backspace => {
                if let Mode::SearchInput(buf) = &mut self.mode {
                    buf.pop();
                }
            }
            KeyCode::Char(c) => {
                if let Mode::SearchInput(buf) = &mut self.mode {
                    buf.push(c);
                }
            }
            _ => {}
        }
        std::ops::ControlFlow::Continue(())
    }

    fn confirm_search(&mut self, data: &[u8]) {
        let Mode::SearchInput(buf) = std::mem::replace(&mut self.mode, Mode::Normal) else {
            return;
        };
        match buf.parse::<Pattern>() {
            Ok(Pattern(bytes)) => {
                self.matches = search::find_all(data, &bytes);
                self.match_idx = usize::MAX;
                if let Some(&first) = self.matches.first() {
                    self.cursor = Some(first);
                    self.match_idx = 0;
                    self.status_message =
                        Some(format!("match 1/{} at 0x{first:08x}", self.matches.len()));
                } else {
                    self.status_message = Some(format!("no match for {buf:?}"));
                }
            }
            Err(_) => {
                self.status_message = Some(format!("bad pattern: {buf:?}"));
            }
        }
    }

    fn jump_to_next_match(&mut self) {
        if self.matches.is_empty() {
            self.status_message = Some("no active search".to_owned());
            return;
        }
        let next = if self.match_idx == usize::MAX {
            0
        } else {
            (self.match_idx + 1) % self.matches.len()
        };
        self.match_idx = next;
        let byte = self.matches[next];
        self.cursor = Some(byte);
        self.status_message = Some(format!(
            "match {}/{} at 0x{byte:08x}",
            next + 1,
            self.matches.len()
        ));
    }

    fn jump_to_prev_match(&mut self) {
        if self.matches.is_empty() {
            self.status_message = Some("no active search".to_owned());
            return;
        }
        let prev = if self.match_idx == usize::MAX || self.match_idx == 0 {
            self.matches.len() - 1
        } else {
            self.match_idx - 1
        };
        self.match_idx = prev;
        let byte = self.matches[prev];
        self.cursor = Some(byte);
        self.status_message = Some(format!(
            "match {}/{} at 0x{byte:08x}",
            prev + 1,
            self.matches.len()
        ));
    }

    fn cycle_lens(&mut self) {
        self.lens_mode = self.lens_mode.cycle();
        self.lens = build_lens(self.lens_mode, self.scale, self.purist);
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

/// Render `offset` (a byte address) as Sumero-Babylonian cuneiform,
/// stripping leading zero-placeholders so the display is compact.
///
/// A u64 takes up to 11 base-60 digits, but offsets are usually small;
/// skipping leading zeroes keeps the status line readable.
fn cuneiform_offset(offset: u64) -> String {
    let digits = crate::convert::u64_to_base60(offset);
    let start = digits
        .iter()
        .position(|&d| d != 0)
        .unwrap_or(digits.len() - 1);
    let mut s = String::new();
    for (i, &d) in digits[start..].iter().enumerate() {
        if i > 0 {
            s.push(' ');
        }
        s.push_str(cuneiform::glyph(d));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(data_len: usize) -> ViewState {
        ViewState::new(data_len, LensMode::None, TimeScale::Gar, false)
    }

    #[test]
    fn new_empty_input_has_no_cursor() {
        let s = state(0);
        assert_eq!(s.total_lines, 0);
        assert_eq!(s.scroll, 0);
        assert_eq!(s.cursor, None);
    }

    #[test]
    fn new_nonempty_input_starts_cursor_at_zero() {
        let s = state(80);
        assert_eq!(s.cursor, Some(0));
    }

    #[test]
    fn hjkl_moves_cursor_not_scroll() {
        let mut s = state(8 * 100); // 100 lines
        s.view_rows = 10;

        // Right: +1 byte.
        let _ = s.handle_key(KeyCode::Char('l'), KeyModifiers::NONE, b"");
        assert_eq!(s.cursor, Some(1));

        // Left past zero saturates.
        let _ = s.handle_key(KeyCode::Char('h'), KeyModifiers::NONE, b"");
        let _ = s.handle_key(KeyCode::Char('h'), KeyModifiers::NONE, b"");
        assert_eq!(s.cursor, Some(0));

        // Down: +CHUNK bytes = +1 row.
        let _ = s.handle_key(KeyCode::Char('j'), KeyModifiers::NONE, b"");
        assert_eq!(s.cursor, Some(CHUNK));

        // Up past zero saturates.
        let _ = s.handle_key(KeyCode::Char('k'), KeyModifiers::NONE, b"");
        let _ = s.handle_key(KeyCode::Char('k'), KeyModifiers::NONE, b"");
        assert_eq!(s.cursor, Some(0));
    }

    #[test]
    fn cursor_clamps_to_last_byte_on_g() {
        let mut s = state(8 * 100);
        let _ = s.handle_key(KeyCode::Char('G'), KeyModifiers::NONE, b"");
        assert_eq!(s.cursor, Some(8 * 100 - 1));
    }

    #[test]
    fn line_end_jumps_to_last_byte_of_row() {
        let mut s = state(8 * 100);
        let _ = s.handle_key(KeyCode::Char('$'), KeyModifiers::NONE, b"");
        assert_eq!(s.cursor, Some(CHUNK - 1));
    }

    #[test]
    fn line_start_returns_to_row_origin() {
        let mut s = state(8 * 100);
        // Move cursor to middle of a line then jump to line start.
        s.cursor = Some(CHUNK * 3 + 5);
        let _ = s.handle_key(KeyCode::Char('0'), KeyModifiers::NONE, b"");
        assert_eq!(s.cursor, Some(CHUNK * 3));
    }

    #[test]
    fn cursor_at_last_byte_clamps_instead_of_overflowing() {
        let mut s = state(8 * 2); // 2 rows.
        // Force cursor to last valid byte then try to move past.
        s.cursor = Some(15);
        let _ = s.handle_key(KeyCode::Char('l'), KeyModifiers::NONE, b"");
        assert_eq!(s.cursor, Some(15));
    }

    #[test]
    fn ctrl_d_moves_cursor_by_half_page_in_bytes() {
        let mut s = state(8 * 100);
        s.view_rows = 10;
        // Half page = 5 rows = 40 bytes.
        let _ = s.handle_key(KeyCode::Char('d'), KeyModifiers::CONTROL, b"");
        assert_eq!(s.cursor, Some(40));
    }

    #[test]
    fn quit_returns_break() {
        let mut s = state(80);
        let flow = s.handle_key(KeyCode::Char('q'), KeyModifiers::NONE, b"");
        assert!(flow.is_break());
    }

    #[test]
    fn non_ctrl_d_does_not_scroll_half_page() {
        let mut s = state(8 * 100);
        s.view_rows = 20;
        // `d` without Ctrl is unbound; cursor must not budge.
        let _ = s.handle_key(KeyCode::Char('d'), KeyModifiers::NONE, b"");
        assert_eq!(s.cursor, Some(0));
    }

    #[test]
    fn scroll_into_view_pulls_viewport_when_cursor_drops_below() {
        let mut s = state(8 * 100);
        s.view_rows = 10;
        s.cursor = Some(8 * 50);
        s.scroll_into_view();
        // Cursor sits at row 50; with viewport 10 rows, scroll must be
        // exactly 41 (so row 50 is the 10th visible row).
        assert_eq!(s.scroll, 41);
    }

    #[test]
    fn scroll_into_view_pulls_viewport_when_cursor_jumps_above() {
        let mut s = state(8 * 100);
        s.view_rows = 10;
        s.scroll = 40;
        s.cursor = Some(8 * 5); // row 5 — above the viewport.
        s.scroll_into_view();
        assert_eq!(s.scroll, 5);
    }

    #[test]
    fn shift_l_cycles_lens_mode() {
        let mut s = state(80);
        assert_eq!(s.lens_mode, LensMode::None);
        let _ = s.handle_key(KeyCode::Char('L'), KeyModifiers::NONE, b"");
        assert_eq!(s.lens_mode, LensMode::Time);
        let _ = s.handle_key(KeyCode::Char('L'), KeyModifiers::NONE, b"");
        assert_eq!(s.lens_mode, LensMode::Angle);
        let _ = s.handle_key(KeyCode::Char('L'), KeyModifiers::NONE, b"");
        assert_eq!(s.lens_mode, LensMode::Tablet);
        let _ = s.handle_key(KeyCode::Char('L'), KeyModifiers::NONE, b"");
        assert_eq!(s.lens_mode, LensMode::Cuneiform);
        let _ = s.handle_key(KeyCode::Char('L'), KeyModifiers::NONE, b"");
        assert_eq!(s.lens_mode, LensMode::None);
    }

    #[test]
    fn shift_l_rebuilds_lens_trait_object() {
        let mut s = state(80);
        assert!(s.lens.is_none());
        let _ = s.handle_key(KeyCode::Char('L'), KeyModifiers::NONE, b"");
        assert!(s.lens.is_some());
    }

    #[test]
    fn status_line_empty_input() {
        let s = state(0);
        let line = s.status_line(0, 0);
        let joined: String = line
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect();
        assert_eq!(joined, " empty input ");
    }

    #[test]
    fn status_line_populated_mentions_cursor_and_lens() {
        let mut s = state(8 * 100);
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
        assert!(joined.contains("lens —"));
    }

    #[test]
    fn status_line_shows_cuneiform_offset_when_cuneiform_lens_active() {
        let mut s = ViewState::new(80, LensMode::Cuneiform, TimeScale::Gar, false);
        s.cursor = Some(60); // offset encodes to "1:0" in base 60.
        let line = s.status_line(0, 10);
        let joined: String = line
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect();
        // We emit one cuneiform wedge-pair followed by a zero-placeholder.
        assert!(joined.contains('𒁹'), "expected wedge in {joined:?}");
        assert!(joined.contains('𒑰'), "expected zero mark in {joined:?}");
    }

    #[test]
    fn cuneiform_offset_strips_leading_zeros() {
        let rendered = cuneiform_offset(5);
        // `5` in base-60 is `00:00:...:05` — only the five-wedge digit should remain.
        assert_eq!(rendered, "𒁹𒁹𒁹𒁹𒁹");
    }

    #[test]
    fn cuneiform_offset_zero_renders_single_placeholder() {
        assert_eq!(cuneiform_offset(0), "𒑰");
    }

    #[test]
    fn slash_enters_search_mode() {
        let mut s = state(80);
        let _ = s.handle_key(KeyCode::Char('/'), KeyModifiers::NONE, b"");
        assert!(matches!(s.mode, Mode::SearchInput(ref b) if b.is_empty()));
    }

    #[test]
    fn typing_in_search_mode_accumulates_buffer() {
        let data = b"Hello there";
        let mut s = state(data.len());
        let _ = s.handle_key(KeyCode::Char('/'), KeyModifiers::NONE, data);
        let _ = s.handle_key(KeyCode::Char('H'), KeyModifiers::NONE, data);
        let _ = s.handle_key(KeyCode::Char('i'), KeyModifiers::NONE, data);
        assert!(matches!(s.mode, Mode::SearchInput(ref b) if b == "Hi"));
    }

    #[test]
    fn backspace_in_search_mode_pops_last_char() {
        let mut s = state(80);
        let _ = s.handle_key(KeyCode::Char('/'), KeyModifiers::NONE, b"");
        let _ = s.handle_key(KeyCode::Char('a'), KeyModifiers::NONE, b"");
        let _ = s.handle_key(KeyCode::Char('b'), KeyModifiers::NONE, b"");
        let _ = s.handle_key(KeyCode::Backspace, KeyModifiers::NONE, b"");
        assert!(matches!(s.mode, Mode::SearchInput(ref b) if b == "a"));
    }

    #[test]
    fn esc_cancels_search_input() {
        let mut s = state(80);
        let _ = s.handle_key(KeyCode::Char('/'), KeyModifiers::NONE, b"");
        let _ = s.handle_key(KeyCode::Esc, KeyModifiers::NONE, b"");
        assert!(matches!(s.mode, Mode::Normal));
    }

    #[test]
    fn enter_confirms_search_and_jumps_to_first_match() {
        let data: &[u8] = b"garbage\x00ELF\x7fstuff";
        let mut s = state(data.len());
        s.cursor = Some(0);
        let _ = s.handle_key(KeyCode::Char('/'), KeyModifiers::NONE, data);
        for c in "ELF".chars() {
            let _ = s.handle_key(KeyCode::Char(c), KeyModifiers::NONE, data);
        }
        let _ = s.handle_key(KeyCode::Enter, KeyModifiers::NONE, data);
        assert!(matches!(s.mode, Mode::Normal));
        assert_eq!(s.matches, vec![8]);
        assert_eq!(s.cursor, Some(8));
        assert_eq!(s.match_idx, 0);
    }

    #[test]
    fn n_and_capital_n_cycle_through_matches() {
        let data: &[u8] = b"abXYZcdXYZefXYZgh";
        let mut s = state(data.len());
        // Prime the matches as if a `/XYZ` search just confirmed.
        s.matches = search::find_all(data, b"XYZ");
        s.match_idx = 0;
        s.cursor = Some(s.matches[0]);

        let _ = s.handle_key(KeyCode::Char('n'), KeyModifiers::NONE, data);
        assert_eq!(s.match_idx, 1);
        assert_eq!(s.cursor, Some(7));

        let _ = s.handle_key(KeyCode::Char('n'), KeyModifiers::NONE, data);
        assert_eq!(s.match_idx, 2);

        // Wrap-around forward.
        let _ = s.handle_key(KeyCode::Char('n'), KeyModifiers::NONE, data);
        assert_eq!(s.match_idx, 0);

        // Backward from 0 wraps to the last match.
        let _ = s.handle_key(KeyCode::Char('N'), KeyModifiers::NONE, data);
        assert_eq!(s.match_idx, 2);
    }

    #[test]
    fn n_without_active_search_sets_status_message() {
        let mut s = state(80);
        let _ = s.handle_key(KeyCode::Char('n'), KeyModifiers::NONE, b"");
        assert_eq!(s.status_message.as_deref(), Some("no active search"));
    }

    #[test]
    fn search_bad_pattern_reports_error_via_status_message() {
        let mut s = state(80);
        let _ = s.handle_key(KeyCode::Char('/'), KeyModifiers::NONE, b"");
        let _ = s.handle_key(KeyCode::Char('h'), KeyModifiers::NONE, b"");
        let _ = s.handle_key(KeyCode::Char('e'), KeyModifiers::NONE, b"");
        let _ = s.handle_key(KeyCode::Char('x'), KeyModifiers::NONE, b"");
        let _ = s.handle_key(KeyCode::Char(':'), KeyModifiers::NONE, b"");
        let _ = s.handle_key(KeyCode::Char('z'), KeyModifiers::NONE, b"");
        let _ = s.handle_key(KeyCode::Enter, KeyModifiers::NONE, b"");
        let msg = s.status_message.as_deref().unwrap_or("");
        assert!(msg.starts_with("bad pattern"), "got {msg:?}");
    }

    #[test]
    fn search_mode_quit_key_does_not_exit_tui() {
        // While typing, `q` must be captured as literal text, not fire
        // the quit accelerator — otherwise a search query beginning with
        // `q` would close the viewer instead of filtering results.
        let mut s = state(80);
        let _ = s.handle_key(KeyCode::Char('/'), KeyModifiers::NONE, b"");
        let flow = s.handle_key(KeyCode::Char('q'), KeyModifiers::NONE, b"");
        assert!(flow.is_continue());
        assert!(matches!(s.mode, Mode::SearchInput(ref b) if b == "q"));
    }

    #[test]
    fn status_line_shows_search_prompt_in_search_mode() {
        let mut s = state(80);
        let _ = s.handle_key(KeyCode::Char('/'), KeyModifiers::NONE, b"");
        let _ = s.handle_key(KeyCode::Char('a'), KeyModifiers::NONE, b"");
        let _ = s.handle_key(KeyCode::Char('b'), KeyModifiers::NONE, b"");
        let line = s.status_line(0, 10);
        let joined: String = line
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect();
        assert!(joined.contains("search:"));
        assert!(joined.contains("ab"));
    }
}
