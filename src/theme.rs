use floem::{
    peniko::Color,
    style::{CursorColor, Style},
    views::editor::{
        CurrentLineColor, IndentGuideColor, PreeditUnderlineColor, SelectionColor,
        VisibleWhitespaceColor,
        gutter::{DimColor, GutterClass},
        view::EditorViewClass,
    },
};

pub(crate) fn editor_theme_style() -> Style {
    let fg = Color::from_rgb8(0x38, 0x3A, 0x42);
    let bg = Color::from_rgb8(0xFA, 0xFA, 0xFA);
    let grey = Color::from_rgb8(0xE5, 0xE5, 0xE6);
    let dim = Color::from_rgb8(0xA0, 0xA1, 0xA7);
    let cursor = Color::from_rgb8(0x52, 0x6F, 0xFF);
    let current_line = Color::from_rgb8(0xF2, 0xF2, 0xF2);

    Style::new()
        .color(fg)
        .background(bg)
        .class(GutterClass, move |s| {
            s.background(bg)
                .set(DimColor, Some(dim))
                .set(CurrentLineColor, current_line)
        })
        .class(EditorViewClass, move |s| {
            s.set(CursorColor, cursor)
                .set(SelectionColor, grey)
                .set(CurrentLineColor, current_line)
                .set(VisibleWhitespaceColor, grey)
                .set(PreeditUnderlineColor, fg)
                .set(IndentGuideColor, grey)
        })
}
