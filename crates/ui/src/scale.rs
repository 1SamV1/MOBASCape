//! One whole-number scale for the whole interface, chosen from the window.

/// The largest scale the interface is ever drawn at.
pub const LARGEST: u32 = 4;

/// The largest whole scale, from 1 to [`LARGEST`], at which a layout that
/// needs `need` pixels at its own size fits in `window`. Never less than 1:
/// a window too small for scale 1 is the layout's problem, not the scale's.
pub fn choose(window: (u32, u32), need: (u32, u32)) -> u32 {
    let fit = |have: u32, need: u32| have.checked_div(need).unwrap_or(LARGEST);
    fit(window.0, need.0)
        .min(fit(window.1, need.1))
        .clamp(1, LARGEST)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_scale_steps_at_whole_multiples_of_what_the_layout_needs() {
        let need = (640, 360);
        assert_eq!(choose((640, 360), need), 1);
        assert_eq!(choose((1279, 720), need), 1, "one pixel short of two");
        assert_eq!(choose((1280, 719), need), 1);
        assert_eq!(choose((1280, 720), need), 2);
        assert_eq!(choose((1920, 1080), need), 3);
        assert_eq!(choose((2560, 1440), need), 4);
        assert_eq!(choose((7680, 4320), need), 4, "never more than four");
        assert_eq!(choose((300, 200), need), 1, "never less than one");
        // the tighter of the two directions decides
        assert_eq!(choose((2560, 720), need), 2);
    }
}
