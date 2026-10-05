use super::UnifiedExecProcessDetails;
use super::UnifiedExecProcessesCell;
use crate::history_cell::HistoryCell;
use ratatui::text::Line;

#[test]
fn background_terminal_chunks_drop_partial_controls() {
    let cell = UnifiedExecProcessesCell::new(vec![UnifiedExecProcessDetails {
        command_display: "echo".to_string(),
        recent_chunks: vec!["\x1b]133;A\x1b\\kept\x1b[?2026h\x1b[?1049h\x1b".to_string()],
    }]);

    let rendered = cell
        .display_lines(/*width*/ 80)
        .into_iter()
        .map(|line| line.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(rendered.contains("kept"), "{rendered}");
    assert!(
        !rendered.as_bytes().contains(&0x1b),
        "display leaked an escape: {rendered:?}"
    );

    let raw = cell
        .raw_lines()
        .into_iter()
        .map(|line: Line<'_>| line.to_string())
        .collect::<String>();
    assert!(raw.contains("kept"), "{raw}");
    assert!(
        !raw.as_bytes().contains(&0x1b),
        "raw leaked an escape: {raw:?}"
    );
}
