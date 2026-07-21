//! Shared grammar tokens for gar's HTML dump and decoder.
//!
//! The HTML format is intentionally hand-written. Keeping the structural
//! tokens here makes emitter and decoder changes compile together instead of
//! letting their accepted formats drift silently.

/// Classes used for base-60 digit spans, ordered from lowest to highest tier.
pub(crate) const DIGIT_CLASSES: [&str; 4] = ["d-zero", "d-low", "d-mid", "d-high"];

/// Prefix before an HTML span's class name.
pub(crate) const SPAN_OPEN: &str = "<span class=\"";

/// Closing tag for every emitted span.
pub(crate) const SPAN_CLOSE: &str = "</span>";

/// Full separator span between adjacent base-60 pairs.
pub(crate) const SEPARATOR: &str = "<span class=\"sep\">:</span>";

/// Prefix before the hexadecimal byte count in the HTML trailer.
pub(crate) const LENGTH_PREFIX: &str = "<!-- bytes=0x";

/// Suffix after the hexadecimal byte count in the HTML trailer.
pub(crate) const LENGTH_SUFFIX: &str = " -->";

/// Return the heat-map class for one base-60 digit.
#[must_use]
pub(crate) const fn digit_class(digit: u8) -> &'static str {
    match digit {
        0 => DIGIT_CLASSES[0],
        1..20 => DIGIT_CLASSES[1],
        20..40 => DIGIT_CLASSES[2],
        _ => DIGIT_CLASSES[3],
    }
}
