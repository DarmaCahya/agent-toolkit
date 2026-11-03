#![forbid(unsafe_code)]

//! Pure-math polar coordinate and arc-slice helpers.

use std::f32::consts::PI;

/// A point in polar coordinates (angle measured in radians).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PolarPoint {
    pub radius: f32,
    pub angle_rad: f32,
}

impl PolarPoint {
    pub fn new(radius: f32, angle_rad: f32) -> Self {
        Self { radius, angle_rad }
    }

    /// Convert to Cartesian coordinates around the given center.
    pub fn to_cartesian(&self, center_x: f32, center_y: f32) -> (f32, f32) {
        polar_to_cartesian(center_x, center_y, self.radius, self.angle_rad)
    }
}

pub fn deg_to_rad(deg: f32) -> f32 {
    deg * PI / 180.0
}

pub fn rad_to_deg(rad: f32) -> f32 {
    rad * 180.0 / PI
}

/// Convert polar to Cartesian coordinates. Angle 0 points along +x and
/// increases toward +y (clockwise on a y-down screen).
pub fn polar_to_cartesian(center_x: f32, center_y: f32, radius: f32, angle_rad: f32) -> (f32, f32) {
    (
        center_x + radius * angle_rad.cos(),
        center_y + radius * angle_rad.sin(),
    )
}

/// Split a full turn proportionally across `values`.
///
/// Returns `(start_rad, end_rad, percentage_0_to_1)` per slice, in input
/// order. Negative and non-finite values count as zero (zero-width slices
/// keep indices aligned with the input). Returns an empty vec when the input
/// is empty or the total is not positive. The last slice ends exactly at
/// `start_angle_rad + TAU`.
pub fn calculate_slice_angles(values: &[f32], start_angle_rad: f32) -> Vec<(f32, f32, f32)> {
    let clean = |v: f32| if v.is_finite() && v > 0.0 { v as f64 } else { 0.0 };
    let total: f64 = values.iter().map(|&v| clean(v)).sum();
    if values.is_empty() || !total.is_finite() || total <= 0.0 {
        return Vec::new();
    }

    let start = if start_angle_rad.is_finite() { start_angle_rad as f64 } else { 0.0 };
    let last = values.len() - 1;
    let mut cumulative = 0.0_f64;
    let mut prev_end = start as f32;

    values
        .iter()
        .enumerate()
        .map(|(i, &v)| {
            let v = clean(v);
            cumulative += v;
            let slice_start = prev_end;
            let slice_end = if i == last {
                (start + std::f64::consts::TAU) as f32
            } else {
                (start + std::f64::consts::TAU * cumulative / total) as f32
            };
            prev_end = slice_end;
            (slice_start, slice_end, (v / total) as f32)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::TAU;

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-5
    }

    #[test]
    fn degree_radian_round_trip() {
        assert!(approx(deg_to_rad(180.0), PI));
        assert!(approx(rad_to_deg(PI / 2.0), 90.0));
        assert!(approx(rad_to_deg(deg_to_rad(37.5)), 37.5));
    }

    #[test]
    fn cartesian_conversion() {
        let (x, y) = polar_to_cartesian(10.0, 20.0, 5.0, 0.0);
        assert!(approx(x, 15.0) && approx(y, 20.0));
        let (x, y) = polar_to_cartesian(0.0, 0.0, 2.0, PI / 2.0);
        assert!(approx(x, 0.0) && approx(y, 2.0));
        let p = PolarPoint::new(1.0, PI);
        let (x, _) = p.to_cartesian(0.0, 0.0);
        assert!(approx(x, -1.0));
    }

    #[test]
    fn slices_cover_full_turn() {
        let s = calculate_slice_angles(&[1.0, 1.0, 2.0], 0.0);
        assert_eq!(s.len(), 3);
        assert!(approx(s[0].1, TAU / 4.0));
        assert!(approx(s[2].2, 0.5));
        assert_eq!(s[0].0, 0.0);
        assert_eq!(s[1].0, s[0].1);
        assert_eq!(s[2].1, TAU);
        let pct: f32 = s.iter().map(|x| x.2).sum();
        assert!(approx(pct, 1.0));
    }

    #[test]
    fn slices_respect_start_angle() {
        let start = -PI / 2.0;
        let s = calculate_slice_angles(&[3.0, 1.0], start);
        assert_eq!(s[0].0, start);
        assert!(approx(s[1].1, start + TAU));
    }

    #[test]
    fn slices_degenerate_inputs() {
        assert!(calculate_slice_angles(&[], 0.0).is_empty());
        assert!(calculate_slice_angles(&[0.0, 0.0], 0.0).is_empty());
        assert!(calculate_slice_angles(&[-1.0, f32::NAN], 0.0).is_empty());
        let s = calculate_slice_angles(&[5.0, -3.0, f32::NAN], 0.0);
        assert_eq!(s.len(), 3);
        assert_eq!(s[1].2, 0.0);
        assert_eq!(s[1].0, s[1].1);
        assert!(s.iter().all(|x| x.0.is_finite() && x.1.is_finite()));
    }
}
