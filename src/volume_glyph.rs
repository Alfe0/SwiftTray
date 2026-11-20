//! Windows Fluent volume glyphs, chosen from the effective sound state.
//! The original app icons are deliberately independent of this system glyph.

pub fn for_level(level: f32, muted: bool) -> &'static str {
    let percentage = (level.clamp(0.0, 1.0) * 100.0).round() as u32;
    if muted || percentage == 0 { return "\u{E74F}"; }
    match percentage {
        0 => "\u{E74F}",      // Muted / no volume
        1..=32 => "\u{E993}",  // Volume1
        33..=65 => "\u{E994}", // Volume2
        _ => "\u{E995}",       // Volume3
    }
}

#[cfg(test)]
mod tests {
    use super::for_level;
    #[test]
    fn matches_windows_volume_steps() {
        assert_eq!(for_level(0.0, false), "\u{E74F}");
        assert_eq!(for_level(0.01, false), "\u{E993}");
        assert_eq!(for_level(0.32, false), "\u{E993}");
        assert_eq!(for_level(0.33, false), "\u{E994}");
        assert_eq!(for_level(0.65, false), "\u{E994}");
        assert_eq!(for_level(0.66, false), "\u{E995}");
        assert_eq!(for_level(1.0, true), "\u{E74F}");
    }
}
