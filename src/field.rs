//! The resonant field — a 2D scalar wave medium evolved with a leapfrog
//! integrator. This is the substrate everything else (injection, decay,
//! dreaming, primitive discovery) operates on.

use crate::material::Material;
use rand::Rng;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

pub const DEFAULT_SIZE: usize = 128;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    pub size: usize,
    /// Current displacement u(t).
    pub u: Vec<f64>,
    /// Previous displacement u(t-1) — needed by the leapfrog step.
    pub u_prev: Vec<f64>,
    /// Simulation step counter.
    pub step_count: u64,
}

impl Field {
    pub fn new(size: usize) -> Self {
        Field {
            size,
            u: vec![0.0; size * size],
            u_prev: vec![0.0; size * size],
            step_count: 0,
        }
    }

    #[inline]
    pub fn idx(&self, x: usize, y: usize) -> usize {
        y * self.size + x
    }

    pub fn at(&self, x: usize, y: usize) -> f64 {
        self.u[self.idx(x, y)]
    }

    /// Add a value into the field (signals superpose — injection never
    /// overwrites what is already resonating).
    pub fn inject_at(&mut self, x: usize, y: usize, value: f64) {
        if x < self.size && y < self.size {
            let i = self.idx(x, y);
            self.u[i] += value;
        }
    }

    /// One leapfrog step of the damped, softly-saturating wave equation:
    ///
    /// u_next = 2u - u_prev + c^2 * lap(u) - damping*(u - u_prev) - nl*u^3 + noise
    ///
    /// Boundary cells reflect with the material's reflection coefficient.
    /// Decay here is *bulk* damping; per-structure decay emerges from
    /// geometry (leaky boundaries disperse what mirrors would hold).
    pub fn step(
        &mut self,
        material: &Material,
        temperature_k: f64,
        noise_amp: f64,
        rng: &mut ChaCha8Rng,
    ) {
        let n = self.size;
        let c2 = material.wave_speed * material.wave_speed;
        let damping = material.damping;
        let nl = material.nonlinearity;
        let thermal = material.thermal_noise(temperature_k) + noise_amp;

        // Kelvin–Voigt artificial viscosity: damps the laplacian of the
        // *velocity*. Grid-scale (near-Nyquist) modes have near-zero group
        // velocity in the discrete dispersion relation — without this they
        // sit in place forever and never reach the boundary sponge, so even
        // an absorbing medium appears lossless. Smooth waves see almost none
        // of it. A property of the discretization, not of any material.
        const VISCOSITY: f64 = 0.01;

        let mut u_next = vec![0.0; n * n];
        for y in 1..n - 1 {
            for x in 1..n - 1 {
                let i = y * n + x;
                let lap =
                    self.u[i - 1] + self.u[i + 1] + self.u[i - n] + self.u[i + n] - 4.0 * self.u[i];
                let lap_prev = self.u_prev[i - 1]
                    + self.u_prev[i + 1]
                    + self.u_prev[i - n]
                    + self.u_prev[i + n]
                    - 4.0 * self.u_prev[i];
                let u = self.u[i];
                // Bounded saturation: u^3/(1+u^2) ~ u^3 for small amplitudes
                // but tends to u for large ones. A raw cubic is explicit-
                // integration unstable above |u| ~ 1/sqrt(nl) — cells ring
                // against any hard clamp at full amplitude forever.
                let sat = u * u * u / (1.0 + u * u);
                let mut v =
                    2.0 * u - self.u_prev[i] + c2 * lap - damping * (u - self.u_prev[i]) - nl * sat
                        + VISCOSITY * (lap - lap_prev);
                if thermal > 0.0 {
                    v += rng.gen_range(-thermal..thermal);
                }
                // Physical saturation ceiling. The explicit cubic term is
                // conditionally stable: for large |u| it overshoots with
                // alternating sign and diverges to NaN, which then poisons
                // every correlation and fitness sort downstream. Real media
                // saturate; so does this one.
                u_next[i] = v.clamp(-100.0, 100.0);
            }
        }
        // Boundaries: reflected copy of the adjacent interior cell, scaled.
        let r = material.boundary_reflect;
        for x in 0..n {
            u_next[x] = u_next[n + x] * r;
            u_next[(n - 1) * n + x] = u_next[(n - 2) * n + x] * r;
        }
        for y in 0..n {
            u_next[y * n] = u_next[y * n + 1] * r;
            u_next[y * n + n - 1] = u_next[y * n + n - 2] * r;
        }
        // Sponge layer: a hard edge alone acts as a mirror regardless of `r`
        // (a zeroed rim is a Dirichlet wall). Genuine absorption needs a
        // graded damping zone — cells near the edge lose amplitude in
        // proportion to (1 - r), so r=0 swallows outgoing waves and r=1
        // leaves them untouched.
        // Gentle but deep: a sharp absorber is an impedance mismatch and
        // REFLECTS most of the incoming wave. Grading the loss over ~n/8
        // cells lets waves enter the sponge and die inside it.
        if r < 1.0 {
            let margin = (n / 8).max(6);
            for y in 0..n {
                for x in 0..n {
                    let d = x.min(y).min(n - 1 - x).min(n - 1 - y);
                    if d < margin {
                        let depth = (margin - d) as f64 / margin as f64;
                        let factor = 1.0 - (1.0 - r) * 0.15 * depth * depth;
                        u_next[y * n + x] *= factor;
                        self.u[y * n + x] *= factor;
                    }
                }
            }
        }

        self.u_prev = std::mem::take(&mut self.u);
        self.u = u_next;
        self.step_count += 1;
    }

    /// Total field energy: kinetic term (du/dt)^2 plus gradient energy
    /// |grad u|^2. The gradient term (not raw |u|^2) is what the wave
    /// equation conserves — with |u|^2 a slow box-mode's "energy" swings
    /// with its oscillation phase and a uniform offset counts as energy.
    pub fn energy(&self) -> f64 {
        let n = self.size;
        let mut total = 0.0;
        for y in 0..n {
            for x in 0..n {
                let i = y * n + x;
                let v = self.u[i] - self.u_prev[i];
                total += v * v;
                if x + 1 < n {
                    let gx = self.u[i + 1] - self.u[i];
                    total += 0.25 * gx * gx;
                }
                if y + 1 < n {
                    let gy = self.u[i + n] - self.u[i];
                    total += 0.25 * gy * gy;
                }
            }
        }
        total
    }

    /// Normalized correlation of the field against a pattern — the recall
    /// primitive. 1.0 = the pattern is ringing exactly; 0.0 = gone.
    pub fn correlate(&self, pattern: &[f64]) -> f64 {
        assert_eq!(pattern.len(), self.u.len());
        let dot: f64 = self.u.iter().zip(pattern).map(|(a, b)| a * b).sum();
        let na: f64 = self.u.iter().map(|a| a * a).sum::<f64>().sqrt();
        let nb: f64 = pattern.iter().map(|b| b * b).sum::<f64>().sqrt();
        if na < 1e-12 || nb < 1e-12 {
            0.0
        } else {
            (dot / (na * nb)).abs()
        }
    }

    /// Phase-insensitive correlation: cosine of |u| against |pattern|.
    /// Pointwise correlation decays as phases evolve even when the
    /// structure itself survives; the envelope is what persistence means.
    pub fn correlate_envelope(&self, pattern: &[f64]) -> f64 {
        assert_eq!(pattern.len(), self.u.len());
        let dot: f64 = self
            .u
            .iter()
            .zip(pattern)
            .map(|(a, b)| a.abs() * b.abs())
            .sum();
        let na: f64 = self.u.iter().map(|a| a * a).sum::<f64>().sqrt();
        let nb: f64 = pattern.iter().map(|b| b * b).sum::<f64>().sqrt();
        if na < 1e-12 || nb < 1e-12 {
            0.0
        } else {
            dot / (na * nb)
        }
    }

    /// Downsample |u| into a coarse grid for the Observatory / signatures.
    pub fn downsample_abs(&self, out_size: usize) -> Vec<f64> {
        let mut out = vec![0.0; out_size * out_size];
        let scale = self.size as f64 / out_size as f64;
        for oy in 0..out_size {
            for ox in 0..out_size {
                let x0 = (ox as f64 * scale) as usize;
                let y0 = (oy as f64 * scale) as usize;
                let x1 = (((ox + 1) as f64 * scale) as usize).min(self.size);
                let y1 = (((oy + 1) as f64 * scale) as usize).min(self.size);
                let mut acc = 0.0;
                let mut cnt = 0usize;
                for y in y0..y1 {
                    for x in x0..x1 {
                        acc += self.u[y * self.size + x].abs();
                        cnt += 1;
                    }
                }
                out[oy * out_size + ox] = if cnt > 0 { acc / cnt as f64 } else { 0.0 };
            }
        }
        out
    }
}

/// Deterministic RNG for reproducible experiments (seed 0 = default lab).
pub fn seeded_rng(seed: u64) -> ChaCha8Rng {
    ChaCha8Rng::seed_from_u64(seed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::find_material;

    #[test]
    fn wave_propagates_outward() {
        let mat = find_material("ideal_resonator").unwrap();
        let mut f = Field::new(64);
        let mut rng = seeded_rng(1);
        f.inject_at(32, 32, 1.0);
        for _ in 0..10 {
            f.step(&mat, 0.0, 0.0, &mut rng);
        }
        // Energy must have spread beyond the injection point.
        let away = f.at(32 + 5, 32).abs() + f.at(32, 32 + 5).abs();
        assert!(away > 0.0, "wave did not propagate");
    }

    #[test]
    fn vacuum_disperses_faster_than_resonator() {
        let vac = find_material("vacuum").unwrap();
        let res = find_material("ideal_resonator").unwrap();
        let run = |mat| {
            let mut f = Field::new(64);
            let mut rng = seeded_rng(2);
            f.inject_at(32, 32, 1.0);
            for _ in 0..400 {
                f.step(&mat, 0.0, 0.0, &mut rng);
            }
            f.energy()
        };
        assert!(
            run(res) > run(vac) * 2.0,
            "resonator should retain more energy"
        );
    }

    #[test]
    fn correlation_of_identical_pattern_is_high() {
        let mut f = Field::new(32);
        let mut pattern = vec![0.0; 32 * 32];
        for i in (0..pattern.len()).step_by(7) {
            pattern[i] = ((i % 13) as f64 - 6.0) / 6.0;
        }
        f.u = pattern.clone();
        assert!(f.correlate(&pattern) > 0.999);
    }

    #[test]
    fn extreme_amplitudes_never_produce_nan() {
        // Regression: the explicit cubic term diverges to NaN for large |u|
        // without the saturation clamp (seen live: `evolve --seed 42`
        // panicked in a fitness sort on NaN persistence).
        let mat = find_material("metamaterial").unwrap();
        let mut f = Field::new(64);
        let mut rng = seeded_rng(42);
        for i in 0..20 {
            f.inject_at(22 + i, 32, 40.0);
            f.inject_at(22 + i, 33, -40.0);
        }
        for _ in 0..2000 {
            f.step(&mat, 293.0, 0.05, &mut rng);
        }
        assert!(f.u.iter().all(|v| v.is_finite()), "field went NaN");
        assert!(f.energy().is_finite());
    }

    #[test]
    fn energy_is_finite_under_nonlinearity() {
        let mat = find_material("metamaterial").unwrap();
        let mut f = Field::new(64);
        let mut rng = seeded_rng(3);
        for i in 0..10 {
            f.inject_at(20 + i, 32, 2.0);
        }
        for _ in 0..1000 {
            f.step(&mat, 293.0, 0.0, &mut rng);
        }
        assert!(f.energy().is_finite(), "field blew up");
    }
}
