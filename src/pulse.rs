//! Signal injection — how information enters the medium.
//!
//! Text is encoded deterministically: blake3(text) seeds a ChaCha stream that
//! places a constellation of signed Gaussian sources. The same text always
//! produces the same wavefront, so recall can later correlate against a
//! freshly re-encoded copy (content addressing, no stored coordinates).

use crate::field::Field;
use rand::Rng;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

/// A parametric pulse — the unit gene of evolutionary discovery.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Pulse {
    /// Center, normalized [0,1] coordinates.
    pub x: f64,
    pub y: f64,
    /// Gaussian radius in normalized units.
    pub radius: f64,
    pub amplitude: f64,
    /// Spatial frequency of the carrier ripple (0 = plain Gaussian).
    pub frequency: f64,
    /// Carrier phase in radians.
    pub phase: f64,
}

impl Pulse {
    /// Render this pulse into a pattern buffer of `size * size`.
    pub fn render(&self, size: usize, out: &mut [f64]) {
        let cx = self.x * size as f64;
        let cy = self.y * size as f64;
        let r = (self.radius * size as f64).max(1.0);
        let reach = (r * 3.0) as isize;
        let (icx, icy) = (cx as isize, cy as isize);
        for dy in -reach..=reach {
            for dx in -reach..=reach {
                let (px, py) = (icx + dx, icy + dy);
                if px < 0 || py < 0 || px >= size as isize || py >= size as isize {
                    continue;
                }
                let fx = px as f64 - cx;
                let fy = py as f64 - cy;
                let d2 = fx * fx + fy * fy;
                let envelope = (-d2 / (2.0 * r * r)).exp();
                let carrier = if self.frequency > 0.0 {
                    ((d2.sqrt() * self.frequency / r) + self.phase).cos()
                } else {
                    1.0
                };
                out[py as usize * size + px as usize] += self.amplitude * envelope * carrier;
            }
        }
    }

    pub fn apply(&self, field: &mut Field) {
        let size = field.size;
        let mut buf = vec![0.0; size * size];
        self.render(size, &mut buf);
        for (u, b) in field.u.iter_mut().zip(buf.iter()) {
            *u += b;
        }
    }
}

/// Deterministically encode text as a wavefront pattern.
pub fn encode_text(text: &str, size: usize) -> Vec<f64> {
    let hash = blake3::hash(text.as_bytes());
    let mut seed = [0u8; 32];
    seed.copy_from_slice(hash.as_bytes());
    let mut rng = ChaCha8Rng::from_seed(seed);

    let mut pattern = vec![0.0; size * size];
    // A constellation of 24 signed sources: enough structure to be
    // distinctive, sparse enough that different texts stay near-orthogonal.
    // Radii and carrier frequencies are band-limited — several grid cells
    // wide — so the encoding lives in smooth propagating modes, not in
    // grid-scale modes that the scheme's artificial viscosity (rightly)
    // dissipates.
    for _ in 0..24 {
        let pulse = Pulse {
            x: rng.gen_range(0.15..0.85),
            y: rng.gen_range(0.15..0.85),
            radius: rng.gen_range(0.05..0.12),
            amplitude: if rng.gen_bool(0.5) { 1.0 } else { -1.0 },
            frequency: rng.gen_range(0.5..2.5),
            phase: rng.gen_range(0.0..std::f64::consts::TAU),
        };
        pulse.render(size, &mut pattern);
    }
    pattern
}

/// Inject encoded text into the field, scaled by importance.
pub fn write_text(field: &mut Field, text: &str, importance: f64) -> Vec<f64> {
    let pattern = encode_text(text, field.size);
    for (u, p) in field.u.iter_mut().zip(pattern.iter()) {
        *u += p * importance;
    }
    pattern
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_encoding_is_deterministic() {
        let a = encode_text("hello kannaka", 64);
        let b = encode_text("hello kannaka", 64);
        assert_eq!(a, b);
    }

    #[test]
    fn different_texts_are_near_orthogonal() {
        let size = 64;
        let a = encode_text("alpha structure", size);
        let b = encode_text("totally different memory", size);
        let dot: f64 = a.iter().zip(&b).map(|(x, y)| x * y).sum();
        let na: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
        let nb: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
        assert!((dot / (na * nb)).abs() < 0.3, "encodings too correlated");
    }

    #[test]
    fn pulse_renders_energy_at_center() {
        let mut buf = vec![0.0; 64 * 64];
        let p = Pulse {
            x: 0.5,
            y: 0.5,
            radius: 0.05,
            amplitude: 1.0,
            frequency: 0.0,
            phase: 0.0,
        };
        p.render(64, &mut buf);
        assert!(buf[32 * 64 + 32] > 0.9);
        assert!(buf[0] == 0.0);
    }
}
