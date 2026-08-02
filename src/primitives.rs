//! Crystal Primitives — repeatable, stable informational geometries detected
//! in a stability map: Echo Rings, Standing Echoes, Harmonic Bridges,
//! Phase Knots, Attractor Fields, Memory Seeds.

use serde::{Deserialize, Serialize};

/// **Morphological** primitive classes (ADR-0004 §5): observational
/// labels from shape heuristics, never behavioral claims. A structure
/// the heuristics cannot place confidently is `Unknown` — the system
/// does not force every region into a named class.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PrimitiveClass {
    /// Annular structure — energy on a ring, quiet core.
    EchoRing,
    /// Compact, roughly circular standing structure.
    StandingEcho,
    /// Strongly elongated structure connecting two regions.
    HarmonicBridge,
    /// Multiple lobes twisted around a shared center.
    PhaseKnot,
    /// Large diffuse basin that keeps drawing energy in.
    AttractorField,
    /// Small, very dense, very stable kernel.
    MemorySeed,
    /// No morphological heuristic fired with confidence.
    Unknown,
}

impl std::fmt::Display for PrimitiveClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            PrimitiveClass::EchoRing => "Echo Ring",
            PrimitiveClass::StandingEcho => "Standing Echo",
            PrimitiveClass::HarmonicBridge => "Harmonic Bridge",
            PrimitiveClass::PhaseKnot => "Phase Knot",
            PrimitiveClass::AttractorField => "Attractor Field",
            PrimitiveClass::MemorySeed => "Memory Seed",
            PrimitiveClass::Unknown => "Unknown",
        };
        write!(f, "{s}")
    }
}

/// The raw morphology features a classification was derived from
/// (ADR-0004 §5) — kept so a future classifier version can re-classify
/// without re-running the experiment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MorphologyFeatures {
    pub relative_area: f64,
    pub elongation: f64,
    pub annularity: f64,
    pub angular_gap_count: usize,
    pub occupied_bins: usize,
    pub stability_ratio: f64,
}

/// Explicit classification metadata (ADR-0004 §5): the label, the
/// domain it belongs to (morphological, never behavioral), the
/// classifier version that produced it, and how confident it was.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Classification {
    pub display_class: String,
    pub primitive_domain: String,
    pub classifier_version: String,
    pub classifier_confidence: f64,
    pub features: MorphologyFeatures,
}

/// A detected (not yet registered) structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedStructure {
    pub class: PrimitiveClass,
    /// Full classification metadata for the label above.
    pub classification: Classification,
    pub centroid: (f64, f64),
    /// Cell count of the connected region.
    pub area: usize,
    /// Mean stability inside the region relative to the map mean (>1 = stable).
    pub stability_score: f64,
    /// 16x16 normalized signature of the region — used for novelty /
    /// similarity search and hashing.
    pub signature: Vec<f64>,
}

/// Detect connected high-stability regions in a stability map.
pub fn detect_structures(stability: &[f64], size: usize) -> Vec<DetectedStructure> {
    let mean = stability.iter().sum::<f64>() / stability.len() as f64;
    if mean < 1e-12 {
        return Vec::new();
    }
    let var = stability
        .iter()
        .map(|s| (s - mean) * (s - mean))
        .sum::<f64>()
        / stability.len() as f64;
    // Half a standard deviation above the mean: consolidated fields are
    // smooth, so a full-sigma cut misses genuinely persistent plateaus.
    let threshold = mean + 0.5 * var.sqrt();

    let mut visited = vec![false; size * size];
    let mut out = Vec::new();
    let min_area = (size * size) / 512; // ignore specks

    for start in 0..size * size {
        if visited[start] || stability[start] < threshold {
            continue;
        }
        // Flood fill.
        let mut cells = Vec::new();
        let mut stack = vec![start];
        visited[start] = true;
        while let Some(i) = stack.pop() {
            cells.push(i);
            let (x, y) = (i % size, i / size);
            let mut push = |nx: isize, ny: isize| {
                if nx >= 0 && ny >= 0 && (nx as usize) < size && (ny as usize) < size {
                    let j = ny as usize * size + nx as usize;
                    if !visited[j] && stability[j] >= threshold {
                        visited[j] = true;
                        stack.push(j);
                    }
                }
            };
            push(x as isize - 1, y as isize);
            push(x as isize + 1, y as isize);
            push(x as isize, y as isize - 1);
            push(x as isize, y as isize + 1);
        }
        if cells.len() < min_area.max(4) {
            continue;
        }
        out.push(analyze_region(&cells, stability, size, mean));
    }
    // Most stable first.
    out.sort_by(|a, b| b.stability_score.total_cmp(&a.stability_score));
    out
}

fn analyze_region(
    cells: &[usize],
    stability: &[f64],
    size: usize,
    map_mean: f64,
) -> DetectedStructure {
    let n = cells.len() as f64;
    let (mut cx, mut cy, mut mass) = (0.0, 0.0, 0.0);
    for &i in cells {
        let (x, y) = ((i % size) as f64, (i / size) as f64);
        let w = stability[i];
        cx += x * w;
        cy += y * w;
        mass += w;
    }
    cx /= mass.max(1e-12);
    cy /= mass.max(1e-12);

    // Shape statistics: radial profile + covariance for elongation.
    let (mut sxx, mut syy, mut sxy) = (0.0, 0.0, 0.0);
    let mut radii: Vec<f64> = Vec::with_capacity(cells.len());
    for &i in cells {
        let dx = (i % size) as f64 - cx;
        let dy = (i / size) as f64 - cy;
        sxx += dx * dx;
        syy += dy * dy;
        sxy += dx * dy;
        radii.push((dx * dx + dy * dy).sqrt());
    }
    sxx /= n;
    syy /= n;
    sxy /= n;
    // Eigenvalues of the 2x2 covariance -> elongation ratio.
    let tr = sxx + syy;
    let det = sxx * syy - sxy * sxy;
    let disc = (tr * tr / 4.0 - det).max(0.0).sqrt();
    let (l1, l2) = (tr / 2.0 + disc, (tr / 2.0 - disc).max(1e-9));
    let elongation = (l1 / l2).sqrt();

    let mean_r = radii.iter().sum::<f64>() / n;
    let min_r = radii.iter().cloned().fold(f64::INFINITY, f64::min);
    // Annularity: a ring has a hole — its minimum radius is a large
    // fraction of its mean radius.
    let annularity = if mean_r > 1e-9 { min_r / mean_r } else { 0.0 };

    let region_mean = mass / n;
    let stability_score = region_mean / map_mean.max(1e-12);
    let area = cells.len();
    let field_area = size * size;

    // Lobe count via angular histogram occupancy gaps.
    let mut ang_hist = [0u32; 16];
    for &i in cells {
        let dx = (i % size) as f64 - cx;
        let dy = (i / size) as f64 - cy;
        let a = dy.atan2(dx) + std::f64::consts::PI;
        let bin = ((a / std::f64::consts::TAU) * 16.0) as usize % 16;
        ang_hist[bin] += 1;
    }
    let occupied = ang_hist.iter().filter(|c| **c > 0).count();
    let mut gaps = 0;
    for k in 0..16 {
        if ang_hist[k] > 0 && ang_hist[(k + 1) % 16] == 0 {
            gaps += 1;
        }
    }

    let features = MorphologyFeatures {
        relative_area: area as f64 / field_area as f64,
        elongation,
        annularity,
        angular_gap_count: gaps,
        occupied_bins: occupied,
        stability_ratio: stability_score,
    };
    let (class, confidence) = classify(&features);

    DetectedStructure {
        class,
        classification: Classification {
            display_class: class.to_string(),
            primitive_domain: "morphological".into(),
            classifier_version: crate::versions::CLASSIFIER_VERSION.into(),
            classifier_confidence: confidence,
            features,
        },
        centroid: (cx / size as f64, cy / size as f64),
        area,
        stability_score,
        signature: signature(cells, stability, size, cx, cy),
    }
}

/// First-match morphological heuristics with a margin-derived confidence
/// in [0.3, 0.95]: how far past its decision threshold the winning rule
/// fired. A region that only reaches the fallthrough with weak stability
/// is `Unknown` — better an honest non-answer than a forced label
/// (ADR-0004 §5).
fn classify(f: &MorphologyFeatures) -> (PrimitiveClass, f64) {
    let conf = |margin: f64| 0.55 + 0.4 * margin.clamp(0.0, 1.0);
    if f.annularity > 0.45 && f.occupied_bins >= 12 {
        let margin = ((f.annularity - 0.45) / 0.3).min((f.occupied_bins as f64 - 12.0) / 4.0);
        (PrimitiveClass::EchoRing, conf(margin))
    } else if f.elongation > 2.5 {
        (
            PrimitiveClass::HarmonicBridge,
            conf((f.elongation - 2.5) / 2.0),
        )
    } else if f.angular_gap_count >= 3 {
        (
            PrimitiveClass::PhaseKnot,
            conf((f.angular_gap_count as f64 - 2.0) / 3.0),
        )
    } else if f.relative_area > 0.08 {
        (
            PrimitiveClass::AttractorField,
            conf((f.relative_area - 0.08) / 0.08),
        )
    } else if f.relative_area < 0.01 && f.stability_ratio > 3.0 {
        (
            PrimitiveClass::MemorySeed,
            conf((f.stability_ratio - 3.0) / 2.0),
        )
    } else if f.stability_ratio >= 1.3 {
        (
            PrimitiveClass::StandingEcho,
            conf((f.stability_ratio - 1.3) / 1.5),
        )
    } else {
        // Barely above the field mean and no shape heuristic fired.
        (PrimitiveClass::Unknown, 0.3)
    }
}

/// 16x16 normalized signature: the region's stability resampled around its
/// centroid. Rotation-naive but translation-invariant — good enough for
/// novelty detection and similarity search in the MVP.
fn signature(cells: &[usize], stability: &[f64], size: usize, cx: f64, cy: f64) -> Vec<f64> {
    const S: usize = 16;
    let max_r = cells
        .iter()
        .map(|&i| {
            let dx = (i % size) as f64 - cx;
            let dy = (i / size) as f64 - cy;
            (dx * dx + dy * dy).sqrt()
        })
        .fold(1.0f64, f64::max);
    let mut sig = vec![0.0; S * S];
    for &i in cells {
        let dx = ((i % size) as f64 - cx) / max_r; // [-1, 1]
        let dy = ((i / size) as f64 - cy) / max_r;
        let sx = (((dx + 1.0) / 2.0) * (S - 1) as f64).round() as usize;
        let sy = (((dy + 1.0) / 2.0) * (S - 1) as f64).round() as usize;
        sig[sy.min(S - 1) * S + sx.min(S - 1)] += stability[i];
    }
    let norm: f64 = sig.iter().map(|v| v * v).sum::<f64>().sqrt();
    if norm > 1e-12 {
        for v in sig.iter_mut() {
            *v /= norm;
        }
    }
    sig
}

/// Cosine similarity between two signatures.
pub fn signature_similarity(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(x, y)| x * y)
        .sum::<f64>()
        .clamp(-1.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blob(size: usize, cx: usize, cy: usize, r: f64) -> Vec<f64> {
        let mut m = vec![0.01; size * size];
        for y in 0..size {
            for x in 0..size {
                let d2 = (x as f64 - cx as f64).powi(2) + (y as f64 - cy as f64).powi(2);
                m[y * size + x] += (-d2 / (2.0 * r * r)).exp();
            }
        }
        m
    }

    #[test]
    fn detects_a_compact_blob_as_standing_structure() {
        let size = 64;
        let m = blob(size, 32, 32, 4.0);
        let found = detect_structures(&m, size);
        assert!(!found.is_empty(), "no structure detected");
        let s = &found[0];
        assert!((s.centroid.0 - 0.5).abs() < 0.1);
        assert!(matches!(
            s.class,
            PrimitiveClass::StandingEcho | PrimitiveClass::MemorySeed
        ));
    }

    #[test]
    fn detects_a_ring_as_echo_ring() {
        let size = 64;
        let mut m = vec![0.01; size * size];
        for y in 0..size {
            for x in 0..size {
                let d = ((x as f64 - 32.0).powi(2) + (y as f64 - 32.0).powi(2)).sqrt();
                if (d - 12.0).abs() < 2.5 {
                    m[y * size + x] = 1.0;
                }
            }
        }
        let found = detect_structures(&m, size);
        assert!(!found.is_empty());
        assert_eq!(
            found[0].class,
            PrimitiveClass::EchoRing,
            "got {:?}",
            found[0]
        );
    }

    #[test]
    fn similar_blobs_have_similar_signatures() {
        let size = 64;
        let a = detect_structures(&blob(size, 20, 20, 4.0), size);
        let b = detect_structures(&blob(size, 44, 40, 4.0), size);
        assert!(!a.is_empty() && !b.is_empty());
        let sim = signature_similarity(&a[0].signature, &b[0].signature);
        assert!(sim > 0.8, "translated blobs should match: {sim}");
    }

    #[test]
    fn empty_map_detects_nothing() {
        assert!(detect_structures(&vec![0.0; 64 * 64], 64).is_empty());
    }
}
