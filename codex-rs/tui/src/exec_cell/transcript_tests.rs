//! Grouped command details retain terminal input in raw output without exposing hidden reasoning.

use super::*;
use crate::exec_cell::CommandOutput;
use crate::exec_cell::model::ExecCall;
use crate::history_cell::HistoryCell;
use crate::history_cell::new_reasoning_summary_block;
use crate::history_cell::new_unified_exec_interaction;
use codex_app_server_protocol::CommandExecutionSource;
use pretty_assertions::assert_eq;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

fn completed_read(name: &str, output: &str) -> ExecCell {
    let command = vec!["cat".to_owned(), name.to_owned()];
    let parsed = codex_shell_command::parse_command::parse_command(&command);
    ExecCell::new(
        ExecCall {
            call_id: name.to_owned(),
            command,
            parsed,
            output: Some(CommandOutput::new(/*exit_code*/ 0, output.to_owned())),
            source: CommandExecutionSource::UnifiedExecStartup,
            start_time: None,
            duration: Some(Duration::from_millis(/*millis*/ 5)),
            interaction_input: None,
        },
        /*animations_enabled*/ false,
    )
}

#[test]
fn raw_grouped_history_retains_terminal_input_and_omits_reasoning() {
    let mut group = completed_read("first.txt", "first output");
    group
        .group
        .push_detail(Arc::from(new_reasoning_summary_block(
            vec!["Inspecting the next file".to_owned()],
            Path::new("."),
        ) as Box<dyn HistoryCell>));
    group
        .group
        .push_detail(Arc::new(new_unified_exec_interaction(
            Some("cat first.txt".to_owned()),
            "continue\n".to_owned(),
        )));
    group
        .append_completed(completed_read("second.txt", "second output"))
        .expect("adjacent reads stay grouped");

    let raw = group
        .raw_lines()
        .iter()
        .map(Line::to_string)
        .collect::<Vec<_>>();
    let rich = group
        .transcript_lines(/*width*/ 80)
        .iter()
        .map(Line::to_string)
        .collect::<Vec<_>>()
        .join("\n");
    insta::assert_snapshot!(format!("Rich\n{rich}\n\nRaw\n{}", raw.join("\n")));

    let mut earlier = completed_read("earlier.txt", "earlier output");
    earlier
        .group
        .push_detail(Arc::from(new_reasoning_summary_block(
            vec!["Inspect the first file".to_owned()],
            Path::new("."),
        ) as Box<dyn HistoryCell>));
    earlier
        .group
        .push_detail(Arc::new(new_unified_exec_interaction(
            Some("cat earlier.txt".to_owned()),
            "next\n".to_owned(),
        )));
    let mut expected_raw = earlier.raw_lines();
    expected_raw.push(Line::from(""));
    expected_raw.extend(group.raw_lines());
    let mut expected_rich = earlier.transcript_hyperlink_lines(/*width*/ 80);
    expected_rich.push(HyperlinkLine::from(""));
    expected_rich.extend(group.transcript_hyperlink_lines(/*width*/ 80));
    group.prepend(earlier);
    assert_eq!(group.raw_lines(), expected_raw);
    assert_eq!(
        group.transcript_hyperlink_lines(/*width*/ 80),
        expected_rich
    );
}

#[test]
fn resumed_partial_tool_output_draws_without_an_escape() {
    let output = "\
before\x1b]133;A\x1b\\kept\x1b[?2026h\x1b[?1049h\x1b[31m!\x1b[0m\x1b]133;A\x1b";
    let cell = ExecCell::new(
        ExecCall {
            call_id: "call-1".to_string(),
            command: vec!["echo".to_string(), "partial".to_string()],
            parsed: Vec::new(),
            output: Some(CommandOutput::new(/*exit_code*/ 0, output.to_string())),
            source: CommandExecutionSource::UnifiedExecStartup,
            start_time: None,
            duration: Some(Duration::from_millis(/*millis*/ 5)),
            interaction_input: None,
        },
        /*animations_enabled*/ false,
    );

    let display = draw_lines(cell.display_lines(/*width*/ 80));
    assert!(display.contains("kept"), "{display:?}");
    assert!(
        !display.as_bytes().contains(&0x1b),
        "display buffer leaked an escape: {display:?}"
    );

    let raw = draw_lines(cell.raw_lines());
    assert!(raw.contains("kept"), "{raw:?}");
    assert!(
        !raw.as_bytes().contains(&0x1b),
        "raw buffer leaked an escape: {raw:?}"
    );
}

fn draw_lines(lines: Vec<Line<'static>>) -> String {
    let width = 80u16;
    let height = u16::try_from(lines.len().max(1)).expect("line count fits");
    let area = Rect::new(0, 0, width, height);
    let mut buffer = Buffer::empty(area);
    Paragraph::new(lines).render(area, &mut buffer);
    let mut rendered = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            let symbol = buffer[(x, y)].symbol();
            assert!(
                !symbol.as_bytes().contains(&0x1b),
                "cell ({x}, {y}) contains an escape: {symbol:?}"
            );
            rendered.push_str(symbol);
        }
        rendered.push('\n');
    }
    rendered
}
