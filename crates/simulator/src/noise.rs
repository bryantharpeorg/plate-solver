//! Reproducible pixel noise without a random-number dependency.
use std::{f64::consts::TAU, sync::LazyLock};

pub(crate) struct SplitMix64 {
    state: u64,
    spare_normal: Option<f64>,
}

impl SplitMix64 {
    pub fn new(seed: u64, frame: u64) -> Self {
        // Separate frame streams, including frame zero. Every exposure starts afresh.
        Self {
            state: seed.wrapping_add(frame.wrapping_mul(0x9e3779b97f4a7c15)),
            spare_normal: None,
        }
    }

    pub fn next(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e3779b97f4a7c15);
        let mut word = self.state;
        word = (word ^ (word >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        word = (word ^ (word >> 27)).wrapping_mul(0x94d049bb133111eb);
        word ^ (word >> 31)
    }

    fn uniform(&mut self) -> f64 {
        // Open interval: log and reciprocal operations never see an endpoint.
        ((self.next() >> 12) as f64 + 0.5) / 4_503_599_627_370_496.0
    }

    pub fn normal(&mut self) -> f64 {
        if let Some(spare) = self.spare_normal.take() {
            return spare;
        }
        let radius = (-2.0 * self.uniform().ln()).sqrt();
        let (sin, cos) = (TAU * self.uniform()).sin_cos();
        self.spare_normal = Some(radius * sin);
        radius * cos
    }

    /// Exact small-mean product sampling and Hoermann's PTRS for larger means
    /// (Insurance: Mathematics and Economics 12 (1993), 39–45). No normal
    /// approximation to shot noise. The log-factorial is cached for camera counts.
    pub fn poisson(&mut self, mean: f64) -> f64 {
        if mean < 10.0 {
            let stop = (-mean).exp();
            let mut product = self.uniform();
            let mut count = 0;
            while product > stop {
                count += 1;
                product *= self.uniform();
            }
            return f64::from(count);
        }
        let b = 0.931 + 2.53 * mean.sqrt();
        let a = -0.059 + 0.02483 * b;
        let inv_alpha = 1.1239 + 1.1328 / (b - 3.4);
        let squeeze = 0.9277 - 3.6224 / (b - 2.0);
        loop {
            let u = self.uniform() - 0.5;
            let v = self.uniform();
            let distance = 0.5 - u.abs();
            let count = ((2.0 * a / distance + b) * u + mean + 0.43).floor();
            if distance >= 0.07 && v <= squeeze {
                return count;
            }
            if count < 0.0 || (distance < 0.013 && v > distance) {
                continue;
            }
            let lhs = (v * inv_alpha / (a / distance.powi(2) + b)).ln();
            if lhs <= -mean + count * mean.ln() - log_factorial(count as usize) {
                return count;
            }
        }
    }
}

fn log_factorial(n: usize) -> f64 {
    static TABLE: LazyLock<[f64; 1024]> = LazyLock::new(|| {
        let mut values = [0.0; 1024];
        for i in 1..values.len() {
            values[i] = values[i - 1] + (i as f64).ln();
        }
        values
    });
    if n < TABLE.len() {
        return TABLE[n];
    }
    let x = n as f64;
    // Stirling series; the omitted term is below 1e-18 here.
    (x + 0.5) * x.ln() - x + 0.5 * TAU.ln() + 1.0 / (12.0 * x) - 1.0 / (360.0 * x.powi(3))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splitmix_sequence_and_noise_moments_have_independent_oracles() {
        // Published SplitMix64 stream for state zero (wrapping u64 arithmetic).
        let mut random = SplitMix64::new(0, 0);
        assert_eq!(random.next(), 0xe220a8397b1dcdaf);
        assert_eq!(random.next(), 0x6e789e6aa1b965f4);
        assert_eq!(random.next(), 0x06c45d188009454f);
        for mean in [0.0, 4.0, 128.0, 2000.0] {
            let mut sum = 0.0;
            let mut squares = 0.0;
            for _ in 0..20_000 {
                let value = random.poisson(mean);
                assert_eq!(value.fract(), 0.0);
                sum += value;
                squares += (value - mean).powi(2);
            }
            // Poisson mean and variance both equal lambda; 6 standard errors.
            assert!((sum / 20_000.0 - mean).abs() <= 6.0 * (mean / 20_000.0).sqrt());
            assert!((squares / 20_000.0 - mean).abs() <= 0.06 * mean);
        }
        let normals: Vec<_> = (0..20_000).map(|_| random.normal()).collect();
        assert!((normals.iter().sum::<f64>() / 20_000.0).abs() < 0.04);
        assert!((normals.iter().map(|n| n * n).sum::<f64>() / 20_000.0 - 1.0).abs() < 0.06);
    }
}
