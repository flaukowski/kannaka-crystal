//! Primitive Discovery Engine — evolutionary search over injection genomes.
//!
//! A genome is a small set of pulses plus a resonance schedule. Fitness is
//! persistence (how much of the injected structure survives observation)
//! plus a novelty bonus (distance from everything already in the registry).
//! Survivors are mutated; stable, novel structures are registered with
//! lineage back to the genomes that produced them.

use crate::dream::{dream, DreamMode};
use crate::engine::CrystalEngine;
use crate::field::seeded_rng;
use crate::primitives::detect_structures;
use crate::pulse::Pulse;
use crate::registry::{Primitive, Registry};
use rand::Rng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Genome {
    pub pulses: Vec<Pulse>,
    /// Steps of free evolution before scoring.
    pub resonate_steps: u64,
    /// Whether a deep dream runs before scoring.
    pub dream_first: bool,
}

impl Genome {
    fn random(rng: &mut ChaCha8Rng) -> Self {
        let n = rng.gen_range(1..=4);
        Genome {
            pulses: (0..n).map(|_| random_pulse(rng)).collect(),
            resonate_steps: rng.gen_range(100..600),
            dream_first: rng.gen_bool(0.5),
        }
    }

    fn mutate(&self, rng: &mut ChaCha8Rng) -> Self {
        let mut g = self.clone();
        match rng.gen_range(0..5) {
            0 if g.pulses.len() > 1 => {
                let i = rng.gen_range(0..g.pulses.len());
                g.pulses.remove(i);
            }
            1 if g.pulses.len() < 6 => g.pulses.push(random_pulse(rng)),
            2 => {
                let i = rng.gen_range(0..g.pulses.len());
                let p = &mut g.pulses[i];
                p.x = (p.x + rng.gen_range(-0.1..0.1)).clamp(0.05, 0.95);
                p.y = (p.y + rng.gen_range(-0.1..0.1)).clamp(0.05, 0.95);
                p.radius = (p.radius * rng.gen_range(0.7..1.4)).clamp(0.04, 0.2);
            }
            3 => {
                let i = rng.gen_range(0..g.pulses.len());
                let p = &mut g.pulses[i];
                p.frequency = (p.frequency + rng.gen_range(-1.0..1.0)).clamp(0.0, 4.0);
                p.phase = (p.phase + rng.gen_range(-0.8..0.8)).rem_euclid(std::f64::consts::TAU);
                p.amplitude = (p.amplitude * rng.gen_range(0.7..1.4)).clamp(-3.0, 3.0);
            }
            _ => {
                g.resonate_steps =
                    ((g.resonate_steps as f64 * rng.gen_range(0.7..1.4)) as u64).clamp(50, 1200);
                g.dream_first = rng.gen_bool(0.5);
            }
        }
        g
    }
}

fn random_pulse(rng: &mut ChaCha8Rng) -> Pulse {
    // Band-limited like the text encoder: sub-grid-scale pulses dissipate
    // under the scheme's artificial viscosity before they can be scored.
    Pulse {
        x: rng.gen_range(0.15..0.85),
        y: rng.gen_range(0.15..0.85),
        radius: rng.gen_range(0.05..0.15),
        amplitude: rng.gen_range(0.4..2.0) * if rng.gen_bool(0.5) { 1.0 } else { -1.0 },
        frequency: rng.gen_range(0.0..3.0),
        phase: rng.gen_range(0.0..std::f64::consts::TAU),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionConfig {
    pub material_id: String,
    pub generations: usize,
    pub population: usize,
    pub field_size: usize,
    pub seed: u64,
    /// Extra noise injected during the noise-tolerance re-run.
    pub noise_probe_amp: f64,
}

impl Default for EvolutionConfig {
    fn default() -> Self {
        EvolutionConfig {
            material_id: "ideal_resonator".into(),
            generations: 10,
            population: 12,
            field_size: 96,
            seed: 0,
            noise_probe_amp: 0.02,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionReport {
    pub generations_run: usize,
    pub evaluated: usize,
    pub best_fitness: f64,
    pub discovered: Vec<Primitive>,
    /// ADR-0004 §2: the manifest for this evolution run. Callers persist
    /// it with `report.manifest.save()`; every primitive registered by
    /// the run links back to it.
    pub manifest: crate::manifest::ExperimentManifest,
}

struct Scored {
    genome: Genome,
    fitness: f64,
}

/// Run one evaluation: inject genome, evolve, measure persistence + detect
/// structures. Returns (persistence, noise_tolerance, structures, energy profile).
fn evaluate(
    genome: &Genome,
    cfg: &EvolutionConfig,
    noise: f64,
) -> (f64, Vec<crate::primitives::DetectedStructure>, Vec<f64>) {
    let mut engine = CrystalEngine::new(&cfg.material_id, cfg.field_size, cfg.seed)
        .expect("material validated by caller");
    engine.noise_amp = noise;
    for p in &genome.pulses {
        engine.pulse(p);
    }
    // Settle before baselining: right after injection u_prev is zero, so the
    // energy metric sees a huge velocity impulse that inflates e0 and makes
    // every retention ratio look terrible.
    engine.resonate(20);
    let initial: Vec<f64> = engine.field.u.clone();
    let e0 = engine.field.energy();

    let mut energy_profile = vec![e0];
    let chunks = 4u64;
    for _ in 0..chunks {
        engine.resonate(genome.resonate_steps / chunks);
        energy_profile.push(engine.field.energy());
    }
    if genome.dream_first {
        dream(&mut engine, DreamMode::Deep);
    }

    // Persistence: how much the surviving field still correlates with what
    // was injected (envelope — structure, not phase), weighted by how much
    // energy survived.
    let corr = engine.field.correlate_envelope(&initial);
    let e1 = engine.field.energy();
    let retention = if e0 > 1e-12 { (e1 / e0).min(1.0) } else { 0.0 };
    let persistence = (corr * 0.7 + retention * 0.3).clamp(0.0, 1.0);

    // Structures from a fresh stability observation.
    let report = dream(&mut engine, DreamMode::Light);
    let structures = detect_structures(&report.stability_map, cfg.field_size);

    (persistence, structures, energy_profile)
}

/// Evolutionary discovery loop. Registers novel stable structures into
/// `registry` (caller saves). Progress lines go through `progress`.
pub fn evolve(
    cfg: &EvolutionConfig,
    registry: &mut Registry,
    mut progress: impl FnMut(String),
) -> EvolutionReport {
    // ADR-0004 §2: the manifest is the experiment's identity; every
    // primitive this run registers links to it by (id, protocol hash).
    let material =
        crate::material::find_material(&cfg.material_id).expect("material validated by caller");
    let mut manifest = crate::manifest::ExperimentManifest::new(
        &material,
        cfg.field_size,
        cfg.seed,
        material.default_temperature_k,
        0.0,
        serde_json::json!({ "kind": "evolve", "config": cfg }),
    );
    let experiment = (manifest.experiment_id, manifest.experiment_hash());

    let mut rng = seeded_rng(cfg.seed.wrapping_add(0xC0FFEE));
    let mut population: Vec<Scored> = (0..cfg.population)
        .map(|_| Scored {
            genome: Genome::random(&mut rng),
            fitness: 0.0,
        })
        .collect();

    let mut discovered = Vec::new();
    let mut evaluated = 0usize;
    let mut best_fitness = 0.0f64;

    for gen in 0..cfg.generations {
        for s in population.iter_mut() {
            let (persistence, structures, energy_profile) = evaluate(&s.genome, cfg, 0.0);
            evaluated += 1;

            // Novelty: best structure's distance from the registry.
            let novelty = structures
                .first()
                .map(|st| 1.0 - registry.max_similarity(&st.signature))
                .unwrap_or(0.0);
            s.fitness = persistence + 0.5 * novelty;
            best_fitness = best_fitness.max(s.fitness);

            // Register anything stable AND novel.
            for st in structures.iter().take(2) {
                if persistence < 0.2 || registry.max_similarity(&st.signature) >= 0.92 {
                    continue;
                }
                // Noise-tolerance probe: re-run the same genome under noise.
                let (noisy_persistence, _, _) = evaluate(&s.genome, cfg, cfg.noise_probe_amp);
                evaluated += 1;
                let noise_tolerance = if persistence > 1e-9 {
                    (noisy_persistence / persistence).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let lineage: Vec<String> = discovered
                    .iter()
                    .rev()
                    .take(1)
                    .map(|p: &Primitive| p.id.clone())
                    .collect();
                if let Some(prim) = registry.register(
                    st,
                    persistence,
                    noise_tolerance,
                    energy_profile.clone(),
                    &cfg.material_id,
                    lineage,
                    &format!("evolve gen {gen}"),
                    Some(experiment.clone()),
                ) {
                    progress(format!(
                        "  [gen {gen}] discovered {} ({}) persistence={:.1}% noise-tol={:.1}%",
                        prim.id,
                        prim.class,
                        prim.persistence * 100.0,
                        prim.noise_tolerance * 100.0
                    ));
                    discovered.push(prim);
                }
            }
        }

        // Selection: keep the top half, refill with mutants of survivors.
        population.sort_by(|a, b| b.fitness.total_cmp(&a.fitness));
        let keep = (cfg.population / 2).max(1);
        population.truncate(keep);
        while population.len() < cfg.population {
            let parent = rng.gen_range(0..keep);
            let child = population[parent].genome.mutate(&mut rng);
            population.push(Scored {
                genome: child,
                fitness: 0.0,
            });
        }
        progress(format!(
            "generation {gen}: best fitness {:.3}, {} primitives total",
            population.first().map(|s| s.fitness).unwrap_or(0.0),
            registry.primitives.len()
        ));
    }

    manifest.results = serde_json::json!({
        "evaluated": evaluated,
        "best_fitness": best_fitness,
        "discovered": discovered.iter().map(|p| p.id.clone()).collect::<Vec<_>>(),
    });
    EvolutionReport {
        generations_run: cfg.generations,
        evaluated,
        best_fitness,
        discovered,
        manifest,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evolution_discovers_at_least_one_primitive_in_resonator() {
        let cfg = EvolutionConfig {
            generations: 2,
            population: 4,
            field_size: 64,
            seed: 5,
            ..Default::default()
        };
        let mut registry = Registry::default();
        let report = evolve(&cfg, &mut registry, |_| {});
        assert!(report.evaluated >= 8);
        assert!(
            !registry.primitives.is_empty(),
            "ideal resonator should yield at least one stable structure"
        );
        // Genealogy sanity: any lineage entries reference real primitives.
        for p in &registry.primitives {
            for parent in &p.lineage {
                assert!(registry.find(parent).is_some(), "dangling lineage {parent}");
            }
        }
        // ADR-0004: every primitive from this run links to its manifest,
        // and the manifest's protocol hash matches.
        let hash = report.manifest.experiment_hash();
        for p in &registry.primitives {
            assert_eq!(p.experiment_id, Some(report.manifest.experiment_id));
            assert_eq!(p.experiment_hash.as_deref(), Some(hash.as_str()));
            assert!(
                p.classification.is_some(),
                "classification metadata missing"
            );
        }
    }

    #[test]
    fn mutation_keeps_genomes_in_bounds() {
        let mut rng = seeded_rng(99);
        let mut g = Genome::random(&mut rng);
        for _ in 0..200 {
            g = g.mutate(&mut rng);
            assert!(!g.pulses.is_empty() && g.pulses.len() <= 6);
            for p in &g.pulses {
                assert!((0.0..=1.0).contains(&p.x) && (0.0..=1.0).contains(&p.y));
            }
            assert!(g.resonate_steps >= 50 && g.resonate_steps <= 1200);
        }
    }
}
