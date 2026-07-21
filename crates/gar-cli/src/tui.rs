//! Interactive terminal viewer (optional `--interactive` flag).

use crate::analyze::{self, Analysis, DEFAULT_WINDOW, RegionKind};
use crate::chunk::CHUNK;
use crate::cli::{LensMode, TimeScale, build_lens};
use crate::dump::{ASCII_COLUMN_START, border_style, status_style, styled_line, title_style};
use crate::persist::{self, PersistedState};
use crate::search::{self, Pattern};
use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use gar_core::cuneiform;
use gar_core::lens::Lens;
use ratatui::Terminal;
use ratatui::backend::Backend;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use std::borrow::Cow;
use std::io;
use std::path::{Path, PathBuf};

const TITLE: &str = "— hjkl  </>:pan  v/V:pin  L:lens  /:find  m/':mark  ]/[ pze:jump  q:quit 𒌋 ";

/// Maximum bytes fed to the background analysis thread. Keeps startup
/// fast for huge files — semantic jumps cover this prefix; the cursor
/// and scroll work across the full input regardless.
const MAX_ANALYSIS_BYTES: usize = 64 * 1024 * 1024;

/// Modal state of the viewer. `Normal` is the cursor-driven default; the
/// other variants trap almost every keypress so the user can compose an
/// input without accidentally scrolling.
enum Mode {
    Normal,
    /// User is typing a search query into the status bar.
    SearchInput(String),
    /// `m` was pressed; the next printable ASCII letter names the slot to
    /// save the cursor byte into.
    BookmarkSet,
    /// `'` was pressed; the next printable ASCII letter names the slot
    /// whose offset replaces the cursor.
    BookmarkJump,
    /// `]` or `[` was pressed; the next letter picks a region kind.
    /// `forward` mirrors the leader: `]` → true, `[` → false.
    SemanticJump {
        forward: bool,
    },
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
    input_file: Option<&Path>,
) -> Result<()> {
    ratatui::run(|terminal| -> Result<()> {
        run_with_terminal(
            terminal,
            data,
            base_offset,
            initial_mode,
            scale,
            purist,
            input_file,
            || crossterm::event::read().map(Some),
        )
    })
}

/// Seam driving the TUI event loop against an arbitrary [`Backend`] and
/// event source. The production [`run`] wraps this with
/// [`ratatui::run`] + [`crossterm::event::read`]; integration tests pass
/// [`ratatui::backend::TestBackend`] + a pre-built event iterator.
///
/// `next_event` returns `Ok(None)` to signal clean shutdown (event
/// source exhausted) — the TUI saves state (if `input_file` is `Some`)
/// and returns without error.
///
/// # Errors
///
/// Propagates I/O errors from [`Terminal::draw`] or the injected
/// `next_event` closure.
// 8 args maps 1:1 to `run`'s 6 args plus the `&mut Terminal` + event
// closure injected for tests; splitting would just move the tuple one
// indirection deeper without improving the call sites.
#[allow(
    clippy::too_many_arguments,
    reason = "test hook maps 1:1 to run's args plus terminal and event closure"
)]
#[doc(hidden)]
pub fn run_with_terminal<B, F>(
    terminal: &mut Terminal<B>,
    data: &[u8],
    base_offset: u64,
    initial_mode: LensMode,
    scale: TimeScale,
    purist: bool,
    input_file: Option<&Path>,
    mut next_event: F,
) -> Result<()>
where
    B: Backend,
    B::Error: std::error::Error + Send + Sync + 'static,
    F: FnMut() -> io::Result<Option<Event>>,
{
    let mut state = ViewState::new(data, initial_mode, scale, purist);

    // Per-file persistence: restore on entry, persist on clean exit.
    // Stdin input has no persistent identity, so we skip entirely.
    let persist_path: Option<PathBuf> = input_file.map(Path::to_path_buf);
    if let Some(path) = &persist_path
        && let Some(saved) = persist::load(path)
    {
        state.apply_persisted(saved);
    }

    loop {
        terminal.draw(|frame| state.draw(frame, data, base_offset))?;

        // Collect background analysis result without blocking the frame.
        if state.analysis.is_none()
            && let Some(rx) = &state.analysis_rx
            && let Ok(a) = rx.try_recv()
        {
            state.analysis = Some(a);
            state.analysis_rx = None;
        }

        let Some(event) = next_event()? else {
            // Event source exhausted — persist state on clean shutdown.
            if let Some(path) = &persist_path {
                persist::save(path, &state.snapshot());
            }
            return Ok(());
        };

        let Event::Key(key) = event else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        if state.handle_key(key.code, key.modifiers, data).is_break() {
            if let Some(path) = &persist_path {
                persist::save(path, &state.snapshot());
            }
            return Ok(());
        }
    }
}

/// Scroll state + derived layout sizes shared between `draw` and
/// `handle_key`.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
struct RowCacheKey {
    scroll: usize,
    visible_end: usize,
    lens_mode: LensMode,
    base_offset: u64,
    width: u16,
    height: u16,
    horizontal: u16,
}

#[derive(Default)]
struct RowCache {
    key: Option<RowCacheKey>,
    paragraph: Paragraph<'static>,
    max_horizontal: u16,
    #[cfg(test)]
    rebuilds: usize,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
struct PinnedView {
    scroll: usize,
    cursor: usize,
}

#[allow(
    clippy::too_many_arguments,
    reason = "row cache key and renderer inputs stay explicit at the draw boundary"
)]
fn ensure_row_cache(
    cache: &mut RowCache,
    data: &[u8],
    scroll: usize,
    rows: usize,
    area: ratatui::layout::Rect,
    base_offset: u64,
    lens_mode: LensMode,
    lens: Option<&dyn Lens>,
    horizontal: u16,
    title_label: &str,
    title_offset: u64,
) -> (usize, u16, u16) {
    let total_lines = data.len().div_ceil(CHUNK);
    let visible_end = scroll.saturating_add(rows).min(total_lines);
    let requested_key = RowCacheKey {
        scroll,
        visible_end,
        lens_mode,
        base_offset,
        width: area.width,
        height: area.height,
        horizontal,
    };
    if cache.key != Some(requested_key) {
        let lines: Vec<Line<'static>> = (scroll..visible_end)
            .map(|row| {
                let start = row * CHUNK;
                let end = (start + CHUNK).min(data.len());
                let offset = base_offset.saturating_add(start as u64);
                styled_line(offset, &data[start..end], lens, None)
            })
            .collect();
        let content_width = lines.iter().map(Line::width).max().unwrap_or(0);
        let inner_width = usize::from(area.width.saturating_sub(2));
        cache.max_horizontal =
            u16::try_from(content_width.saturating_sub(inner_width)).unwrap_or(u16::MAX);
        let horizontal = horizontal.min(cache.max_horizontal);
        let title = Line::from(Span::styled(
            format!(" {title_label} 0x{title_offset:08x} {TITLE}"),
            title_style(),
        ));
        cache.paragraph = Paragraph::new(lines).scroll((0, horizontal)).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style())
                .title(title),
        );
        cache.key = Some(RowCacheKey {
            horizontal,
            ..requested_key
        });
        #[cfg(test)]
        {
            cache.rebuilds += 1;
        }
    }

    let horizontal = cache.key.map_or(0, |key| key.horizontal);
    (visible_end, cache.max_horizontal, horizontal)
}

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
    /// Horizontal text offset shared by the active and pinned panes.
    horizontal: u16,
    /// Largest valid horizontal offset for the most recently rendered pane.
    max_horizontal: u16,
    /// Frozen viewport used for side-by-side region comparison.
    pinned: Option<PinnedView>,
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
    status_message: Option<Cow<'static, str>>,
    /// Vim-style bookmarks indexed by letter (`a-z` → 0–25). Values
    /// are byte offsets inside the loaded slice.
    bookmarks: [Option<usize>; 26],
    /// Precomputed statistical analysis of the loaded slice. Used by
    /// semantic jumps (`]p`, `]z`, `]e`) to locate the next/prev
    /// ASCII run, zero fill, or entropy spike without rescanning on
    /// every keystroke. `None` until the background thread delivers.
    analysis: Option<Analysis>,
    /// Receiver for the background analysis thread. `None` once the
    /// result has been collected.
    analysis_rx: Option<std::sync::mpsc::Receiver<Analysis>>,
    /// Bounded render cache containing only the current visible row window.
    row_cache: RowCache,
    /// Separate bounded cache for the frozen comparison pane.
    pinned_cache: RowCache,
}

impl ViewState {
    fn new(data: &[u8], initial_mode: LensMode, scale: TimeScale, purist: bool) -> Self {
        let total_lines = data.len().div_ceil(CHUNK);
        Self {
            data_len: data.len(),
            total_lines,
            scroll: 0,
            cursor: if data.is_empty() { None } else { Some(0) },
            view_rows: 1,
            horizontal: 0,
            max_horizontal: 0,
            pinned: None,
            lens_mode: initial_mode,
            lens: build_lens(initial_mode, scale, purist, true),
            scale,
            purist,
            mode: Mode::Normal,
            matches: Vec::new(),
            match_idx: usize::MAX,
            status_message: None,
            bookmarks: [None; 26],
            analysis: None,
            analysis_rx: {
                let (tx, rx) = std::sync::mpsc::channel();
                let slice = &data[..data.len().min(MAX_ANALYSIS_BYTES)];
                let owned = slice.to_vec();
                std::thread::spawn(move || {
                    let _ = tx.send(analyze::analyze(&owned, DEFAULT_WINDOW));
                });
                Some(rx)
            },
            row_cache: RowCache::default(),
            pinned_cache: RowCache::default(),
        }
    }

    /// Construct with eagerly-computed analysis. Used only by tests so
    /// semantic-jump assertions don't race the background thread.
    #[cfg(test)]
    fn new_with_analysis(
        data: &[u8],
        initial_mode: LensMode,
        scale: TimeScale,
        purist: bool,
    ) -> Self {
        let mut s = Self::new(data, initial_mode, scale, purist);
        // Drain the background result synchronously.
        if let Some(rx) = s.analysis_rx.take()
            && let Ok(a) = rx.recv()
        {
            s.analysis = Some(a);
        }
        s
    }

    fn draw(&mut self, frame: &mut ratatui::Frame<'_>, data: &[u8], base_offset: u64) {
        let [body_area, status_area] =
            Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(frame.area());

        let split_pinned = self.pinned.is_some() && body_area.height >= 6;
        let (active_area, pinned_area) = if split_pinned {
            let [active, pinned] =
                Layout::vertical([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
                    .areas(body_area);
            (active, Some(pinned))
        } else {
            (body_area, None)
        };

        // Subtract the two border rows from the active pane. Page movement
        // follows the pane the cursor actually controls.
        let rows = usize::from(active_area.height).saturating_sub(2).max(1);
        self.view_rows = rows;
        self.scroll_into_view();

        let lens_ref: Option<&dyn Lens> = self.lens.as_deref();
        let (visible_end, max_horizontal, horizontal) = ensure_row_cache(
            &mut self.row_cache,
            data,
            self.scroll,
            rows,
            active_area,
            base_offset,
            self.lens_mode,
            lens_ref,
            self.horizontal,
            "active",
            base_offset.saturating_add((self.scroll * CHUNK) as u64),
        );
        self.max_horizontal = max_horizontal;
        self.horizontal = horizontal;
        frame.render_widget(&self.row_cache.paragraph, active_area);
        self.render_cursor_at(frame, active_area, self.scroll, visible_end, self.cursor);

        if let (Some(area), Some(pin)) = (pinned_area, self.pinned) {
            let pinned_rows = usize::from(area.height).saturating_sub(2).max(1);
            let (pinned_end, _, _) = ensure_row_cache(
                &mut self.pinned_cache,
                data,
                pin.scroll,
                pinned_rows,
                area,
                base_offset,
                self.lens_mode,
                lens_ref,
                self.horizontal,
                "pinned",
                base_offset.saturating_add(pin.cursor as u64),
            );
            frame.render_widget(&self.pinned_cache.paragraph, area);
            self.render_cursor_at(frame, area, pin.scroll, pinned_end, Some(pin.cursor));
        }

        let status_line = if self.pinned.is_some() && !split_pinned {
            Line::from(Span::styled(
                " terminal too short for pinned view ",
                status_style(),
            ))
        } else {
            self.status_line(base_offset, visible_end)
        };
        frame.render_widget(
            Paragraph::new(status_line).style(Style::default()),
            status_area,
        );
    }

    /// Apply cursor emphasis directly to the rendered cell so moving within
    /// the cached viewport does not rebuild or clone any row spans.
    fn render_cursor_at(
        &self,
        frame: &mut ratatui::Frame<'_>,
        body_area: ratatui::layout::Rect,
        scroll: usize,
        visible_end: usize,
        cursor: Option<usize>,
    ) {
        let Some(cursor) = cursor else {
            return;
        };
        let row = cursor / CHUNK;
        if row < scroll || row >= visible_end {
            return;
        }
        let Ok(cursor_column) = u16::try_from(cursor % CHUNK) else {
            return;
        };
        let Ok(visible_row) = u16::try_from(row - scroll) else {
            return;
        };

        let logical_column = ASCII_COLUMN_START.saturating_add(cursor_column);
        if logical_column < self.horizontal {
            return;
        }

        let x = body_area
            .x
            .saturating_add(1)
            .saturating_add(logical_column - self.horizontal);
        let y = body_area.y.saturating_add(1).saturating_add(visible_row);
        if x >= body_area.right().saturating_sub(1) || y >= body_area.bottom().saturating_sub(1) {
            return;
        }
        if let Some(cell) = frame.buffer_mut().cell_mut((x, y)) {
            cell.set_style(cell.style().add_modifier(Modifier::REVERSED));
        }
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
        if matches!(self.mode, Mode::BookmarkSet) {
            return Line::from(vec![
                Span::styled(" 𒑭 set bookmark (a-z): ", label),
                Span::raw(" "),
            ]);
        }
        if matches!(self.mode, Mode::BookmarkJump) {
            return Line::from(vec![
                Span::styled(" 𒑭 jump bookmark (a-z): ", label),
                Span::raw(" "),
            ]);
        }
        if let Mode::SemanticJump { forward } = self.mode {
            let arrow = if forward { "next" } else { "prev" };
            return Line::from(vec![
                Span::styled(format!(" 𒑭 jump {arrow} (p/z/e): "), label),
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
            return Line::from(Span::styled(" empty input   size 0   position 0% ", label));
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

        let position = self.cursor.map_or(0, |cursor| {
            let numerator = u128::try_from(cursor.saturating_add(1))
                .unwrap_or(u128::MAX)
                .saturating_mul(100);
            let denominator = u128::try_from(self.data_len).unwrap_or(u128::MAX);
            u8::try_from(numerator / denominator).unwrap_or(100)
        });

        spans.push(Span::styled("   size ", label));
        spans.push(Span::styled(self.data_len.to_string(), Style::default()));
        spans.push(Span::styled("   position ", label));
        spans.push(Span::styled(format!("{position}%"), Style::default()));

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
        // quit the TUI while the user is typing a search query or
        // naming a bookmark slot.
        if let Mode::SearchInput(_) = &self.mode {
            return self.handle_search_input_key(code, data);
        }
        if matches!(self.mode, Mode::BookmarkSet | Mode::BookmarkJump) {
            return self.handle_bookmark_key(code);
        }
        if let Mode::SemanticJump { forward } = self.mode {
            return self.handle_semantic_jump_key(code, forward);
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
            KeyCode::Char('>') => {
                self.horizontal = self.horizontal.saturating_add(4).min(self.max_horizontal);
            }
            KeyCode::Char('<') => self.horizontal = self.horizontal.saturating_sub(4),
            KeyCode::Char('v') => self.toggle_pin(),
            KeyCode::Char('V') => self.set_pin(),
            // Capital L cycles through the five lens modes: None → Time →
            // Angle → Tablet → Cuneiform → None. Lower-case `l` already
            // moves the cursor, hence the `Shift+l` choice.
            KeyCode::Char('L') => self.cycle_lens(),
            // Search: `/` opens the modal input; `n`/`N` navigate between
            // matches from the last confirmed query.
            KeyCode::Char('/') => self.mode = Mode::SearchInput(String::new()),
            KeyCode::Char('n') => self.jump_to_next_match(),
            KeyCode::Char('N') => self.jump_to_prev_match(),
            // Bookmarks: `m` + letter stores the cursor; `'` + letter
            // jumps to the saved offset. Gaps in the slot namespace
            // (`a-z`) are sentinel — they don't bind anything else.
            KeyCode::Char('m') => self.mode = Mode::BookmarkSet,
            KeyCode::Char('\'') => self.mode = Mode::BookmarkJump,
            // Semantic jumps: `]<letter>` forward, `[<letter>` backward.
            // Letters: p = printable ASCII run, z = low-entropy window,
            // e = high-entropy window. Uses the precomputed Analysis.
            KeyCode::Char(']') => self.mode = Mode::SemanticJump { forward: true },
            KeyCode::Char('[') => self.mode = Mode::SemanticJump { forward: false },
            _ => {}
        }
        std::ops::ControlFlow::Continue(())
    }

    fn handle_semantic_jump_key(
        &mut self,
        code: KeyCode,
        forward: bool,
    ) -> std::ops::ControlFlow<()> {
        self.mode = Mode::Normal;
        let KeyCode::Char(c) = code else {
            return std::ops::ControlFlow::Continue(());
        };
        let kind = match c {
            'p' => RegionKind::Ascii,
            'z' => RegionKind::LowEntropy,
            'e' => RegionKind::HighEntropy,
            _ => {
                self.status_message =
                    Some(Cow::Owned(format!("unknown jump letter {c:?} (p/z/e)")));
                return std::ops::ControlFlow::Continue(());
            }
        };
        self.jump_to_region(kind, forward);
        std::ops::ControlFlow::Continue(())
    }

    fn jump_to_region(&mut self, kind: RegionKind, forward: bool) {
        let Some(here) = self.cursor else {
            return;
        };
        let direction_label = if forward { "next" } else { "prev" };
        let kind_label = region_label(kind);

        let Some(analysis) = &self.analysis else {
            self.status_message = Some(Cow::Borrowed("analysing... (jump deferred)"));
            return;
        };

        let candidate = if forward {
            analysis
                .regions
                .iter()
                .find(|r| r.kind == kind && r.start > here)
        } else {
            // `rev()` walks regions right-to-left; we want the last region
            // starting strictly before the cursor.
            analysis
                .regions
                .iter()
                .rev()
                .find(|r| r.kind == kind && r.start < here)
        };

        if let Some(region) = candidate {
            self.cursor = Some(region.start);
            self.status_message = Some(Cow::Owned(format!(
                "{direction_label} {kind_label} @ 0x{:08x}",
                region.start
            )));
        } else {
            self.status_message = Some(Cow::Owned(format!("no {direction_label} {kind_label}")));
        }
    }

    fn handle_bookmark_key(&mut self, code: KeyCode) -> std::ops::ControlFlow<()> {
        let setting = matches!(self.mode, Mode::BookmarkSet);
        self.mode = Mode::Normal;
        let KeyCode::Char(c) = code else {
            // Esc, enter, arrow keys — anything non-letter just cancels.
            return std::ops::ControlFlow::Continue(());
        };
        let Some(idx) = bookmark_idx(c) else {
            self.status_message = Some(Cow::Owned(format!("bookmarks use a-z, got {c:?}")));
            return std::ops::ControlFlow::Continue(());
        };
        if setting {
            let Some(byte) = self.cursor else {
                self.status_message = Some(Cow::Borrowed("empty input — nothing to mark"));
                return std::ops::ControlFlow::Continue(());
            };
            self.bookmarks[idx] = Some(byte);
            self.status_message = Some(Cow::Owned(format!(
                "marked '{}' = 0x{byte:08x}",
                slot_char(idx),
            )));
        } else if let Some(byte) = self.bookmarks[idx] {
            self.cursor = Some(byte);
            self.status_message = Some(Cow::Owned(format!(
                "jumped '{}' = 0x{byte:08x}",
                slot_char(idx),
            )));
        } else {
            self.status_message = Some(Cow::Owned(format!(
                "bookmark '{}' not set",
                slot_char(idx),
            )));
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
                    self.status_message = Some(Cow::Owned(format!(
                        "match 1/{} at 0x{first:08x}",
                        self.matches.len()
                    )));
                } else {
                    self.status_message = Some(Cow::Owned(format!("no match for {buf:?}")));
                }
            }
            Err(_) => {
                self.status_message = Some(Cow::Owned(format!("bad pattern: {buf:?}")));
            }
        }
    }

    fn jump_to_next_match(&mut self) {
        if self.matches.is_empty() {
            self.status_message = Some(Cow::Borrowed("no active search"));
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
        self.status_message = Some(Cow::Owned(format!(
            "match {}/{} at 0x{byte:08x}",
            next + 1,
            self.matches.len()
        )));
    }

    fn jump_to_prev_match(&mut self) {
        if self.matches.is_empty() {
            self.status_message = Some(Cow::Borrowed("no active search"));
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
        self.status_message = Some(Cow::Owned(format!(
            "match {}/{} at 0x{byte:08x}",
            prev + 1,
            self.matches.len()
        )));
    }

    fn cycle_lens(&mut self) {
        self.lens_mode = self.lens_mode.cycle();
        self.lens = build_lens(self.lens_mode, self.scale, self.purist, true);
    }

    fn toggle_pin(&mut self) {
        if self.pinned.is_some() {
            self.pinned = None;
            self.pinned_cache = RowCache::default();
        } else {
            self.set_pin();
        }
    }

    fn set_pin(&mut self) {
        let Some(cursor) = self.cursor else {
            self.status_message = Some(Cow::Borrowed("empty input — nothing to pin"));
            return;
        };
        self.pinned = Some(PinnedView {
            scroll: self.scroll,
            cursor,
        });
        self.pinned_cache = RowCache::default();
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

    /// Apply a previously-persisted state on top of the defaults set by
    /// [`ViewState::new`]. Out-of-range cursor/scroll are clamped against
    /// the current slice (e.g. if the file shrank between runs).
    fn apply_persisted(&mut self, saved: PersistedState) {
        let last = self.data_len.saturating_sub(1);
        let max_scroll = self.total_lines.saturating_sub(1);
        self.scroll = byte_min(saved.scroll, max_scroll);
        self.cursor = saved.cursor.map(|c| byte_min(c, last));
        self.lens_mode = saved.lens_mode;
        self.lens = build_lens(saved.lens_mode, self.scale, self.purist, true);
        self.bookmarks = [None; 26];
        for (c, byte) in saved.bookmarks {
            if byte < self.data_len
                && let Some(idx) = bookmark_idx(c)
            {
                self.bookmarks[idx] = Some(byte);
            }
        }
    }

    /// Snapshot the subset of state that survives across runs.
    fn snapshot(&self) -> PersistedState {
        let marks: Vec<(char, usize)> = (0..26)
            .filter_map(|i| self.bookmarks[i].map(|b| (slot_char(i), b)))
            .collect();
        PersistedState {
            scroll: self.scroll,
            cursor: self.cursor,
            lens_mode: self.lens_mode,
            bookmarks: marks,
        }
    }
}

const fn byte_min(a: usize, b: usize) -> usize {
    if a < b { a } else { b }
}

/// Map a letter to a bookmark array index: `'a'` → 0, `'z'` → 25.
/// Returns `None` for non-alphabetic or non-ASCII characters.
#[must_use]
const fn bookmark_idx(c: char) -> Option<usize> {
    let b = c as u32;
    if b >= 0x61 && b <= 0x7a {
        Some((b - 0x61) as usize)
    } else if b >= 0x41 && b <= 0x5a {
        Some((b - 0x41) as usize)
    } else {
        None
    }
}

/// Map a bookmark array index (0–25) back to its lowercase letter.
/// Returns `'a'` for index 0, `'z'` for index 25.
#[must_use]
const fn slot_char(idx: usize) -> char {
    const SLOTS: [u8; 26] = *b"abcdefghijklmnopqrstuvwxyz";
    SLOTS[idx] as char
}

const fn region_label(kind: RegionKind) -> &'static str {
    match kind {
        RegionKind::Ascii => "printable",
        RegionKind::LowEntropy => "zero-run",
        RegionKind::HighEntropy => "entropy-spike",
    }
}

/// Render `offset` (a byte address) as Sumero-Babylonian cuneiform,
/// stripping leading zero-placeholders so the display is compact.
///
/// A u64 takes up to 11 base-60 digits, but offsets are usually small;
/// skipping leading zeroes keeps the status line readable.
fn cuneiform_offset(offset: u64) -> String {
    let digits = gar_core::convert::u64_to_base60(offset);
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
    use ratatui::backend::TestBackend;

    fn state(data_len: usize) -> ViewState {
        // Analysis scans the slice we hand it; a buffer of zeros is a
        // stable, cheap fixture for the geometry-only tests that don't
        // care about actual byte contents.
        let data = vec![0_u8; data_len];
        ViewState::new(&data, LensMode::None, TimeScale::Gar, false)
    }

    fn render_test_frame(
        state: &mut ViewState,
        data: &[u8],
        width: u16,
        height: u16,
    ) -> ratatui::buffer::Buffer {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| state.draw(frame, data, 0)).unwrap();
        terminal.backend().buffer().clone()
    }

    fn draw_test_frame(state: &mut ViewState, data: &[u8], width: u16, height: u16) {
        let _ = render_test_frame(state, data, width, height);
    }

    fn buffer_text(buffer: &ratatui::buffer::Buffer) -> String {
        let mut text = String::new();
        for y in buffer.area.y..buffer.area.bottom() {
            for x in buffer.area.x..buffer.area.right() {
                text.push_str(buffer[(x, y)].symbol());
            }
            text.push('\n');
        }
        text
    }

    #[test]
    fn unchanged_frames_reuse_cached_rows() {
        let data = vec![0_u8; 80];
        let mut state = ViewState::new(&data, LensMode::None, TimeScale::Gar, false);
        draw_test_frame(&mut state, &data, 80, 24);
        draw_test_frame(&mut state, &data, 80, 24);
        assert_eq!(state.row_cache.rebuilds, 1);
    }

    #[test]
    fn scrolling_and_lens_changes_invalidate_cached_rows() {
        let data = vec![0_u8; 8 * 100];
        let mut state = ViewState::new(&data, LensMode::None, TimeScale::Gar, false);
        draw_test_frame(&mut state, &data, 80, 12);
        state.cursor = Some(8 * 50);
        draw_test_frame(&mut state, &data, 80, 12);
        state.cycle_lens();
        draw_test_frame(&mut state, &data, 80, 12);
        assert_eq!(state.row_cache.rebuilds, 3);
    }

    #[test]
    fn cursor_overlay_moves_without_rebuilding_cached_rows() {
        let data = b"ABCDEFGH";
        let mut state = ViewState::new(data, LensMode::None, TimeScale::Gar, false);
        state.cursor = Some(3);
        let first = render_test_frame(&mut state, data, 80, 6);
        assert!(
            first[(ASCII_COLUMN_START + 1 + 3, 1)]
                .modifier
                .contains(Modifier::REVERSED)
        );

        state.cursor_fwd(1);
        let second = render_test_frame(&mut state, data, 80, 6);
        assert!(
            !second[(ASCII_COLUMN_START + 1 + 3, 1)]
                .modifier
                .contains(Modifier::REVERSED)
        );
        assert!(
            second[(ASCII_COLUMN_START + 1 + 4, 1)]
                .modifier
                .contains(Modifier::REVERSED)
        );
        assert_eq!(state.row_cache.rebuilds, 1);
    }

    #[test]
    fn horizontal_scroll_moves_four_columns_and_clamps() {
        let data = vec![0_u8; 80];
        let mut state = ViewState::new(&data, LensMode::None, TimeScale::Gar, false);
        draw_test_frame(&mut state, &data, 30, 8);
        assert!(state.max_horizontal > 4);

        let _ = state.handle_key(KeyCode::Char('>'), KeyModifiers::NONE, &data);
        assert_eq!(state.horizontal, 4);
        for _ in 0..100 {
            let _ = state.handle_key(KeyCode::Char('>'), KeyModifiers::NONE, &data);
        }
        assert_eq!(state.horizontal, state.max_horizontal);
        let _ = state.handle_key(KeyCode::Char('<'), KeyModifiers::NONE, &data);
        assert_eq!(state.horizontal, state.max_horizontal.saturating_sub(4));
        for _ in 0..100 {
            let _ = state.handle_key(KeyCode::Char('<'), KeyModifiers::NONE, &data);
        }
        assert_eq!(state.horizontal, 0);
    }

    #[test]
    fn horizontal_scroll_keeps_cursor_overlay_aligned() {
        let data = b"ABCDEFGH";
        let mut state = ViewState::new(data, LensMode::None, TimeScale::Gar, false);
        state.cursor = Some(3);
        draw_test_frame(&mut state, data, 30, 6);
        for _ in 0..100 {
            let _ = state.handle_key(KeyCode::Char('>'), KeyModifiers::NONE, data);
        }
        let buffer = render_test_frame(&mut state, data, 30, 6);
        let shifted_x = ASCII_COLUMN_START + 1 + 3 - state.horizontal;
        assert!(buffer[(shifted_x, 1)].modifier.contains(Modifier::REVERSED));
    }

    #[test]
    fn pin_toggle_and_replace_capture_current_view() {
        let data = vec![0_u8; 8 * 100];
        let mut state = ViewState::new(&data, LensMode::None, TimeScale::Gar, false);
        state.cursor = Some(16);
        state.scroll = 2;
        let _ = state.handle_key(KeyCode::Char('v'), KeyModifiers::NONE, &data);
        assert_eq!(state.pinned.as_ref().map(|pin| pin.cursor), Some(16));
        assert_eq!(state.pinned.as_ref().map(|pin| pin.scroll), Some(2));

        let _ = state.handle_key(KeyCode::Char('v'), KeyModifiers::NONE, &data);
        assert!(state.pinned.is_none());

        state.cursor = Some(64);
        state.scroll = 8;
        let _ = state.handle_key(KeyCode::Char('V'), KeyModifiers::NONE, &data);
        assert_eq!(state.pinned.as_ref().map(|pin| pin.cursor), Some(64));
        state.cursor = Some(96);
        state.scroll = 12;
        let _ = state.handle_key(KeyCode::Char('V'), KeyModifiers::NONE, &data);
        assert_eq!(state.pinned.as_ref().map(|pin| pin.cursor), Some(96));
        assert_eq!(state.pinned.as_ref().map(|pin| pin.scroll), Some(12));
    }

    #[test]
    fn split_view_titles_show_distinct_absolute_offsets() {
        let data = vec![0_u8; 8 * 100];
        let mut state = ViewState::new(&data, LensMode::None, TimeScale::Gar, false);
        let _ = state.handle_key(KeyCode::Char('v'), KeyModifiers::NONE, &data);
        state.cursor = Some(8 * 50);

        let buffer = render_test_frame(&mut state, &data, 80, 20);
        let rendered = buffer_text(&buffer);
        let active_offset = state.scroll * CHUNK;
        assert!(rendered.contains(&format!("active 0x{active_offset:08x}")));
        assert!(rendered.contains("pinned 0x00000000"));
    }

    #[test]
    fn unchanged_split_frames_reuse_both_bounded_caches() {
        let data = vec![0_u8; 8 * 100];
        let mut state = ViewState::new(&data, LensMode::None, TimeScale::Gar, false);
        let _ = state.handle_key(KeyCode::Char('v'), KeyModifiers::NONE, &data);
        draw_test_frame(&mut state, &data, 80, 20);
        draw_test_frame(&mut state, &data, 80, 20);
        assert_eq!(state.row_cache.rebuilds, 1);
        assert_eq!(state.pinned_cache.rebuilds, 1);
    }

    #[test]
    fn short_terminal_falls_back_to_one_pane_with_message() {
        let data = vec![0_u8; 80];
        let mut state = ViewState::new(&data, LensMode::None, TimeScale::Gar, false);
        let _ = state.handle_key(KeyCode::Char('v'), KeyModifiers::NONE, &data);

        let rendered = buffer_text(&render_test_frame(&mut state, &data, 80, 6));
        assert!(rendered.contains("terminal too short for pinned view"));
        assert!(!rendered.contains("pinned 0x"));
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
        assert_eq!(joined, " empty input   size 0   position 0% ");
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
        assert!(joined.contains("size 800"));
        assert!(joined.contains("position 5%"));
        assert!(joined.contains("lens —"));
    }

    #[test]
    fn status_line_shows_cuneiform_offset_when_cuneiform_lens_active() {
        let data = vec![0_u8; 80];
        let mut s = ViewState::new(&data, LensMode::Cuneiform, TimeScale::Gar, false);
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

    #[test]
    fn m_enters_bookmark_set_mode() {
        let mut s = state(80);
        let _ = s.handle_key(KeyCode::Char('m'), KeyModifiers::NONE, b"");
        assert!(matches!(s.mode, Mode::BookmarkSet));
    }

    #[test]
    fn apostrophe_enters_bookmark_jump_mode() {
        let mut s = state(80);
        let _ = s.handle_key(KeyCode::Char('\''), KeyModifiers::NONE, b"");
        assert!(matches!(s.mode, Mode::BookmarkJump));
    }

    #[test]
    fn bookmark_set_then_jump_restores_cursor() {
        let mut s = state(80);
        s.cursor = Some(42);
        let _ = s.handle_key(KeyCode::Char('m'), KeyModifiers::NONE, b"");
        let _ = s.handle_key(KeyCode::Char('a'), KeyModifiers::NONE, b"");
        assert_eq!(s.bookmarks[bookmark_idx('a').unwrap()], Some(42));
        // Move cursor away.
        s.cursor = Some(10);
        // Now jump back.
        let _ = s.handle_key(KeyCode::Char('\''), KeyModifiers::NONE, b"");
        let _ = s.handle_key(KeyCode::Char('a'), KeyModifiers::NONE, b"");
        assert_eq!(s.cursor, Some(42));
        assert!(matches!(s.mode, Mode::Normal));
    }

    #[test]
    fn bookmark_set_slot_is_case_insensitive() {
        let mut s = state(80);
        s.cursor = Some(7);
        let _ = s.handle_key(KeyCode::Char('m'), KeyModifiers::NONE, b"");
        let _ = s.handle_key(KeyCode::Char('Z'), KeyModifiers::NONE, b"");
        assert_eq!(s.bookmarks[bookmark_idx('z').unwrap()], Some(7));
        assert_eq!(s.bookmarks[bookmark_idx('Z').unwrap()], Some(7));
    }

    #[test]
    fn bookmark_jump_to_unset_slot_reports_message() {
        let mut s = state(80);
        let _ = s.handle_key(KeyCode::Char('\''), KeyModifiers::NONE, b"");
        let _ = s.handle_key(KeyCode::Char('x'), KeyModifiers::NONE, b"");
        assert_eq!(s.cursor, Some(0)); // unchanged
        let msg = s.status_message.as_deref().unwrap_or("");
        assert!(msg.contains("'x'"), "got {msg:?}");
    }

    #[test]
    fn bookmark_set_with_digit_is_rejected() {
        let mut s = state(80);
        let _ = s.handle_key(KeyCode::Char('m'), KeyModifiers::NONE, b"");
        let _ = s.handle_key(KeyCode::Char('5'), KeyModifiers::NONE, b"");
        assert!(s.bookmarks.iter().all(Option::is_none));
        assert!(
            s.status_message
                .as_deref()
                .unwrap()
                .contains("bookmarks use a-z")
        );
    }

    #[test]
    fn bookmark_set_esc_cancels_without_changing_state() {
        let mut s = state(80);
        s.cursor = Some(17);
        let _ = s.handle_key(KeyCode::Char('m'), KeyModifiers::NONE, b"");
        let _ = s.handle_key(KeyCode::Esc, KeyModifiers::NONE, b"");
        assert!(s.bookmarks.iter().all(Option::is_none));
        assert!(matches!(s.mode, Mode::Normal));
    }

    #[test]
    fn bracket_enters_semantic_jump_mode() {
        let mut s = state(80);
        let _ = s.handle_key(KeyCode::Char(']'), KeyModifiers::NONE, b"");
        assert!(matches!(s.mode, Mode::SemanticJump { forward: true }));
        let _ = s.handle_key(KeyCode::Esc, KeyModifiers::NONE, b"");
        let _ = s.handle_key(KeyCode::Char('['), KeyModifiers::NONE, b"");
        assert!(matches!(s.mode, Mode::SemanticJump { forward: false }));
    }

    #[test]
    fn semantic_jump_p_lands_on_next_ascii_region() {
        // Construct data with a known ASCII run at offset 10.
        let mut data = vec![0_u8; 10];
        data.extend_from_slice(b"Hello, world!"); // 10..23, len 13 ≥ 4 → Ascii region
        data.extend_from_slice(&[0_u8; 10]);

        let mut s = ViewState::new_with_analysis(&data, LensMode::None, TimeScale::Gar, false);
        s.cursor = Some(0);

        let _ = s.handle_key(KeyCode::Char(']'), KeyModifiers::NONE, &data);
        let _ = s.handle_key(KeyCode::Char('p'), KeyModifiers::NONE, &data);

        assert_eq!(s.cursor, Some(10));
        assert!(matches!(s.mode, Mode::Normal));
        let msg = s.status_message.as_deref().unwrap_or("");
        assert!(msg.contains("printable"), "got {msg:?}");
    }

    #[test]
    fn semantic_jump_p_backward_finds_previous_ascii_region() {
        let mut data = vec![0_u8; 10];
        data.extend_from_slice(b"early string"); // offset 10
        data.extend_from_slice(&[0_u8; 10]);
        data.extend_from_slice(b"late string"); // offset 32 (22 bytes in)
        data.extend_from_slice(&[0_u8; 10]);

        let mut s = ViewState::new_with_analysis(&data, LensMode::None, TimeScale::Gar, false);
        s.cursor = Some(50);
        let _ = s.handle_key(KeyCode::Char('['), KeyModifiers::NONE, &data);
        let _ = s.handle_key(KeyCode::Char('p'), KeyModifiers::NONE, &data);
        // Should have stepped back to the most recent ascii start before 50.
        assert!(s.cursor.unwrap() < 50);
    }

    #[test]
    fn semantic_jump_with_no_match_reports_message() {
        let data = vec![0_u8; 10]; // no ASCII regions anywhere
        let mut s = ViewState::new_with_analysis(&data, LensMode::None, TimeScale::Gar, false);
        s.cursor = Some(0);

        let _ = s.handle_key(KeyCode::Char(']'), KeyModifiers::NONE, &data);
        let _ = s.handle_key(KeyCode::Char('p'), KeyModifiers::NONE, &data);
        // Cursor unchanged.
        assert_eq!(s.cursor, Some(0));
        let msg = s.status_message.as_deref().unwrap_or("");
        assert!(msg.contains("no next printable"), "got {msg:?}");
    }

    #[test]
    fn semantic_jump_unknown_letter_reports_error() {
        let mut s = state(80);
        let _ = s.handle_key(KeyCode::Char(']'), KeyModifiers::NONE, b"");
        let _ = s.handle_key(KeyCode::Char('x'), KeyModifiers::NONE, b"");
        assert!(matches!(s.mode, Mode::Normal));
        let msg = s.status_message.as_deref().unwrap_or("");
        assert!(msg.contains("unknown jump letter"), "got {msg:?}");
    }

    #[test]
    fn status_line_shows_semantic_jump_prompt() {
        let mut s = state(80);
        let _ = s.handle_key(KeyCode::Char(']'), KeyModifiers::NONE, b"");
        let line = s.status_line(0, 10);
        let joined: String = line
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect();
        assert!(joined.contains("jump next"));
        assert!(joined.contains("p/z/e"));
    }

    #[test]
    fn apply_persisted_restores_scroll_cursor_lens_and_bookmarks() {
        let mut s = state(8 * 100);
        let saved = PersistedState {
            scroll: 7,
            cursor: Some(42),
            lens_mode: LensMode::Cuneiform,
            bookmarks: vec![('a', 10), ('z', 500)],
        };
        s.apply_persisted(saved);
        assert_eq!(s.scroll, 7);
        assert_eq!(s.cursor, Some(42));
        assert_eq!(s.lens_mode, LensMode::Cuneiform);
        assert!(s.lens.is_some());
        assert_eq!(s.bookmarks[bookmark_idx('a').unwrap()], Some(10));
        assert_eq!(s.bookmarks[bookmark_idx('z').unwrap()], Some(500));
    }

    #[test]
    fn apply_persisted_clamps_out_of_range_offsets() {
        // File shrank between runs — saved offsets must be reined in so
        // the TUI doesn't land the cursor past the end.
        let mut s = state(16);
        let saved = PersistedState {
            scroll: 999,
            cursor: Some(999),
            lens_mode: LensMode::None,
            bookmarks: vec![('a', 5), ('b', 999)],
        };
        s.apply_persisted(saved);
        assert_eq!(s.cursor, Some(15));
        assert_eq!(s.scroll, 1);
        // Out-of-range bookmark is dropped silently.
        assert_eq!(s.bookmarks[bookmark_idx('a').unwrap()], Some(5));
        assert_eq!(s.bookmarks[bookmark_idx('b').unwrap()], None);
    }

    #[test]
    fn snapshot_captures_cursor_scroll_and_bookmarks() {
        let mut s = state(80);
        s.scroll = 3;
        s.cursor = Some(25);
        s.lens_mode = LensMode::Tablet;
        s.bookmarks[bookmark_idx('m').unwrap()] = Some(42);
        s.bookmarks[bookmark_idx('a').unwrap()] = Some(10);

        let snap = s.snapshot();
        assert_eq!(snap.scroll, 3);
        assert_eq!(snap.cursor, Some(25));
        assert_eq!(snap.lens_mode, LensMode::Tablet);
        // Sorted for diffability.
        assert_eq!(snap.bookmarks, vec![('a', 10), ('m', 42)]);
    }
}
