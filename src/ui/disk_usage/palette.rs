use gpui::Hsla;

const GOLDEN_ANGLE_DEG: f32 = 137.50776;

pub fn wedge_color(branch_index: usize, depth: u32) -> Hsla {
    let hue = (branch_index as f32 * GOLDEN_ANGLE_DEG).rem_euclid(360.0) / 360.0;
    let lightness = (0.55 - (depth.saturating_sub(1)) as f32 * 0.07).max(0.28);
    Hsla { h: hue, s: 0.55, l: lightness, a: 1.0 }
}

pub fn inaccessible_color() -> Hsla {
    Hsla { h: 0.0, s: 0.0, l: 0.35, a: 1.0 }
}

pub fn aggregate_color() -> Hsla {
    Hsla { h: 0.0, s: 0.0, l: 0.5, a: 1.0 }
}

pub fn highlight(color: Hsla) -> Hsla {
    Hsla { l: (color.l + 0.15).min(0.92), ..color }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn different_branches_get_different_hues() {
        let a = wedge_color(0, 1);
        let b = wedge_color(1, 1);
        assert_ne!(a.h, b.h);
    }

    #[test]
    fn deeper_rings_are_darker() {
        let shallow = wedge_color(0, 1);
        let deep = wedge_color(0, 3);
        assert!(deep.l < shallow.l);
    }

    #[test]
    fn highlight_never_exceeds_full_lightness_bound() {
        let color = Hsla { h: 0.5, s: 0.5, l: 0.9, a: 1.0 };
        assert!(highlight(color).l <= 0.92);
    }
}
