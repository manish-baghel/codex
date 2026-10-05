# oai-codex-ansi-escape

Small helper functions that wrap functionality from
<https://crates.io/crates/ansi-to-tui>:

```rust
pub fn ansi_escape_line(s: &str) -> Line<'static>
pub fn ansi_escape<'a>(s: &'a str) -> Text<'a>
```

Advantages:

- `ansi_to_tui::IntoText` is not in scope for the entire TUI crate
- non-SGR terminal controls are stripped before parsing, and a parser error
  falls back to the plain text instead of panicking
