//! Core building blocks shared by the `base60` CLI and any downstream
//! library consumer.
//!
//! The crate exposes three layers:
//!
//! 1. **Numeric conversion** — [`u64_to_base60`] and the associated
//!    constant [`DIGITS`]. Eleven base-60 digits cover the full `u64`
//!    range (`60^11 ≈ 3.65 · 10¹⁹ > u64::MAX`).
//! 2. **Sumero-Babylonian lenses** — a [`Lens`] trait plus four
//!    implementations ([`TimeLens`], [`AngleLens`], [`TabletLens`],
//!    [`CuneiformLens`]) that reinterpret a raw `u64` as a piece of
//!    the systems that drove the original sexagesimal notation.
//! 3. **URL-safe encoding** — [`encode_u64`] / [`decode_u64`] map a
//!    `u64` to an 11-char string using an unambiguous subset of
//!    alphanumerics (`0-9A-Za-x`), useful for shorter-than-hex hash
//!    prefixes in URLs or identifiers.
//!
//! The crate uses `std` for [`std::sync::LazyLock`] in the cuneiform
//! glyph table and for [`String`] allocation in the lens renderers.

pub mod convert;
pub mod cuneiform;
pub mod lens;
pub mod url;

pub use convert::{ASCII_STR, DIGIT_PAIRS_STR, DIGITS, u64_to_base60};
pub use cuneiform::{ascii_fallback_forced, ascii_pair, glyph};
pub use lens::{AngleLens, CuneiformLens, Lens, TabletLens, TimeLens, TimeScale};
pub use url::{DecodeError, decode_u64, encode_u64};
