//! Great-circle distances and the speed-of-light floor, for the static S2 table (the live
//! instrument computes the same thing in the browser).

/// IUGG mean Earth radius, km.
const EARTH_RADIUS_KM: f64 = 6371.0088;
/// Group index of standard single-mode fibre (G.652) at 1550 nm.
pub const FIBRE_INDEX: f64 = 1.468;
/// Speed of light in vacuum, km/ms.
const C_KM_PER_MS: f64 = 299_792.458 / 1000.0;

/// Kilometres light covers per millisecond in fibre (≈ 204.2).
pub fn fibre_km_per_ms() -> f64 {
    C_KM_PER_MS / FIBRE_INDEX
}

pub fn haversine_km((lat1, lon1): (f64, f64), (lat2, lon2): (f64, f64)) -> f64 {
    let (p1, p2) = (lat1.to_radians(), lat2.to_radians());
    let dp = p2 - p1;
    let dl = (lon2 - lon1).to_radians();
    let a = (dp / 2.0).sin().powi(2) + p1.cos() * p2.cos() * (dl / 2.0).sin().powi(2);
    2.0 * EARTH_RADIUS_KM * a.sqrt().asin()
}

/// The shortest possible round trip through fibre over a great circle, in ms.
pub fn round_trip_floor_ms(km: f64) -> f64 {
    2.0 * km / fibre_km_per_ms()
}

/// Reference cities for the static table: (message id, lat, lon).
pub const CITIES: [(&str, f64, f64); 5] = [
    ("city-almaty", 43.2389, 76.8897),
    ("city-dubai", 25.2048, 55.2708),
    ("city-moscow", 55.7558, 37.6173),
    ("city-frankfurt", 50.1109, 8.6821),
    ("city-new-york", 40.7128, -74.0060),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_in_fibre() {
        assert!((fibre_km_per_ms() - 204.218).abs() < 0.01);
    }

    #[test]
    fn known_distance() {
        // London–Paris is ~344 km on the great circle.
        let d = haversine_km((51.5074, -0.1278), (48.8566, 2.3522));
        assert!((340.0..348.0).contains(&d), "{d}");
    }
}
