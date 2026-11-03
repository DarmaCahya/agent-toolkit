#![forbid(unsafe_code)]

//! Pure-math scales mapping data domains onto coordinate ranges.

/// Replace NaN/Infinity with a finite fallback.
fn finite_or(val: f32, fallback: f32) -> f32 {
    if val.is_finite() {
        val
    } else {
        fallback
    }
}

/// Linear mapping from a data domain `[min, max]` to a range `[start, end]`.
///
/// A degenerate domain (`min == max`) or non-finite bounds are widened to
/// `[min - 1.0, max + 1.0]` so that `map`/`invert` never produce NaN.
/// Reversed domains and ranges (e.g. SVG y-axes) are supported.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LinearScale {
    domain_min: f32,
    domain_max: f32,
    range_start: f32,
    range_end: f32,
}

impl LinearScale {
    pub fn new(domain_min: f32, domain_max: f32, range_start: f32, range_end: f32) -> Self {
        let mut d0 = finite_or(domain_min, 0.0);
        let mut d1 = finite_or(domain_max, 0.0);
        if !domain_min.is_finite() || !domain_max.is_finite() || d0 == d1 {
            if domain_min.is_finite() != domain_max.is_finite() {
                // Only one bound is usable: anchor on it.
                let anchor = if domain_min.is_finite() { d0 } else { d1 };
                d0 = anchor;
                d1 = anchor;
            }
            d0 -= 1.0;
            d1 += 1.0;
        }
        Self {
            domain_min: d0,
            domain_max: d1,
            range_start: finite_or(range_start, 0.0),
            range_end: finite_or(range_end, 0.0),
        }
    }

    pub fn domain(&self) -> (f32, f32) {
        (self.domain_min, self.domain_max)
    }

    pub fn range(&self) -> (f32, f32) {
        (self.range_start, self.range_end)
    }

    /// Map a domain value to a range position. Values outside the domain
    /// extrapolate; NaN maps to the range start.
    pub fn map(&self, val: f32) -> f32 {
        if val.is_nan() {
            return self.range_start;
        }
        let (d0, d1) = (self.domain_min as f64, self.domain_max as f64);
        let (r0, r1) = (self.range_start as f64, self.range_end as f64);
        let v = (val as f64).clamp(f32::MIN as f64, f32::MAX as f64);
        let out = (r0 + (v - d0) / (d1 - d0) * (r1 - r0)) as f32;
        if out.is_finite() {
            out
        } else {
            self.range_start
        }
    }

    /// Map a range position back to a domain value. A zero-length range
    /// yields the domain minimum.
    pub fn invert(&self, pos: f32) -> f32 {
        if pos.is_nan() {
            return self.domain_min;
        }
        let (d0, d1) = (self.domain_min as f64, self.domain_max as f64);
        let (r0, r1) = (self.range_start as f64, self.range_end as f64);
        if r1 == r0 {
            return self.domain_min;
        }
        let p = (pos as f64).clamp(f32::MIN as f64, f32::MAX as f64);
        let out = (d0 + (p - r0) / (r1 - r0) * (d1 - d0)) as f32;
        if out.is_finite() {
            out
        } else {
            self.domain_min
        }
    }

    /// Clamp a domain value into the domain bounds. NaN yields the minimum.
    pub fn clamp(&self, val: f32) -> f32 {
        let lo = self.domain_min.min(self.domain_max);
        let hi = self.domain_min.max(self.domain_max);
        if val.is_nan() {
            return lo;
        }
        val.clamp(lo, hi)
    }
}

/// Band scale placing named categories into equal slots across a range.
///
/// `padding` (0.0..=1.0) is the fraction of each slot left empty and split
/// evenly on both sides of the band.
#[derive(Debug, Clone, PartialEq)]
pub struct CategoryScale {
    categories: Vec<String>,
    range_start: f32,
    range_end: f32,
    padding: f32,
}

impl CategoryScale {
    pub fn new(categories: Vec<String>, range_start: f32, range_end: f32, padding: f32) -> Self {
        let padding = if padding.is_nan() { 0.0 } else { padding.clamp(0.0, 1.0) };
        Self {
            categories,
            range_start: finite_or(range_start, 0.0),
            range_end: finite_or(range_end, 0.0),
            padding,
        }
    }

    pub fn categories(&self) -> &[String] {
        &self.categories
    }

    pub fn len(&self) -> usize {
        self.categories.len()
    }

    pub fn is_empty(&self) -> bool {
        self.categories.is_empty()
    }

    pub fn index_of(&self, category: &str) -> Option<usize> {
        self.categories.iter().position(|c| c == category)
    }

    /// Signed distance between consecutive slot origins.
    fn step(&self) -> f64 {
        let n = self.categories.len().max(1) as f64;
        (self.range_end as f64 - self.range_start as f64) / n
    }

    /// Width of the full slot (band plus padding), always non-negative.
    pub fn step_size(&self) -> f32 {
        self.step().abs() as f32
    }

    /// Width of the drawable band inside each slot, always non-negative.
    pub fn bandwidth(&self) -> f32 {
        (self.step().abs() * (1.0 - self.padding as f64)) as f32
    }

    /// Coordinate where the band for `index` begins (the edge nearest the
    /// range start). Out-of-bounds indices extrapolate.
    pub fn map_start(&self, index: usize) -> f32 {
        let step = self.step();
        (self.range_start as f64 + step * index as f64 + step * self.padding as f64 / 2.0) as f32
    }

    /// Coordinate of the center of the band for `index`.
    pub fn map_center(&self, index: usize) -> f32 {
        let step = self.step();
        (self.range_start as f64 + step * (index as f64 + 0.5)) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    #[test]
    fn linear_maps_and_inverts() {
        let s = LinearScale::new(0.0, 100.0, 0.0, 500.0);
        assert!(approx(s.map(50.0), 250.0));
        assert!(approx(s.invert(250.0), 50.0));
    }

    #[test]
    fn linear_reversed_range() {
        let s = LinearScale::new(0.0, 10.0, 100.0, 0.0);
        assert!(approx(s.map(0.0), 100.0));
        assert!(approx(s.map(10.0), 0.0));
    }

    #[test]
    fn linear_constant_domain_is_finite() {
        let s = LinearScale::new(5.0, 5.0, 0.0, 100.0);
        assert_eq!(s.domain(), (4.0, 6.0));
        assert!(approx(s.map(5.0), 50.0));
        let z = LinearScale::new(0.0, 0.0, 0.0, 10.0);
        assert!(z.map(0.0).is_finite());
    }

    #[test]
    fn linear_non_finite_inputs_never_nan() {
        let s = LinearScale::new(f32::NAN, f32::INFINITY, 0.0, 10.0);
        assert!(s.map(f32::NAN).is_finite());
        assert!(s.map(f32::INFINITY).is_finite());
        assert!(s.invert(f32::NEG_INFINITY).is_finite());
        let t = LinearScale::new(3.0, f32::NAN, 0.0, 10.0);
        assert_eq!(t.domain(), (2.0, 4.0));
    }

    #[test]
    fn linear_zero_range_invert() {
        let s = LinearScale::new(0.0, 10.0, 5.0, 5.0);
        assert!(s.invert(5.0).is_finite());
    }

    #[test]
    fn linear_clamp() {
        let s = LinearScale::new(0.0, 10.0, 0.0, 1.0);
        assert_eq!(s.clamp(-5.0), 0.0);
        assert_eq!(s.clamp(15.0), 10.0);
        assert_eq!(s.clamp(4.0), 4.0);
        let r = LinearScale::new(10.0, 0.0, 0.0, 1.0);
        assert_eq!(r.clamp(15.0), 10.0);
    }

    #[test]
    fn category_geometry() {
        let cats = vec!["a".to_string(), "b".to_string(), "c".to_string(), "d".to_string()];
        let s = CategoryScale::new(cats, 0.0, 400.0, 0.2);
        assert!(approx(s.step_size(), 100.0));
        assert!(approx(s.bandwidth(), 80.0));
        assert!(approx(s.map_start(1), 110.0));
        assert!(approx(s.map_center(0), 50.0));
        assert!(approx(s.map_center(3), 350.0));
        assert_eq!(s.index_of("c"), Some(2));
    }

    #[test]
    fn category_edge_cases() {
        let empty = CategoryScale::new(vec![], 0.0, 100.0, 0.5);
        assert!(empty.bandwidth().is_finite());
        assert!(empty.map_center(0).is_finite());
        let p = CategoryScale::new(vec!["x".into()], 0.0, 10.0, 7.0);
        assert_eq!(p.bandwidth(), 0.0);
        let n = CategoryScale::new(vec!["x".into()], 0.0, 10.0, f32::NAN);
        assert!(approx(n.bandwidth(), 10.0));
        let rev = CategoryScale::new(vec!["x".into(), "y".into()], 100.0, 0.0, 0.0);
        assert!(approx(rev.map_center(0), 75.0));
        assert!(approx(rev.bandwidth(), 50.0));
    }
}
