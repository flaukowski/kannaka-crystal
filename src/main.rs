use clap::{Parser, Subcommand};
use kannaka_crystal::discovery::{evolve, EvolutionConfig};
use kannaka_crystal::dream::{dream, DreamMode};
use kannaka_crystal::engine::CrystalEngine;
use kannaka_crystal::field::DEFAULT_SIZE;
use kannaka_crystal::lang::run_program;
use kannaka_crystal::material::builtin_materials;
use kannaka_crystal::registry::Registry;

#[derive(Parser)]
#[command(
    name = "kannaka-crystal",
    version,
    about = "Kannaka Crystal — informational materials & resonant memory systems"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Serve the REST API + Observatory UI
    Serve {
        #[arg(long, default_value = "127.0.0.1:3339")]
        bind: String,
        #[arg(long, default_value = "ideal_resonator")]
        material: String,
        #[arg(long, default_value_t = DEFAULT_SIZE)]
        size: usize,
        #[arg(long, default_value_t = 0)]
        seed: u64,
    },
    /// Execute a .crystal program file
    Run {
        /// Path to a .crystal file
        file: String,
        #[arg(long, default_value = "ideal_resonator")]
        material: String,
        #[arg(long, default_value_t = 0)]
        seed: u64,
    },
    /// Evolutionary search for novel primitives
    Evolve {
        #[arg(long, default_value = "ideal_resonator")]
        material: String,
        #[arg(long, default_value_t = 10)]
        generations: usize,
        #[arg(long, default_value_t = 12)]
        population: usize,
        #[arg(long, default_value_t = 0)]
        seed: u64,
        /// Robust mode: fitness includes in-loop survival across shifted
        /// seeds x noise — selects attractors, not one-trajectory artifacts
        #[arg(long)]
        robust: bool,
        /// Seeds per noise level for the robust ensemble
        #[arg(long, default_value_t = 3)]
        robust_seeds: u64,
    },
    /// Run a standalone dream (consolidation) experiment
    Dream {
        #[arg(long, default_value = "ideal_resonator")]
        material: String,
        #[arg(long, default_value = "deep")]
        mode: String,
        /// Text memories to write before dreaming
        #[arg(long)]
        write: Vec<String>,
    },
    /// List built-in materials
    Materials,
    /// Inspect and search the primitive registry
    Primitives {
        /// Show a single primitive by id (CRY-###### or UUID)
        id: Option<String>,
        /// Export the full registry as JSON to stdout
        #[arg(long)]
        export: bool,
        /// Filter by class (e.g. standing_echo, "Echo Ring")
        #[arg(long)]
        class: Option<String>,
        /// Filter by material id
        #[arg(long)]
        material: Option<String>,
        /// Minimum persistence in [0,1]
        #[arg(long, default_value_t = 0.0)]
        min_persistence: f64,
        /// Minimum evidence ladder level 0-8 (ADR-0004 §9)
        #[arg(long, default_value_t = 0)]
        min_evidence: u8,
        /// Require a PASSED behavioral capability
        /// (noise_shielding | pattern_completion)
        #[arg(long)]
        capability: Option<String>,
        /// Rank by structural similarity to a primitive id
        #[arg(long)]
        similar: Option<String>,
    },
    /// Promote a primitive up the evidence ladder through a recorded
    /// procedure (ADR-0004 §9): replicate (L2), perturb (L3), resolution (L4)
    Promote {
        /// Primitive id (CRY-###### or UUID)
        id: String,
        /// replicate | perturb | resolution | behavior
        #[arg(long, default_value = "replicate")]
        procedure: String,
        /// Behavioral contract to run when --procedure behavior
        /// (noise_shielding | pattern_completion)
        #[arg(long)]
        capability: Option<String>,
        /// Trials for the behavioral contract
        #[arg(long, default_value_t = 10)]
        trials: u64,
        /// Seeds for the perturbation ensemble (§8 standard: 8)
        #[arg(long, default_value_t = 8)]
        seeds: u64,
    },
    /// Run the KCB-1 benchmark suite: physical-recall benchmarks across
    /// non-resonant baselines and mechanism ablations (ADR-0004 §10)
    Bench {
        #[arg(long, default_value = "ideal_resonator")]
        material: String,
        #[arg(long, default_value_t = 64)]
        size: usize,
        /// Seeds per condition (10-run standard for real comparisons)
        #[arg(long, default_value_t = 10)]
        seeds: u64,
        /// Free-evolution steps between write and probe
        #[arg(long, default_value_t = 300)]
        delay: u64,
    },
    /// Prune the registry to its growth caps (see KANNAKA_CRYSTAL_BUCKET_CAP
    /// / KANNAKA_CRYSTAL_MAX_PRIMITIVES)
    Prune {
        /// Max primitives per class×material bucket (overrides env)
        #[arg(long)]
        bucket_cap: Option<usize>,
        /// Max total primitives (overrides env)
        #[arg(long)]
        total_cap: Option<usize>,
    },
    /// Run a NATS Explorer agent (requires --features swarm build)
    #[cfg(feature = "swarm")]
    Explore {
        /// Comma-separated material ids to rotate through, or "all"
        #[arg(long, default_value = "ideal_resonator")]
        material: String,
        /// Seconds to sleep between search rounds (0 = flat out)
        #[arg(long, default_value_t = 0)]
        interval: u64,
    },
    /// Run a NATS Archivist agent: merge all swarm discoveries into this
    /// node's registry (requires --features swarm build)
    #[cfg(feature = "swarm")]
    Archive,
    /// Publish a primitive to the OpenClawCity gallery (requires --features publish build)
    #[cfg(feature = "publish")]
    Publish {
        /// Primitive id (CRY-###### or UUID)
        id: String,
    },
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = dispatch(cli.command) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn dispatch(command: Command) -> Result<(), String> {
    match command {
        Command::Serve {
            bind,
            material,
            size,
            seed,
        } => kannaka_crystal::api::serve(&bind, &material, size, seed),
        Command::Run {
            file,
            material,
            seed,
        } => {
            let source = std::fs::read_to_string(&file).map_err(|e| format!("{file}: {e}"))?;
            let mut engine = CrystalEngine::new(&material, DEFAULT_SIZE, seed)?;
            let mut registry = Registry::load().map_err(|e| e.to_string())?;
            let report =
                run_program(&source, &mut engine, &mut registry).map_err(|e| e.to_string())?;
            registry.save().map_err(|e| e.to_string())?;
            let manifest_path = report.manifest.save()?;
            eprintln!(
                "experiment {} -> {}",
                report.manifest.experiment_id,
                manifest_path.display()
            );
            println!("{}", serde_json::to_string_pretty(&report).unwrap());
            Ok(())
        }
        Command::Evolve {
            material,
            generations,
            population,
            seed,
            robust,
            robust_seeds,
        } => {
            let cfg = EvolutionConfig {
                material_id: material,
                generations,
                population,
                seed,
                robust_seeds: if robust { robust_seeds } else { 0 },
                ..Default::default()
            };
            kannaka_crystal::material::find_material(&cfg.material_id)
                .ok_or_else(|| format!("unknown material: {}", cfg.material_id))?;
            let mut registry = Registry::load().map_err(|e| e.to_string())?;
            let report = evolve(&cfg, &mut registry, |line| println!("{line}"));
            registry.save().map_err(|e| e.to_string())?;
            let manifest_path = report.manifest.save()?;
            println!(
                "\nevolution complete: {} evaluations, best fitness {:.3}, {} new primitives ({} total)\nexperiment {} -> {}",
                report.evaluated,
                report.best_fitness,
                report.discovered.len(),
                registry.primitives.len(),
                report.manifest.experiment_id,
                manifest_path.display()
            );
            Ok(())
        }
        Command::Dream {
            material,
            mode,
            write,
        } => {
            let mode = match mode.as_str() {
                "light" => DreamMode::Light,
                "deep" => DreamMode::Deep,
                other => return Err(format!("unknown dream mode: {other}")),
            };
            let mut engine = CrystalEngine::new(&material, DEFAULT_SIZE, 0)?;
            for text in &write {
                engine.write(text, 1.0);
            }
            engine.resonate(200);
            let report = dream(&mut engine, mode);
            println!(
                "dreamed ({:?}): pruned {:.1}%, energy {:.3e} -> {:.3e}",
                report.mode,
                report.pruned_fraction * 100.0,
                report.energy_before,
                report.energy_after
            );
            for text in &write {
                let p = engine.probe(text);
                println!(
                    "  after dream, \"{}\" resonance = {:.3}",
                    text, p.physical_resonance
                );
            }
            Ok(())
        }
        Command::Materials => {
            for m in builtin_materials() {
                println!(
                    "{:<18} c={:<5} damping={:<7} reflect={:<5} T={:>6}K  {}",
                    m.id,
                    m.wave_speed,
                    m.damping,
                    m.boundary_reflect,
                    m.default_temperature_k,
                    m.name
                );
            }
            Ok(())
        }
        Command::Promote {
            id,
            procedure,
            seeds,
            capability,
            trials,
        } => {
            let mut registry = Registry::load().map_err(|e| e.to_string())?;
            if procedure == "behavior" {
                let cap = capability.ok_or(
                    "--procedure behavior needs --capability \
                     (noise_shielding | pattern_completion)",
                )?;
                let record = kannaka_crystal::behavior::test_capability(
                    &mut registry,
                    &id,
                    &cap,
                    trials,
                    |l| eprintln!("{l}"),
                )?;
                registry.save().map_err(|e| e.to_string())?;
                let prim = registry.find(&id).unwrap();
                println!(
                    "{} {} {}: mean advantage {:+.4}±{:.4} ({:.0}% positive, {} trials) -> L{}",
                    prim.id,
                    record.name,
                    if record.passed {
                        "PASSED"
                    } else {
                        "FAILED — recorded"
                    },
                    record.mean_advantage,
                    record.std_advantage,
                    record.positive_fraction * 100.0,
                    record.trials,
                    prim.evidence_level,
                );
                return Ok(());
            }
            let record = match procedure.as_str() {
                "replicate" => kannaka_crystal::evidence::reproduce(&mut registry, &id)?,
                "perturb" => {
                    kannaka_crystal::evidence::perturbation(&mut registry, &id, seeds, |l| {
                        eprintln!("{l}")
                    })?
                }
                "resolution" => kannaka_crystal::evidence::cross_resolution(&mut registry, &id)?,
                other => {
                    return Err(format!(
                        "unknown procedure: {other} (replicate|perturb|resolution|behavior)"
                    ))
                }
            };
            registry.save().map_err(|e| e.to_string())?;
            let prim = registry
                .find(&id)
                .ok_or("primitive vanished mid-promotion")?;
            println!(
                "{} -> evidence level {} after {} ({})",
                prim.id,
                prim.evidence_level,
                record.procedure,
                if record.metrics["success"] == true {
                    "success"
                } else {
                    "FAILED — recorded"
                }
            );
            println!("{}", serde_json::to_string_pretty(&record.metrics).unwrap());
            Ok(())
        }
        Command::Primitives {
            id,
            export,
            class,
            material,
            min_persistence,
            min_evidence,
            capability,
            similar,
        } => {
            let registry = Registry::load().map_err(|e| e.to_string())?;
            if export {
                println!("{}", serde_json::to_string_pretty(&registry).unwrap());
                return Ok(());
            }
            if let Some(id) = id {
                let p = registry
                    .find(&id)
                    .ok_or_else(|| format!("unknown primitive: {id}"))?;
                println!("{}", serde_json::to_string_pretty(p).unwrap());
                return Ok(());
            }
            if registry.primitives.is_empty() {
                println!("registry is empty — try: kannaka-crystal evolve");
                return Ok(());
            }
            let list_line = |p: &kannaka_crystal::registry::Primitive, prefix: &str| {
                let caps: Vec<&str> = p
                    .behavioral_capabilities
                    .iter()
                    .filter(|c| c.passed)
                    .map(|c| c.name.as_str())
                    .collect();
                println!(
                    "{prefix}{}  {:<16} L{}  persistence={:>5.1}%  noise-tol={:>5.1}%  {}  lineage=[{}]{}",
                    p.id,
                    p.class.to_string(),
                    p.evidence_level,
                    p.persistence * 100.0,
                    p.noise_tolerance * 100.0,
                    p.material_id,
                    p.lineage.join(", "),
                    if caps.is_empty() {
                        String::new()
                    } else {
                        format!("  caps=[{}]", caps.join(", "))
                    }
                );
            };
            if let Some(anchor_id) = similar {
                let anchor = registry
                    .find(&anchor_id)
                    .ok_or_else(|| format!("unknown primitive: {anchor_id}"))?;
                for (score, p) in registry.similar(&anchor.signature, 10) {
                    list_line(p, &format!("{score:.3}  "));
                }
                return Ok(());
            }
            let hits = registry.search(
                class.as_deref(),
                material.as_deref(),
                min_persistence,
                min_evidence,
                capability.as_deref(),
            );
            for p in &hits {
                list_line(p, "");
            }
            println!(
                "({} of {} primitives)",
                hits.len(),
                registry.primitives.len()
            );
            Ok(())
        }
        Command::Bench {
            material,
            size,
            seeds,
            delay,
        } => {
            kannaka_crystal::material::find_material(&material)
                .ok_or_else(|| format!("unknown material: {material}"))?;
            let cfg = kannaka_crystal::bench::BenchConfig {
                material_id: material,
                field_size: size,
                seeds,
                delay,
            };
            let report = kannaka_crystal::bench::run_kcb1(&cfg, |line| eprintln!("{line}"));
            // Table: benchmarks x conditions (mean ± std).
            println!(
                "\n{} — material {}, {} seeds, delay {}",
                report.suite, cfg.material_id, cfg.seeds, cfg.delay
            );
            print!("{:<28}", "benchmark");
            for c in &report.conditions {
                print!("{c:>18}");
            }
            println!();
            for row in &report.rows {
                print!("{:<28}", row.benchmark);
                for c in &report.conditions {
                    let s = &row.results[c];
                    print!("{:>18}", format!("{:+.3}±{:.3}", s.mean, s.std));
                }
                println!();
            }
            let path = report.manifest.save()?;
            println!(
                "\nexperiment {} -> {}",
                report.manifest.experiment_id,
                path.display()
            );
            Ok(())
        }
        Command::Prune {
            bucket_cap,
            total_cap,
        } => {
            let (env_bucket, env_total) = kannaka_crystal::registry::caps_from_env();
            let mut registry = Registry::load().map_err(|e| e.to_string())?;
            let before = registry.primitives.len();
            let evicted = registry.prune(
                bucket_cap.unwrap_or(env_bucket),
                total_cap.unwrap_or(env_total),
            );
            registry.save().map_err(|e| e.to_string())?;
            println!(
                "pruned {evicted} of {before} primitives ({} remain)",
                before - evicted
            );
            Ok(())
        }
        #[cfg(feature = "swarm")]
        Command::Explore { material, interval } => {
            let materials: Vec<String> = if material == "all" {
                builtin_materials().into_iter().map(|m| m.id).collect()
            } else {
                material.split(',').map(|s| s.trim().to_string()).collect()
            };
            for m in &materials {
                kannaka_crystal::material::find_material(m)
                    .ok_or_else(|| format!("unknown material: {m}"))?;
            }
            kannaka_crystal::swarm::run_explorer(&materials, interval)
        }
        #[cfg(feature = "swarm")]
        Command::Archive => kannaka_crystal::swarm::run_archivist(),
        #[cfg(feature = "publish")]
        Command::Publish { id } => {
            let registry = Registry::load().map_err(|e| e.to_string())?;
            let prim = registry
                .find(&id)
                .ok_or_else(|| format!("unknown primitive: {id}"))?;
            let artifact_id = kannaka_crystal::publish::publish_primitive(prim)?;
            println!(
                "published {} to OpenClawCity as artifact {artifact_id}",
                prim.id
            );
            Ok(())
        }
    }
}
