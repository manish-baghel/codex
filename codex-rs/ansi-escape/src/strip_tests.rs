use super::strip_terminal_controls;

fn assert_only_sgr_escapes(text: &str) {
    let bytes = text.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != 0x1b {
            assert!(
                bytes[index] >= b' ' || bytes[index] == b'\n' || bytes[index] == b'\t',
                "C0 control at {index}: {text:?}"
            );
            index += 1;
            continue;
        }
        assert_eq!(
            bytes.get(index + 1),
            Some(&b'['),
            "non-SGR escape in {text:?}"
        );
        index += 2;
        let mut closed = false;
        while index < bytes.len() {
            let byte = bytes[index];
            index += 1;
            if byte == b'm' {
                closed = true;
                break;
            }
            assert!(
                (0x20..=0x3f).contains(&byte),
                "SGR parameter {byte:#x} in {text:?}"
            );
        }
        assert!(closed, "unclosed SGR in {text:?}");
    }
}

#[test]
fn drops_osc_133_terminated_by_st_and_the_same_sequence_cut_before_st() {
    let complete = strip_terminal_controls("out\x1b]133;A\x1b\\put");
    assert_eq!(complete, "output");
    assert_only_sgr_escapes(&complete);

    let cut = strip_terminal_controls("out\x1b]133;A");
    assert_eq!(cut, "out");
    assert_only_sgr_escapes(&cut);

    let cut_at_st = strip_terminal_controls("out\x1b]133;A\x1b");
    assert_eq!(cut_at_st, "out");
    assert_only_sgr_escapes(&cut_at_st);
}

#[test]
fn drops_private_modes_and_a_bare_trailing_escape() {
    let sync = strip_terminal_controls("\x1b[?2026h");
    assert_eq!(sync, "");
    assert_only_sgr_escapes(&sync);

    let alternate = strip_terminal_controls("a\x1b[?1049hb");
    assert_eq!(alternate, "ab");
    assert_only_sgr_escapes(&alternate);

    let trailing = strip_terminal_controls("tail\x1b");
    assert_eq!(trailing, "tail");
    assert_only_sgr_escapes(&trailing);
}

#[test]
fn keeps_sgr_and_drops_a_cut_sequence_instead_of_completing_it() {
    let mixed = strip_terminal_controls("\x1b[31mred\x1b[0m\x1b[?2026h\x1b[31");
    assert_eq!(mixed, "\x1b[31mred\x1b[0m");
    assert_only_sgr_escapes(&mixed);

    let cut_osc_then_color = strip_terminal_controls("\x1b]133;A\x1b[32mgreen\x1b[0m");
    assert_eq!(cut_osc_then_color, "\x1b[32mgreen\x1b[0m");
    assert_only_sgr_escapes(&cut_osc_then_color);
}

#[test]
fn drops_other_string_sequences_kitty_keyboard_and_c0_controls() {
    let stripped = strip_terminal_controls(
        "a\x1bP+q\x1b\\b\x1b_apc\x07c\x1b^pm\x1b\\d\x1bXsos\x07e\x1b[>1u\r\n\tf\x00",
    );
    assert_eq!(stripped, "abcde\n\tf");
    assert_only_sgr_escapes(&stripped);
}

#[test]
fn leaves_plain_text_borrowed() {
    let text = "plain\tline\n";
    assert!(matches!(
        strip_terminal_controls(text),
        std::borrow::Cow::Borrowed(borrowed) if borrowed == text
    ));
}
