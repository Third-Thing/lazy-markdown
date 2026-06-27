use gpui::{Context, EntityInputHandler, Window};

use crate::{AddBlockQuote, RemoveBlockQuote, window::AppWindow};

impl AppWindow {
    pub(crate) fn on_add_block_quote(
        &mut self,
        _: &AddBlockQuote,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.transform_selected_lines(add_block_quote, window, cx);
    }

    pub(crate) fn on_remove_block_quote(
        &mut self,
        _: &RemoveBlockQuote,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.transform_selected_lines(remove_block_quote, window, cx);
    }

    /// Apply `transform` to every line touched by the active editor's selection.
    ///
    /// The selection is expanded to whole lines, the spanning text is rewritten,
    /// and the rewrite is applied through the input handler so it joins the undo
    /// history and emits a change event for dirty tracking.
    fn transform_selected_lines(
        &mut self,
        transform: impl Fn(&str) -> String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(editor) = self.active_editor() else {
            return;
        };

        editor.update(cx, |state, cx| {
            let value = state.value();
            let text: &str = value.as_ref();
            let selection = state.selected_range();

            let line_start = text[..selection.start]
                .rfind('\n')
                .map_or(0, |index| index + 1);
            let line_end = text[selection.end..]
                .find('\n')
                .map_or(text.len(), |index| selection.end + index);

            let block = &text[line_start..line_end];
            let replacement = transform(block);
            if replacement == block {
                return;
            }

            // `replace_text_in_range` expects a UTF-16 range, but selection and
            // line offsets are byte offsets into the rope.
            let range_utf16 = utf16_len(&text[..line_start])..utf16_len(&text[..line_end]);
            state.replace_text_in_range(Some(range_utf16), &replacement, window, cx);
        });
    }
}

/// Add one level of block-quote nesting to every line in `block`.
///
/// A plain line becomes `> line`; an already-quoted `> line` becomes
/// `> > line`, deepening the nesting by one.
fn add_block_quote(block: &str) -> String {
    transform_lines(block, |line| format!("> {line}"))
}

/// Remove one level of block-quote nesting from every line in `block`.
///
/// `> > line` becomes `> line` and `> line` becomes `line`. Lines without a
/// block-quote marker are returned unchanged.
fn remove_block_quote(block: &str) -> String {
    transform_lines(block, strip_one_quote_level)
}

fn transform_lines(block: &str, transform: impl Fn(&str) -> String) -> String {
    block
        .split('\n')
        .map(transform)
        .collect::<Vec<_>>()
        .join("\n")
}

fn strip_one_quote_level(line: &str) -> String {
    // CommonMark allows up to three spaces of indentation before the marker;
    // beyond that the line is an indented code block, not a block quote.
    let after_indent = line.trim_start_matches(' ');
    let indent = line.len() - after_indent.len();
    if indent > 3 {
        return line.to_string();
    }

    match after_indent.strip_prefix('>') {
        // Drop the marker and at most one following space, keeping any
        // CommonMark-permitted indentation in front of it.
        Some(rest) => format!("{}{}", &line[..indent], rest.strip_prefix(' ').unwrap_or(rest)),
        None => line.to_string(),
    }
}

fn utf16_len(text: &str) -> usize {
    text.chars().map(char::len_utf16).sum()
}

#[cfg(test)]
mod tests {
    use super::{add_block_quote, remove_block_quote, utf16_len};

    #[test]
    fn add_quotes_a_plain_line() {
        assert_eq!(add_block_quote("hello"), "> hello");
    }

    #[test]
    fn add_deepens_existing_nesting() {
        assert_eq!(add_block_quote("> hello"), "> > hello");
        assert_eq!(add_block_quote("> > hello"), "> > > hello");
    }

    #[test]
    fn add_handles_multiple_lines() {
        assert_eq!(add_block_quote("one\ntwo"), "> one\n> two");
    }

    #[test]
    fn remove_strips_one_level() {
        assert_eq!(remove_block_quote("> hello"), "hello");
        assert_eq!(remove_block_quote("> > hello"), "> hello");
    }

    #[test]
    fn remove_handles_tight_markers() {
        assert_eq!(remove_block_quote(">hello"), "hello");
        assert_eq!(remove_block_quote(">> hello"), "> hello");
    }

    #[test]
    fn remove_leaves_unquoted_lines_unchanged() {
        assert_eq!(remove_block_quote("hello"), "hello");
        assert_eq!(remove_block_quote("    > code"), "    > code");
    }

    #[test]
    fn remove_clears_quoted_blank_lines() {
        assert_eq!(remove_block_quote(">"), "");
        assert_eq!(remove_block_quote("> "), "");
    }

    #[test]
    fn add_then_remove_round_trips() {
        for line in ["plain", "> quoted", "> > deep", ""] {
            assert_eq!(remove_block_quote(&add_block_quote(line)), line);
        }
    }

    #[test]
    fn utf16_len_counts_surrogate_pairs() {
        // Astral-plane characters are two UTF-16 code units each.
        assert_eq!(utf16_len("a😀b"), 4);
    }
}
