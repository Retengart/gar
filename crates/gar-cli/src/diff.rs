//! Side-by-side base-60 comparison for two byte slices.

use crate::chunk::{CHUNK, is_printable, pad_chunk};
use crate::color::Palette;
use gar_core::format_chunk;
use std::io::{self, Write};

const SIDE_WIDTH: usize = 44;
const DOT: u8 = b'.';

/// Write every eight-byte row from `old` and `new` and report whether they differ.
pub(crate) fn write_diff<W: Write>(
    old: &[u8],
    new: &[u8],
    base_offset: u64,
    mut out: W,
    palette: &Palette,
) -> io::Result<bool> {
    let rows = old.len().max(new.len()).div_ceil(CHUNK);
    let mut different = false;

    for row in 0..rows {
        let start = row * CHUNK;
        let left = old
            .get(start..old.len().min(start + CHUNK))
            .filter(|bytes| !bytes.is_empty());
        let right = new
            .get(start..new.len().min(start + CHUNK))
            .filter(|bytes| !bytes.is_empty());
        let marker = match (left, right) {
            (Some(a), Some(b)) if a == b => '=',
            (Some(_), Some(_)) => '!',
            (Some(_), None) => '<',
            (None, Some(_)) => '>',
            (None, None) => continue,
        };
        different |= marker != '=';

        let row_offset = u64::try_from(row).unwrap_or(u64::MAX).saturating_mul(8);
        write!(
            out,
            "{:08x} {marker} ",
            base_offset.saturating_add(row_offset)
        )?;
        write_side(&mut out, left, right, palette)?;
        out.write_all(b"  ||  ")?;
        write_side(&mut out, right, left, palette)?;
        out.write_all(b"\n")?;
    }

    out.flush()?;
    Ok(different)
}

fn write_side<W: Write>(
    out: &mut W,
    side: Option<&[u8]>,
    other: Option<&[u8]>,
    palette: &Palette,
) -> io::Result<()> {
    let Some(bytes) = side else {
        return write!(out, "{:SIDE_WIDTH$}", "");
    };

    let value = u64::from_be_bytes(pad_chunk(bytes));
    let changed = other.is_none_or(|candidate| candidate != bytes);
    if changed {
        out.write_all(palette.changed.as_bytes())?;
    }
    out.write_all(format_chunk(value).as_bytes())?;
    if changed {
        out.write_all(palette.reset.as_bytes())?;
    }
    out.write_all(b"  |")?;

    for index in 0..CHUNK {
        let Some(&byte) = bytes.get(index) else {
            out.write_all(b" ")?;
            continue;
        };
        let byte_changed = other.and_then(|candidate| candidate.get(index)) != Some(&byte);
        if byte_changed {
            out.write_all(palette.changed.as_bytes())?;
        }
        out.write_all(&[if is_printable(byte) { byte } else { DOT }])?;
        if byte_changed {
            out.write_all(palette.reset.as_bytes())?;
        }
    }
    out.write_all(b"|")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::PALETTE_NONE;

    #[test]
    fn empty_inputs_are_equal_and_silent() {
        let mut output = Vec::new();
        let differs = write_diff(&[], &[], 0, &mut output, &PALETTE_NONE).expect("write diff");
        assert!(!differs);
        assert!(output.is_empty());
    }

    #[test]
    fn base_offset_is_applied_to_each_row() {
        let mut output = Vec::new();
        write_diff(
            b"12345678abcdefgh",
            b"12345678abcdefgh",
            16,
            &mut output,
            &PALETTE_NONE,
        )
        .expect("write diff");
        let rendered = String::from_utf8(output).expect("ASCII output");
        assert!(rendered.contains("00000010 ="));
        assert!(rendered.contains("00000018 ="));
    }
}
