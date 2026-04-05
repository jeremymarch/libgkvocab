use crate::{ArrowedState, GlossOccurrance};

pub fn get_width(text: &str) -> f32 {
    use rustybuzz::{Face, UnicodeBuffer, shape};
    let font_bytes = include_bytes!("../IFAOGrec.ttf") as &[u8];

    // Create a rustybuzz Face
    let face = Face::from_slice(font_bytes, 0).expect("Failed to load font");

    // Build a UnicodeBuffer for shaping
    let mut buffer = UnicodeBuffer::new();
    buffer.push_str(text);

    // Shape the text with no features (empty slice)
    let glyph_buffer = shape(&face, &[], buffer);

    // Get shaped positions
    let positions = glyph_buffer.glyph_positions();

    // Get the font’s units per EM for scaling
    let upem = face.units_per_em() as f32;

    // Desired pixel font size
    let px_size: f32 = 32.0;

    let scale = px_size / upem;

    // Sum horizontal advances
    let mut width_px: f32 = 0.0;

    for pos in positions {
        // Rustybuzz uses 26.6 fixed-point (divide by 64) to get float font units
        let adv_font_units = pos.x_advance as f32 / 64.0;
        width_px += adv_font_units * scale;
    }
    width_px
}

pub fn count_lines(gloss_occurances: &[GlossOccurrance]) -> Vec<usize> {
    use std::cmp::max;

    let text_lines_per_page = 23;
    let width_of_line: f32 = 7.0;
    let width_of_lemma: f32 = 9.5;
    let width_of_def: f32 = 8.3;

    let mut word_counts = vec![];
    let mut current_text_width = 0.0;
    let mut lines_in_page = 0;
    let mut words_per_page = 0;
    for go in gloss_occurances {
        words_per_page += 1;
        current_text_width += get_width(&go.word.word);
        if current_text_width > width_of_line {
            lines_in_page += 1;
            current_text_width = 0.0;
        }
        if let Some(gloss) = go.gloss
            && go.arrowed_state != ArrowedState::Invisible
        {
            let lemma_width = get_width(&gloss.lemma);
            let def_width = get_width(&gloss.def);
            let lemma_lines = (width_of_lemma / lemma_width).ceil() as usize;
            let def_lines = (width_of_def / def_width).ceil() as usize;
            lines_in_page += max(lemma_lines, def_lines);
        }
        if lines_in_page >= text_lines_per_page {
            word_counts.push(words_per_page);
            words_per_page = 0;
            lines_in_page = 0;
        }
    }
    word_counts
}
