//! SwiftTray v3 identity. Colors originate from the user-provided SVG logo.
//! Centralized values are Win32 COLORREF (0x00BBGGRR); icons from third-party
//! applications retain their native colors. Mute red is deliberately separate.
pub const BRAND_GREEN:u32 = 0x007AB220; // #20B27A
pub const BRAND_TEAL:u32 = 0x00A39F14; // #149FA3
pub const BRAND_BLUE:u32 = 0x00DB781B; // #1B78DB
pub const BRAND_DEEP_BLUE:u32 = 0x00D65627; // #2756D6
pub const BRAND_HOVER:u32 = 0x00CEE869; // #69E8CE
pub const BRAND_METER:u32 = 0x00C8E28B; // #8BE2C8
pub const BRAND_INK:u32 = 0x002A2A09; // #092A2A

// The source SVG keeps its original geometry/colors/gradient. UI controls use
// a gently delayed blue transition so the last third does not appear dominant.
// Keep one continuous brush across the entire slider: no joined segments.
pub const LOGO_STOPS:[f32;4]=[0.0,0.45,0.75,1.0];
pub const STOPS:[f32;4]=[0.0,0.53,0.86,1.0];
pub const COLORS:[u32;4]=[BRAND_GREEN,BRAND_TEAL,BRAND_BLUE,BRAND_DEEP_BLUE];

pub fn color_at(position:f32)->u32 {
    let t=if position.is_finite(){position.clamp(0.0,1.0)}else{0.0};
    for i in 0..3 {
        if t<=STOPS[i+1] {
            let f=(t-STOPS[i])/(STOPS[i+1]-STOPS[i]);
            return blend(COLORS[i],COLORS[i+1],f);
        }
    }
    BRAND_DEEP_BLUE
}

fn blend(a:u32,b:u32,t:f32)->u32 {
    let blend_channel=|shift:u32|{
        let from=((a>>shift)&255) as f32;
        let to=((b>>shift)&255) as f32;
        ((from+(to-from)*t).round() as u32).min(255)<<shift
    };
    blend_channel(0)|blend_channel(8)|blend_channel(16)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn brand_colors_match_svg() {
        assert_eq!(BRAND_GREEN,0x007AB220);
        assert_eq!(BRAND_TEAL,0x00A39F14);
        assert_eq!(BRAND_BLUE,0x00DB781B);
        assert_eq!(BRAND_DEEP_BLUE,0x00D65627);
    }
    #[test]
    fn brand_gradient_has_no_discontinuous_split() {
        for i in 0..4 { assert_eq!(color_at(STOPS[i]),COLORS[i]); }
        // Sample immediately either side of the SVG's 45% and 75% stops.
        for stop in [0.45_f32,0.75_f32] {
            let a=color_at(stop-0.0001);
            let b=color_at(stop+0.0001);
            for shift in [0,8,16] {
                let delta=(((a>>shift)&255) as i32-((b>>shift)&255) as i32).abs();
                assert!(delta<=1);
            }
        }
    }
    #[test]
    fn ui_gradient_has_soft_blue_transition() {
        assert_eq!(LOGO_STOPS,[0.0,0.45,0.75,1.0]);
        assert_eq!(STOPS,[0.0,0.53,0.86,1.0]);
        let mid = color_at(0.70);
        assert_ne!(mid, BRAND_BLUE);
        assert_ne!(mid, BRAND_DEEP_BLUE);
    }
    #[test]
    fn out_of_range_colors_are_clamped() {
        assert_eq!(color_at(-5.0),BRAND_GREEN);
        assert_eq!(color_at(5.0),BRAND_DEEP_BLUE);
        assert_eq!(color_at(f32::NAN),BRAND_GREEN);
    }
}
