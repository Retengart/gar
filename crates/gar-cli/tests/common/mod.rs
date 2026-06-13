//! Shared spawner, fixture factories, matrix-iteration enum, and
//! roundtrip-assertion helper for the gar-cli integration tests.
//!
//! This is the ONLY file under `crates/gar-cli/tests/` allowed to call
//! `assert_cmd::Command::cargo_bin` or `std::process::Command::new` for
//! the gar binary. Enforcement lives in the
//! `crates/xtask/tests/spawn_discipline.rs` static gate (Phase 3 D-16).
//!
//! Visibility note: every item is `pub` rather than `pub(crate)` because
//! integration-test files (`tests/*.rs`) are each compiled as their own
//! crate, with `mod common;` pulling this file in as a *private* module
//! inside that synthetic crate. `clippy::redundant_pub_crate` flags
//! `pub(crate)` in that situation as meaningless — `pub` is the correct
//! way to expose items from a test helper module.

// `dead_code`: each test file pulls only a subset of helpers; a full
// dead-code sweep would be noisy.
// `unreachable_pub`: integration tests compile `tests/common/mod.rs` as
// a private module inside a synthetic per-test crate, so `pub` items
// are never "reachable" from crate-outside callers — but `pub(crate)`
// trips `clippy::redundant_pub_crate` (private-module siblings). The
// only way to satisfy both lints is to use `pub` and silence the rustc
// warning at file scope.
#![allow(dead_code, unreachable_pub, reason = "test infrastructure")]

use assert_cmd::Command;
use gar_core::lens::TimeScale;

/// Build a hermetic `gar` command: cleared env, only the minimum
/// restored so the child process can start on every CI cell.
///
/// On Windows, `CreateProcess` requires `SystemRoot` and (for some DLL
/// loader paths) `USERPROFILE`; on Unix a clean `PATH` is enough.
/// Restoring only what's set avoids injecting empty variables that some
/// libc builds treat differently from "unset". `NO_COLOR` is NOT set
/// here — callers pass `--color=never` explicitly so a repo-wide grep
/// for `--color=` catches every test's colour intent.
pub fn gar_cmd() -> Command {
    let mut cmd = Command::cargo_bin("gar").expect("binary built by cargo");
    cmd.env_clear();
    if let Some(path) = std::env::var_os("PATH") {
        cmd.env("PATH", path);
    }
    #[cfg(windows)]
    {
        if let Some(root) = std::env::var_os("SystemRoot") {
            cmd.env("SystemRoot", root);
        }
        if let Some(profile) = std::env::var_os("USERPROFILE") {
            cmd.env("USERPROFILE", profile);
        }
    }
    cmd
}

// ---------------------------------------------------------------------
// Fixture factories — every fixture generated in-test, ≤ 4 KiB, no
// `include_bytes!` (Pitfall 7). CRCs for minimal_png are pre-computed
// via Python `zlib.crc32` and cited in PHASE 3 RESEARCH §Fixture
// Factories.
// ---------------------------------------------------------------------

pub mod fixtures {
    /// `b"Hello, world!\n"` — 14 bytes (14 % 8 == 6, exercises the
    /// short-tail padding path).
    pub fn hello_world() -> Vec<u8> {
        b"Hello, world!\n".to_vec()
    }

    /// 1 KiB of zero bytes — 128 full 8-byte chunks, zero entropy.
    pub fn zero_fill_1kib() -> Vec<u8> {
        vec![0_u8; 1024]
    }

    /// Minimum structurally valid 1×1 grayscale PNG — 45 bytes.
    /// IHDR CRC `0x3A7E9B55`, IEND CRC `0xAE426082` — both pre-computed
    /// via Python `zlib.crc32` (see `RESEARCH` §`minimal_png` verification).
    pub fn minimal_png() -> Vec<u8> {
        let mut out = Vec::with_capacity(45);
        // 8-byte PNG signature.
        out.extend_from_slice(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
        // IHDR length (13) + "IHDR" + 13 bytes of data + CRC.
        out.extend_from_slice(&[0x00, 0x00, 0x00, 0x0D]);
        out.extend_from_slice(b"IHDR");
        out.extend_from_slice(&[
            0x00, 0x00, 0x00, 0x01, // width  = 1
            0x00, 0x00, 0x00, 0x01, // height = 1
            0x08, // bit depth
            0x00, // color type (grayscale)
            0x00, // compression
            0x00, // filter
            0x00, // interlace
        ]);
        out.extend_from_slice(&0x3A7E_9B55_u32.to_be_bytes());
        // IEND length (0) + "IEND" + CRC.
        out.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
        out.extend_from_slice(b"IEND");
        out.extend_from_slice(&0xAE42_6082_u32.to_be_bytes());
        debug_assert_eq!(out.len(), 45);
        out
    }

    /// Minimum structurally valid empty ZIP — 22-byte End-of-Central-
    /// Directory record (PKWARE APPNOTE 6.3.9 §4.3.16).
    pub fn minimal_zip() -> Vec<u8> {
        vec![
            0x50, 0x4B, 0x05, 0x06, // EOCD signature "PK\x05\x06"
            0x00, 0x00, // disk number
            0x00, 0x00, // disk where CD starts
            0x00, 0x00, // CD entries on this disk
            0x00, 0x00, // CD entries total
            0x00, 0x00, 0x00, 0x00, // CD size
            0x00, 0x00, 0x00, 0x00, // CD offset
            0x00, 0x00, // comment length
        ]
    }

    /// 128-byte ELF64 header + zero-padded single program-header slot.
    /// Structurally recognisable per System V ABI §4; `gar dump`
    /// never parses ELF, so this is purely a realistic 128-byte
    /// fixture (`RESEARCH` §`minimal_elf`, Assumption A3).
    pub fn minimal_elf() -> Vec<u8> {
        let mut out = Vec::with_capacity(128);
        // e_ident (16 bytes).
        out.extend_from_slice(&[
            0x7F, b'E', b'L', b'F', // ELF magic
            2,    // EI_CLASS   = ELFCLASS64
            1,    // EI_DATA    = ELFDATA2LSB (little-endian)
            1,    // EI_VERSION = EV_CURRENT
            0,    // EI_OSABI   = ELFOSABI_SYSV
            0, 0, 0, 0, 0, 0, 0, 0, // EI_PAD
        ]);
        out.extend_from_slice(&0x0002_u16.to_le_bytes()); // e_type     = ET_EXEC
        out.extend_from_slice(&0x003E_u16.to_le_bytes()); // e_machine  = EM_X86_64
        out.extend_from_slice(&0x0000_0001_u32.to_le_bytes()); // e_version
        out.extend_from_slice(&0_u64.to_le_bytes()); // e_entry
        out.extend_from_slice(&0x40_u64.to_le_bytes()); // e_phoff
        out.extend_from_slice(&0_u64.to_le_bytes()); // e_shoff
        out.extend_from_slice(&0_u32.to_le_bytes()); // e_flags
        out.extend_from_slice(&0x0040_u16.to_le_bytes()); // e_ehsize
        out.extend_from_slice(&0x0038_u16.to_le_bytes()); // e_phentsize
        out.extend_from_slice(&0x0001_u16.to_le_bytes()); // e_phnum = 1
        out.extend_from_slice(&0_u16.to_le_bytes()); // e_shentsize
        out.extend_from_slice(&0_u16.to_le_bytes()); // e_shnum
        out.extend_from_slice(&0_u16.to_le_bytes()); // e_shstrndx
        out.resize(128, 0);
        debug_assert_eq!(out.len(), 128);
        out
    }
}

// ---------------------------------------------------------------------
// Matrix iteration: expand LensMode::Time across the three TimeScales so
// the 7-row LensConfig slice hits every distinct CLI flag payload
// (D-02, D-15).
// ---------------------------------------------------------------------

/// Lens × time-scale combinations exercised by the roundtrip matrix.
#[derive(Copy, Clone, Debug)]
pub enum LensConfig {
    None,
    Time(TimeScale),
    Angle,
    Tablet,
    Cuneiform,
}

impl LensConfig {
    /// CLI flags this config produces, in invocation order.
    pub fn cli_args(self) -> Vec<&'static str> {
        match self {
            Self::None => vec!["--lens=none"],
            Self::Time(TimeScale::Gar) => vec!["--lens=time", "--time-scale=gar"],
            Self::Time(TimeScale::Sec) => vec!["--lens=time", "--time-scale=sec"],
            Self::Time(TimeScale::Ms) => vec!["--lens=time", "--time-scale=ms"],
            Self::Angle => vec!["--lens=angle"],
            Self::Tablet => vec!["--lens=tablet"],
            Self::Cuneiform => vec!["--lens=cuneiform"],
        }
    }

    /// Diagnostic label for failure messages.
    pub const fn label(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Time(TimeScale::Gar) => "Time(Gar)",
            Self::Time(TimeScale::Sec) => "Time(Sec)",
            Self::Time(TimeScale::Ms) => "Time(Ms)",
            Self::Angle => "Angle",
            Self::Tablet => "Tablet",
            Self::Cuneiform => "Cuneiform",
        }
    }
}

pub const ALL_LENS_CONFIGS: &[LensConfig] = &[
    LensConfig::None,
    LensConfig::Time(TimeScale::Gar),
    LensConfig::Time(TimeScale::Sec),
    LensConfig::Time(TimeScale::Ms),
    LensConfig::Angle,
    LensConfig::Tablet,
    LensConfig::Cuneiform,
];

/// `(label, factory)` entry in the roundtrip fixture slice. Factored
/// into a named alias because `clippy::type_complexity` flags the
/// inline form `&[(&str, fn() -> Vec<u8>)]`.
pub type FixtureEntry = (&'static str, fn() -> Vec<u8>);

/// Every fixture covered by the full roundtrip matrix.
///
/// Phase 4 REF-04 restored full-matrix coverage: length-preserving
/// `decode` + JSON/HTML decode paths mean every fixture (including
/// short-tail `hello_world` 14 B / `minimal_png` 45 B / `minimal_zip`
/// 22 B) roundtrips byte-identically.
pub const ALL_FIXTURES: &[FixtureEntry] = &[
    ("minimal_elf", fixtures::minimal_elf),
    ("zero_fill_1kib", fixtures::zero_fill_1kib),
    ("hello_world", fixtures::hello_world),
    ("minimal_png", fixtures::minimal_png),
    ("minimal_zip", fixtures::minimal_zip),
];

/// Every output format `dump` emits — all four now roundtrip under
/// REF-04 (length trailer + JSON/HTML decoders).
pub const ROUNDTRIP_FORMATS: &[gar::Format] = gar::Format::ALL;

// ---------------------------------------------------------------------
// Roundtrip assertion helper (D-14.3, D-20): on mismatch, print cell
// identity + first divergence index + ±8-byte hex windows on both
// sides.
// ---------------------------------------------------------------------

/// Compare decoded output to the original fixture. On mismatch, panics
/// with a readable diagnostic naming the failing cell.
pub fn assert_roundtrip(original: &[u8], decoded: &[u8], cell_label: &str) {
    if original == decoded {
        return;
    }
    let diverge = original
        .iter()
        .zip(decoded.iter())
        .position(|(a, b)| a != b)
        .unwrap_or_else(|| original.len().min(decoded.len()));
    let orig_window = hex_window(original, diverge);
    let dec_window = hex_window(decoded, diverge);
    panic!(
        "cell: {cell_label}\n\
         original_len={} decoded_len={}\n\
         first diverge at byte {diverge}\n\
         original ±8: {orig_window}\n\
         decoded  ±8: {dec_window}",
        original.len(),
        decoded.len(),
    );
}

fn hex_window(bytes: &[u8], center: usize) -> String {
    let lo = center.saturating_sub(8);
    let hi = (center + 8).min(bytes.len());
    bytes[lo..hi]
        .iter()
        .enumerate()
        .map(|(i, b)| {
            let abs = lo + i;
            if abs == center {
                format!("[{b:02x}]")
            } else {
                format!("{b:02x}")
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

// ---------------------------------------------------------------------
// BrokenPipe helper — Plan 03 consumes this from `tests/cli.rs`.
// assert_cmd does NOT naturally close stdout on the child; raw
// std::process::Command + drop(child.stdout.take()) is the minimum-dep
// way to trigger EPIPE. This helper MUST live under `tests/common/`
// because the spawn-discipline gate excludes `common/` (D-16, D-17).
// ---------------------------------------------------------------------

/// Spawn `gar` with the given args + stdin, then drop the child's
/// stdout handle immediately to force `BrokenPipe` on the writer side.
/// Returns the child's exit status.
///
/// Windows maps `ERROR_BROKEN_PIPE` (109) to `ErrorKind::BrokenPipe`
/// identically to Unix's `EPIPE` (32), so this helper works on all
/// three CI OSes (`RESEARCH` Assumption A7).
pub fn spawn_with_closed_stdout(args: &[&str], stdin_bytes: &[u8]) -> std::process::ExitStatus {
    use std::io::Write;
    use std::process::{Command as StdCommand, Stdio};

    let bin = env!("CARGO_BIN_EXE_gar");
    let mut child = StdCommand::new(bin)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn gar");
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(stdin_bytes).ok();
        // Drop `stdin` to close the write end so the child reaches EOF.
    }
    // Close stdout immediately so the child's next write gets EPIPE.
    drop(child.stdout.take());
    child.wait().expect("wait gar")
}

// ---------------------------------------------------------------------
// TUI drive helper (Plan 04-04). Shared by `tests/tui.rs` and
// `tests/persist.rs`: spins up an in-process 80×24 TestBackend, pushes
// the canonical drive sequence (`j j j j j m a q` — 5 cursor-downs,
// bookmark slot 'a', quit), and returns when the TUI saves state and
// exits cleanly.
//
// Callers OWN env mutation (XDG_STATE_HOME / HOME) under
// `#[serial(env)]` and the corresponding `unsafe { env::set_var /
// remove_var }` pairs (Rust 2024 + Phase 2 D-07 + Pitfall 5).
// ---------------------------------------------------------------------

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use std::path::Path;

/// Drive the TUI through `j j j j j m a q` against the given fixture
/// file, returning after the TUI cleanly exits.
///
/// # Panics
///
/// Panics if `Terminal::new` or `run_with_terminal` returns an error —
/// both indicate a test-environment failure, not a production bug.
pub fn drive_tui_to_quit_with_fixture(fixture_bytes: &[u8], fixture_path: &Path) {
    let events: Vec<Event> = vec![
        key('j'),
        key('j'),
        key('j'),
        key('j'),
        key('j'),
        key('m'),
        key('a'),
        key('q'),
    ];
    let mut iter = events.into_iter();

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal init");

    gar::__test_hooks::run_with_terminal(
        &mut terminal,
        fixture_bytes,
        0, // base_offset
        gar::LensMode::None,
        gar::__TuiTimeScale::Gar, // re-exported TimeScale
        false,                       // purist
        Some(fixture_path),
        || Ok(iter.next()),
    )
    .expect("tui run");
}

const fn key(c: char) -> Event {
    Event::Key(KeyEvent {
        code: KeyCode::Char(c),
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    })
}
