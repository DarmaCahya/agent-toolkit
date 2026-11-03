#![forbid(unsafe_code)]

//! Heckbert "nice numbers" tick generation and human-friendly tick labels.

/// Upper bound on generated ticks, guarding against pathological inputs.
const MAX_TICKS: usize = 1000;

/// Magnitude suffixes, largest first.
const UNITS: [(f64, &str); 4] = [(1e12, "T"), (1e9, "B"), (1e6, "M"), (1e3, "K")];

pub struct TickGenerator;

/// Heckbert's `nicenum`: round (or ceil) `x` to 1, 2, 5 or 10 times 10^n.
fn nice_num(x: f64, round: bool) -> f64 {
    let exp = x.log10().floor();
    let base = 10f64.powf(exp);
    let f = x / base;
    let nice = if round {
        if f < 1.5 {
            1.0
        } else if f < 3.0 {
            2.0
        } else if f < 7.0 {
            5.0
        } else {
            10.0
        }
    } else if f <= 1.0 {
        1.0
    } else if f <= 2.0 {
        2.0
    } else if f <= 5.0 {
        5.0
    } else {
        10.0
    };
    nice * base
}

/// Format with `decimals` places and strip trailing zeros / dangling dot.
fn trim_decimal(val: f64, decimals: usize) -> String {
    let mut s = format!("{:.*}", decimals, val);
    if s.contains('.') {
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
    }
    if s == "-0" {
        s = "0".to_string();
    }
    s
}

impl TickGenerator {
    /// Generate roughly `target_count` evenly spaced, "nice" ticks that
    /// cover `[min, max]`.
    ///
    /// Swapped bounds are reordered; a non-finite bound falls back to the
    /// other one (or 0.0 if both are non-finite). When the bounds are equal
    /// a single tick is returned. All returned values are finite and sorted
    /// ascending, with steps of 1, 2 or 5 times a power of ten.
    pub fn generate_ticks(min: f32, max: f32, target_count: usize) -> Vec<f32> {
        let (mut lo, mut hi) = match (min.is_finite(), max.is_finite()) {
            (true, true) => (min as f64, max as f64),
            (true, false) => (min as f64, min as f64),
            (false, true) => (max as f64, max as f64),
            (false, false) => (0.0, 0.0),
        };
        if lo > hi {
            std::mem::swap(&mut lo, &mut hi);
        }
        if lo == hi {
            return vec![lo as f32];
        }

        let target = target_count.max(2) as f64;
        let range = nice_num(hi - lo, false);
        let step = nice_num(range / (target - 1.0), true);
        if !step.is_finite() || step <= 0.0 {
            return vec![lo as f32, hi as f32];
        }

        let first = (lo / step).floor();
        let last = (hi / step).ceil();
        if (last - first) >= MAX_TICKS as f64 {
            return vec![lo as f32, hi as f32];
        }
        let (first, last) = (first as i64, last as i64);

        // Round each multiple to the step's decimal precision so values like
        // 0.30000000000000004 come out as 0.3.
        let decimals = (-step.log10().floor()).clamp(0.0, 300.0) as i32;
        let scale = 10f64.powi(decimals);

        let mut ticks: Vec<f32> = (first..=last)
            .map(|k| {
                let v = k as f64 * step;
                let v = if scale.is_finite() { (v * scale).round() / scale } else { v };
                v as f32
            })
            .filter(|v| v.is_finite())
            .collect();
        ticks.dedup();
        ticks
    }

    /// Format a tick value compactly: `50_000_000.0` -> `"50M"`,
    /// `1500.0` -> `"1.5K"`, `0.25` -> `"0.25"`. Never emits trailing zeros;
    /// non-finite input yields `"0"`.
    pub fn format_tick(val: f32) -> String {
        if !val.is_finite() {
            return "0".to_string();
        }
        let v = val as f64;
        let abs = v.abs();
        if abs == 0.0 {
            return "0".to_string();
        }
        if abs < 1e-9 {
            return format!("{:e}", val);
        }

        if abs >= 1e3 {
            // Pick the largest unit that fits, then promote if rounding
            // pushes the scaled value to 1000 (e.g. 999_999 -> "1M").
            let mut idx = UNITS.iter().position(|(u, _)| abs >= *u).unwrap_or(UNITS.len() - 1);
            let rounded = |i: usize| (v / UNITS[i].0 * 100.0).round().abs() / 100.0;
            while idx > 0 && rounded(idx) >= 1000.0 {
                idx -= 1;
            }
            let (unit, suffix) = UNITS[idx];
            return format!("{}{}", trim_decimal(v / unit, 2), suffix);
        }

        let decimals = if abs >= 1.0 {
            2
        } else {
            ((-abs.log10().floor()) as usize + 2).min(12)
        };
        trim_decimal(v, decimals)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_clean(ticks: &[f32]) {
        assert!(!ticks.is_empty());
        assert!(ticks.iter().all(|t| t.is_finite()));
        assert!(ticks.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn basic_range() {
        assert_eq!(
            TickGenerator::generate_ticks(0.0, 100.0, 6),
            vec![0.0, 20.0, 40.0, 60.0, 80.0, 100.0]
        );
    }

    #[test]
    fn covers_bounds() {
        let t = TickGenerator::generate_ticks(3.2, 97.1, 5);
        assert_clean(&t);
        assert!(t[0] <= 3.2 && *t.last().unwrap() >= 97.1);
    }

    #[test]
    fn negative_and_mixed() {
        let t = TickGenerator::generate_ticks(-75.0, 130.0, 6);
        assert_clean(&t);
        assert!(t.contains(&0.0));
        assert!(t[0] <= -75.0 && *t.last().unwrap() >= 130.0);
    }

    #[test]
    fn micro_decimals_are_clean() {
        let t = TickGenerator::generate_ticks(0.0, 0.0003, 4);
        assert_clean(&t);
        assert!(t[0] <= 0.0 && *t.last().unwrap() >= 0.0003);
        let t = TickGenerator::generate_ticks(0.0, 1.0, 11);
        assert_clean(&t);
    }

    #[test]
    fn large_rupiah_values() {
        let t = TickGenerator::generate_ticks(0.0, 350_000_000.0, 5);
        assert_clean(&t);
        assert_eq!(t, vec![0.0, 100_000_000.0, 200_000_000.0, 300_000_000.0, 400_000_000.0]);
        let t = TickGenerator::generate_ticks(120_000_000.0, 480_000_000.0, 6);
        assert_clean(&t);
    }

    #[test]
    fn constant_and_degenerate() {
        assert_eq!(TickGenerator::generate_ticks(5.0, 5.0, 5), vec![5.0]);
        assert_eq!(TickGenerator::generate_ticks(0.0, 0.0, 5), vec![0.0]);
        assert_eq!(TickGenerator::generate_ticks(f32::NAN, f32::NAN, 5), vec![0.0]);
        assert_clean(&TickGenerator::generate_ticks(10.0, 0.0, 0));
        assert_clean(&TickGenerator::generate_ticks(0.0, 10.0, 1));
        assert_clean(&TickGenerator::generate_ticks(f32::MIN, f32::MAX, 5));
        assert_clean(&TickGenerator::generate_ticks(0.0, 1.0, usize::MAX));
        assert_clean(&TickGenerator::generate_ticks(1e-40, 3e-40, 5));
    }

    #[test]
    fn steps_are_nice() {
        for (lo, hi) in [(0.0, 7.0), (0.0, 13.0), (-3.0, 11.0), (0.0, 0.07), (0.0, 9e8)] {
            let t = TickGenerator::generate_ticks(lo, hi, 6);
            let step = (t[1] - t[0]) as f64;
            let mantissa = step / 10f64.powf(step.log10().floor());
            assert!(
                [1.0, 2.0, 5.0].iter().any(|m| (mantissa - m).abs() < 1e-3),
                "step {step} for {lo}..{hi}"
            );
        }
    }

    #[test]
    fn formatting() {
        assert_eq!(TickGenerator::format_tick(50_000_000.0), "50M");
        assert_eq!(TickGenerator::format_tick(1500.0), "1.5K");
        assert_eq!(TickGenerator::format_tick(-2_500_000_000.0), "-2.5B");
        assert_eq!(TickGenerator::format_tick(999_999.0), "1M");
        assert_eq!(TickGenerator::format_tick(0.0), "0");
        assert_eq!(TickGenerator::format_tick(-0.0), "0");
        assert_eq!(TickGenerator::format_tick(20.0), "20");
        assert_eq!(TickGenerator::format_tick(0.25), "0.25");
        assert_eq!(TickGenerator::format_tick(0.0002), "0.0002");
        assert_eq!(TickGenerator::format_tick(f32::NAN), "0");
        assert_eq!(TickGenerator::format_tick(3e12), "3T");
        assert_eq!(TickGenerator::format_tick(1e15), "1000T");
    }
}
