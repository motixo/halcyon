//! Probe the pinned Iced paragraph API, not an unrelated shaping library.
use iced_graphics::core::{Font, Pixels, Size, alignment, font, text};
use iced_graphics::core::text::Paragraph as _;
use iced_graphics::text::Paragraph;

fn main() {
    println!("family,weight,style,text,width,font_faces,glyphs");
    for family in ["Noto Sans", "DejaVu Sans Mono", "Halcyon Missing Probe Font"] {
        for (weight_name, weight, style_name, style) in [
            ("regular", font::Weight::Normal, "normal", font::Style::Normal),
            ("bold", font::Weight::Bold, "normal", font::Style::Normal),
            ("regular", font::Weight::Normal, "italic", font::Style::Italic),
        ] {
            for content in ["11:11", "08:08", "23:59", "Četvrtak العربية"] {
                let paragraph = Paragraph::with_text(text::Text {
                    content, bounds: Size::new(1000., 200.), size: Pixels(48.),
                    line_height: text::LineHeight::Relative(1.2),
                    font: Font { family: font::Family::Name(family), weight, style, ..Font::DEFAULT },
                    align_x: text::Alignment::Left, align_y: alignment::Vertical::Top,
                    shaping: text::Shaping::Advanced, wrapping: text::Wrapping::None,
                    ellipsize: text::Ellipsize::None,
                });
                let glyphs: Vec<_> = paragraph.buffer().layout_runs().flat_map(|r| r.glyphs.iter()).collect();
                assert!(!glyphs.is_empty());
                assert!(glyphs.iter().all(|g| g.glyph_id != 0), "missing glyph for {content}");
                let mut faces: Vec<_> = glyphs.iter().map(|g| g.font_id).collect();
                faces.sort(); faces.dedup();
                let system = iced_graphics::text::font_system().read().unwrap();
                let names: Vec<_> = faces.iter().map(|id| {
                    let face = system.db().face(*id).unwrap();
                    format!("{}:{:?}:{:?}", face.post_script_name, face.weight, face.style)
                }).collect();
                println!("{family},{weight_name},{style_name},{content},{:.3},{},{}", paragraph.min_bounds().width, names.join(";"), glyphs.len());
            }
        }
    }
}
