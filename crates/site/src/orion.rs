//! Orion, from real coordinates. Every drawing of the constellation on the page — the navigation,
//! the S3 figure and the particle targets in the hero engine — is projected from these J2000
//! positions (ICRS; Wikipedia / SIMBAD, rounded to 0.1 s and 1″), so it matches the sky and the
//! full-Orion brand board (which is the same projection at ~30 px per degree).

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tint {
    Starlight,
    Rust,
    Blue,
}

pub struct Star {
    pub id: &'static str,
    /// Right ascension: hours, minutes, seconds.
    pub ra: (f64, f64, f64),
    /// Declination: sign, degrees, arcminutes, arcseconds.
    pub dec: (f64, f64, f64, f64),
    /// Drawn radius on the brand board (brightness, with brand emphasis on Betelgeuse and Mintaka).
    pub r: f64,
    pub tint: Tint,
    /// Section anchor this star stands for.
    pub section: &'static str,
}

pub const STARS: [Star; 8] = [
    Star { id: "betelgeuse", ra: (5.0, 55.0, 10.3), dec: (1.0, 7.0, 24.0, 25.0), r: 23.0, tint: Tint::Rust, section: "highload" },
    Star { id: "meissa", ra: (5.0, 35.0, 8.3), dec: (1.0, 9.0, 56.0, 3.0), r: 4.5, tint: Tint::Starlight, section: "design" },
    Star { id: "bellatrix", ra: (5.0, 25.0, 7.9), dec: (1.0, 6.0, 20.0, 59.0), r: 9.0, tint: Tint::Starlight, section: "team" },
    Star { id: "mintaka", ra: (5.0, 32.0, 0.4), dec: (-1.0, 0.0, 17.0, 57.0), r: 15.0, tint: Tint::Blue, section: "process" },
    Star { id: "alnilam", ra: (5.0, 36.0, 12.8), dec: (-1.0, 1.0, 12.0, 7.0), r: 9.0, tint: Tint::Starlight, section: "process" },
    Star { id: "alnitak", ra: (5.0, 40.0, 45.5), dec: (-1.0, 1.0, 56.0, 34.0), r: 9.0, tint: Tint::Starlight, section: "process" },
    Star { id: "saiph", ra: (5.0, 47.0, 45.4), dec: (-1.0, 9.0, 40.0, 11.0), r: 8.0, tint: Tint::Starlight, section: "security" },
    Star { id: "rigel", ra: (5.0, 14.0, 32.3), dec: (-1.0, 8.0, 12.0, 6.0), r: 16.0, tint: Tint::Starlight, section: "efficiency" },
];

/// Stick-figure lines as drawn on the brand board. The belt segments are dashed there.
pub const LINES: [(&str, &str, bool); 8] = [
    ("betelgeuse", "meissa", false),
    ("meissa", "bellatrix", false),
    ("bellatrix", "mintaka", false),
    ("mintaka", "rigel", false),
    ("betelgeuse", "alnitak", false),
    ("alnitak", "saiph", false),
    ("alnitak", "alnilam", true),
    ("alnilam", "mintaka", true),
];

impl Star {
    pub fn ra_deg(&self) -> f64 {
        (self.ra.0 + self.ra.1 / 60.0 + self.ra.2 / 3600.0) * 15.0
    }

    pub fn dec_deg(&self) -> f64 {
        self.dec.0 * (self.dec.1 + self.dec.2 / 60.0 + self.dec.3 / 3600.0)
    }

    /// "RA 5h 14m 32s"
    pub fn ra_label(&self) -> String {
        format!("RA {}h {:02}m {:02.0}s", self.ra.0, self.ra.1, self.ra.2.floor())
    }

    /// "Dec −8° 12′ 06″" (a real minus sign)
    pub fn dec_label(&self) -> String {
        let sign = if self.dec.0 < 0.0 { "−" } else { "+" };
        format!("Dec {sign}{}° {:02}′ {:02}″", self.dec.1, self.dec.2, self.dec.3)
    }
}

pub fn star(id: &str) -> &'static Star {
    STARS.iter().find(|s| s.id == id).unwrap_or(&STARS[4])
}

/// Gnomonic projection centred on Alnilam, in degrees: x grows to the west (right on a sky map
/// held up with north on top), y grows to the south (down the page).
pub fn project(s: &Star) -> (f64, f64) {
    let c = star("alnilam");
    let (ra0, dec0) = (c.ra_deg().to_radians(), c.dec_deg().to_radians());
    let (ra, dec) = (s.ra_deg().to_radians(), s.dec_deg().to_radians());
    let cos_c = dec0.sin() * dec.sin() + dec0.cos() * dec.cos() * (ra - ra0).cos();
    let x = dec.cos() * (ra - ra0).sin() / cos_c;
    let y = (dec0.cos() * dec.sin() - dec0.sin() * dec.cos() * (ra - ra0).cos()) / cos_c;
    (-x.to_degrees(), -y.to_degrees())
}

/// The projected figure in a box: (star id, x, y, r) in a `width`-wide coordinate system with a
/// margin, plus the box height. Radii scale with the figure.
pub fn layout(width: f64, margin: f64) -> (Vec<(&'static Star, f64, f64, f64)>, f64) {
    let pts: Vec<_> = STARS.iter().map(|s| (s, project(s))).collect();
    let (min_x, max_x) = pts.iter().fold((f64::MAX, f64::MIN), |(a, b), (_, (x, _))| (a.min(*x), b.max(*x)));
    let (min_y, max_y) = pts.iter().fold((f64::MAX, f64::MIN), |(a, b), (_, (_, y))| (a.min(*y), b.max(*y)));
    let scale = (width - 2.0 * margin) / (max_x - min_x);
    // The brand board draws ~30 px per degree with Betelgeuse at r = 23.
    let r_scale = scale / 30.0;
    let height = (max_y - min_y) * scale + 2.0 * margin;
    let out = pts.into_iter().map(|(s, (x, y))| (s, margin + (x - min_x) * scale, margin + (y - min_y) * scale, s.r * r_scale)).collect();
    (out, height)
}

/// The logo's construction coordinates (brand board 10), Alnilam at 0,0, y down.
pub const LOGO: [(&str, f64, f64); 7] = [
    ("betelgeuse", -64.0, -104.0),
    ("bellatrix", 56.0, -90.0),
    ("mintaka", 26.0, -13.0),
    ("alnilam", 0.0, 0.0),
    ("alnitak", -26.0, 13.0),
    ("saiph", -56.0, 104.0),
    ("rigel", 64.0, 90.0),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projection_matches_the_sky() {
        let (bx, by) = project(star("betelgeuse"));
        let (rx, ry) = project(star("rigel"));
        let (mx, my) = project(star("mintaka"));
        let (ax, ay) = project(star("alnitak"));
        assert!(bx < 0.0 && by < 0.0, "Betelgeuse is upper left");
        assert!(rx > 0.0 && ry > 0.0, "Rigel is lower right");
        assert!(mx > ax && my < ay, "the belt rises from Alnitak to Mintaka");
        // Betelgeuse–Rigel are ~18.6° apart on the sky.
        let d = ((bx - rx).powi(2) + (by - ry).powi(2)).sqrt();
        assert!((18.0..19.5).contains(&d), "{d}");
    }

    #[test]
    fn labels_use_real_signs() {
        assert_eq!(star("rigel").dec_label(), "Dec −8° 12′ 06″");
        assert_eq!(star("meissa").ra_label(), "RA 5h 35m 08s");
    }
}
