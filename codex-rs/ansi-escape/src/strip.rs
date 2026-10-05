//! Streaming removal of terminal controls from captured tool output.
//!
//! SGR (`ESC [ … m`) is preserved so color can still be parsed. Every other
//! escape is dropped, including a sequence that was cut off before its
//! terminator. Cut sequences are not completed.

use std::borrow::Cow;

/// Remove terminal controls from `input`.
///
/// Kept: SGR (`ESC [ … m`), `\n`, and `\t`.
/// Dropped: other CSI (private modes, kitty keyboard), OSC, DCS, APC, PM, SOS
/// (BEL or ST), a trailing `ESC`, and every other C0 control.
pub fn strip_terminal_controls(input: &str) -> Cow<'_, str> {
    strip(input, /*keep_sgr*/ true)
}

/// Strip controls and reuse `text`'s buffer when nothing was removed.
pub fn strip_terminal_controls_owned(text: Cow<'_, str>) -> String {
    match strip_terminal_controls(text.as_ref()) {
        Cow::Borrowed(_) => text.into_owned(),
        Cow::Owned(stripped) => stripped,
    }
}

/// Drop SGR as well. Used when the SGR parser rejects the stripped line, so the
/// fallback span cannot write an escape.
pub(crate) fn strip_all_terminal_controls(input: &str) -> String {
    match strip(input, /*keep_sgr*/ false) {
        Cow::Borrowed(text) => text.to_string(),
        Cow::Owned(text) => text,
    }
}

fn strip(input: &str, keep_sgr: bool) -> Cow<'_, str> {
    let bytes = input.as_bytes();
    if !bytes
        .iter()
        .any(|byte| *byte < b' ' && *byte != b'\n' && *byte != b'\t')
    {
        return Cow::Borrowed(input);
    }

    let mut out = String::with_capacity(input.len());
    let mut index = 0;
    let mut clean_start = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte == 0x1b {
            push_clean(input, clean_start, index, &mut out);
            index = consume_escape(input, index, &mut out, keep_sgr);
            clean_start = index;
            continue;
        }
        if byte < b' ' && byte != b'\n' && byte != b'\t' {
            push_clean(input, clean_start, index, &mut out);
            index += 1;
            clean_start = index;
            continue;
        }
        index += 1;
    }
    if out.is_empty() {
        return Cow::Borrowed(&input[clean_start..]);
    }
    push_clean(input, clean_start, bytes.len(), &mut out);
    Cow::Owned(out)
}

fn push_clean(input: &str, start: usize, end: usize, out: &mut String) {
    if start < end {
        out.push_str(&input[start..end]);
    }
}

fn consume_escape(input: &str, start: usize, out: &mut String, keep_sgr: bool) -> usize {
    let bytes = input.as_bytes();
    let Some(next) = bytes.get(start + 1).copied() else {
        return bytes.len();
    };
    match next {
        b'[' => consume_csi(input, start, out, keep_sgr),
        b']' | b'P' | b'X' | b'^' | b'_' => consume_string(bytes, start),
        b'\\' => start + 2,
        byte if is_esc_intermediate(byte) => consume_esc_intermediate(bytes, start),
        byte if is_esc_final(byte) => start + 2,
        _ => start + 1,
    }
}

fn consume_csi(input: &str, start: usize, out: &mut String, keep_sgr: bool) -> usize {
    let bytes = input.as_bytes();
    let mut index = start + 2;
    while index < bytes.len() {
        let byte = bytes[index];
        if is_csi_final(byte) {
            index += 1;
            if keep_sgr && byte == b'm' {
                push_clean(input, start, index, out);
            }
            return index;
        }
        if !is_csi_parameter(byte) {
            return index;
        }
        index += 1;
    }
    index
}

fn consume_string(bytes: &[u8], start: usize) -> usize {
    let mut index = start + 2;
    while index < bytes.len() {
        match bytes[index] {
            0x07 | 0x18 | 0x1a => return index + 1,
            0x1b => {
                if bytes.get(index + 1) == Some(&b'\\') {
                    return index + 2;
                }
                return index;
            }
            _ => index += 1,
        }
    }
    bytes.len()
}

fn consume_esc_intermediate(bytes: &[u8], start: usize) -> usize {
    let mut index = start + 2;
    while index < bytes.len() {
        let byte = bytes[index];
        if is_esc_intermediate(byte) {
            index += 1;
            continue;
        }
        if is_esc_final(byte) {
            return index + 1;
        }
        return index;
    }
    bytes.len()
}

fn is_csi_parameter(byte: u8) -> bool {
    (0x20..=0x3f).contains(&byte)
}

fn is_csi_final(byte: u8) -> bool {
    (0x40..=0x7e).contains(&byte)
}

fn is_esc_intermediate(byte: u8) -> bool {
    (0x20..=0x2f).contains(&byte)
}

fn is_esc_final(byte: u8) -> bool {
    (0x30..=0x7e).contains(&byte)
}

#[cfg(test)]
#[path = "strip_tests.rs"]
mod tests;
