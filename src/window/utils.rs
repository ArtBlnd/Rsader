use iced::{
    advanced::graphics::text::Paragraph, widget::canvas::Text, Font, Rectangle, Renderer, Size,
};
use iced_core::text::{LineHeight, Paragraph as _};

pub fn estimate_text_rect(text: &Text) -> (f32, f32) {
    let iced_core_text = iced_core::text::Text {
        content: &text.content,
        bounds: Size::INFINITY,
        size: text.size,
        line_height: text.line_height,
        font: text.font,
        horizontal_alignment: text.horizontal_alignment,
        vertical_alignment: text.vertical_alignment,
        shaping: text.shaping,
    };

    let mut p = Paragraph::default();
    p.update(iced_core_text);
    let mut w: f32 = 0.0;
    let mut h: f32 = 0.0;
    for run in p.buffer().layout_runs() {
        for glyph in run.glyphs {
            w = w.max(glyph.x + glyph.x_offset);
            h = h.max(glyph.y_offset + text.size.0);
        }
    }
    (w, h)
}

pub fn estimate_text_rect2(text: &str, font: &'static str, size: f32) -> (f32, f32) {
    let iced_core_text = iced_core::text::Text {
        content: text,
        bounds: Size::INFINITY,
        size: size.into(),
        line_height: LineHeight::default(),
        font: Font::with_name(font),
        horizontal_alignment: iced::alignment::Horizontal::Left,
        vertical_alignment: iced::alignment::Vertical::Top,
        shaping: iced_core::text::Shaping::Basic,
    };

    let mut p = Paragraph::default();
    p.update(iced_core_text);
    let mut w: f32 = 0.0;
    let mut h: f32 = 0.0;
    for run in p.buffer().layout_runs() {
        for glyph in run.glyphs {
            w = w.max(glyph.x + glyph.x_offset);
            h = h.max(glyph.y_offset + size);
        }
    }
    (w, h)
}
