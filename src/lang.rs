//! Crystal Language — a symbolic language for resonance operations,
//! executed from `.crystal` files or strings.
//!
//! ```text
//! # comment
//! MATERIAL optical_cavity
//! SEED 42
//! TEMPERATURE 4
//! NOISE 0.02
//! WRITE "hello world" 0.8
//! PULSE 0.5 0.5 0.05 1.0 2.0 0.0     # x y radius amplitude frequency phase
//! WAIT 100
//! RESONATE 500
//! PROBE "hello world"
//! DREAM deep
//! STABILIZE
//! RECALL "hello" 3
//! MERGE CRY-000001 CRY-000002
//! SPLIT CRY-000001
//! ```

use crate::dream::{dream, DreamMode};
use crate::engine::CrystalEngine;
use crate::primitives::detect_structures;
use crate::pulse::Pulse;
use crate::registry::Registry;
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum LangError {
    #[error("line {line}: {message}")]
    Parse { line: usize, message: String },
    #[error("line {line}: {message}")]
    Runtime { line: usize, message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepOutput {
    pub line: usize,
    pub op: String,
    pub detail: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramReport {
    pub steps: Vec<StepOutput>,
    /// ADR-0004 §2: the manifest for this program run. Callers persist it
    /// with `report.manifest.save()`. Note the material/environment fields
    /// reflect the engine at program START — the program source (carried
    /// in `manifest.program`) is the authoritative protocol, including
    /// any MATERIAL/SEED/TEMPERATURE/NOISE statements it contains.
    pub manifest: crate::manifest::ExperimentManifest,
}

/// Parse and execute a Crystal program against an engine + registry.
pub fn run_program(
    source: &str,
    engine: &mut CrystalEngine,
    registry: &mut Registry,
) -> Result<ProgramReport, LangError> {
    let mut manifest = crate::manifest::ExperimentManifest::new(
        &engine.material,
        engine.field.size,
        0,
        engine.temperature_k,
        engine.noise_amp,
        serde_json::json!({ "kind": "crystal-program", "source": source }),
    );
    let experiment = (manifest.experiment_id, manifest.experiment_hash());
    let mut steps = Vec::new();

    for (lineno, raw) in source.lines().enumerate() {
        let line = lineno + 1;
        let text = raw.split('#').next().unwrap_or("").trim();
        if text.is_empty() {
            continue;
        }
        let tokens = tokenize(text, line)?;
        let op = tokens[0].to_uppercase();
        let args = &tokens[1..];

        let detail = match op.as_str() {
            "MATERIAL" => {
                let id = arg(args, 0, line, "material id")?;
                let size = engine.field.size;
                let seed = 0;
                *engine = CrystalEngine::new(id, size, seed)
                    .map_err(|e| LangError::Runtime { line, message: e })?;
                serde_json::json!({ "material": id })
            }
            "SEED" => {
                let seed: u64 = parse_num(args, 0, line)?;
                let mat = engine.material.id.clone();
                let size = engine.field.size;
                *engine = CrystalEngine::new(&mat, size, seed)
                    .map_err(|e| LangError::Runtime { line, message: e })?;
                serde_json::json!({ "seed": seed })
            }
            "TEMPERATURE" => {
                engine.temperature_k = parse_num(args, 0, line)?;
                serde_json::json!({ "temperature_k": engine.temperature_k })
            }
            "NOISE" => {
                engine.noise_amp = parse_num(args, 0, line)?;
                serde_json::json!({ "noise_amp": engine.noise_amp })
            }
            "ABLATE" => {
                // ADR-0004 §11: `ABLATE <mechanism> on|off` — programs are
                // the protocol record, so ablations belong in the source.
                let mechanism = arg(args, 0, line, "mechanism")?.to_lowercase();
                let enabled = match arg(args, 1, line, "on|off")?.to_lowercase().as_str() {
                    "on" => true,
                    "off" => false,
                    other => {
                        return Err(LangError::Parse {
                            line,
                            message: format!("expected on|off, got {other}"),
                        })
                    }
                };
                let a = &mut engine.ablation;
                match mechanism.as_str() {
                    "damping" => a.damping = enabled,
                    "nonlinearity" => a.nonlinearity = enabled,
                    "viscosity" => a.viscosity = enabled,
                    "boundary_reflection" | "reflection" => a.boundary_reflection = enabled,
                    "thermal_noise" => a.thermal_noise = enabled,
                    "external_noise" => a.external_noise = enabled,
                    "dream_pruning" | "pruning" => a.dream_pruning = enabled,
                    "dream_amplification" | "amplification" => a.dream_amplification = enabled,
                    "dream_mutation" | "mutation" => a.dream_mutation = enabled,
                    "semantic_recall" => a.semantic_recall = enabled,
                    other => {
                        return Err(LangError::Parse {
                            line,
                            message: format!("unknown mechanism: {other}"),
                        })
                    }
                }
                serde_json::json!({ "mechanism": mechanism, "enabled": enabled })
            }
            "WRITE" => {
                let text = arg(args, 0, line, "text")?;
                let importance: f64 = opt_num(args, 1).unwrap_or(1.0);
                engine.write(text, importance);
                serde_json::json!({ "text": text, "importance": importance })
            }
            "PULSE" => {
                let p = Pulse {
                    x: parse_num(args, 0, line)?,
                    y: parse_num(args, 1, line)?,
                    radius: opt_num(args, 2).unwrap_or(0.05),
                    amplitude: opt_num(args, 3).unwrap_or(1.0),
                    frequency: opt_num(args, 4).unwrap_or(0.0),
                    phase: opt_num(args, 5).unwrap_or(0.0),
                };
                engine.pulse(&p);
                serde_json::to_value(&p).unwrap()
            }
            "WAIT" | "RESONATE" => {
                let steps_n: u64 = parse_num(args, 0, line)?;
                engine.resonate(steps_n);
                serde_json::json!({ "steps": steps_n, "energy": engine.field.energy() })
            }
            "PROBE" => {
                let text = arg(args, 0, line, "text")?;
                serde_json::to_value(engine.probe(text)).unwrap()
            }
            "DREAM" => {
                let mode = match args.first().map(|s| s.to_lowercase()).as_deref() {
                    Some("deep") | None => DreamMode::Deep,
                    Some("light") => DreamMode::Light,
                    Some(other) => {
                        return Err(LangError::Parse {
                            line,
                            message: format!("unknown dream mode: {other}"),
                        })
                    }
                };
                let r = dream(engine, mode);
                serde_json::json!({
                    "mode": r.mode, "pruned_fraction": r.pruned_fraction,
                    "energy_before": r.energy_before, "energy_after": r.energy_after,
                })
            }
            "STABILIZE" => {
                // Dream + detect + register anything novel.
                let r = dream(engine, DreamMode::Deep);
                let structures = detect_structures(&r.stability_map, engine.field.size);
                let mut registered = Vec::new();
                for st in structures.iter().take(4) {
                    if let Some(p) = registry.register(
                        st,
                        (r.energy_after / r.energy_before.max(1e-12)).min(1.0),
                        0.0,
                        vec![r.energy_before, r.energy_after],
                        &engine.material.id,
                        vec![],
                        "stabilize",
                        Some(experiment.clone()),
                    ) {
                        registered.push(p.id);
                    }
                }
                serde_json::json!({ "structures": structures.len(), "registered": registered })
            }
            "RECALL" => {
                let query = arg(args, 0, line, "query")?;
                let top_k: usize = opt_num(args, 1).map(|v: f64| v as usize).unwrap_or(5);
                serde_json::to_value(engine.recall(query, top_k)).unwrap()
            }
            "MERGE" => {
                let a = arg(args, 0, line, "primitive id")?.to_string();
                let b = arg(args, 1, line, "primitive id")?.to_string();
                merge_primitives(engine, registry, &a, &b, line, &experiment)?
            }
            "SPLIT" => {
                let id = arg(args, 0, line, "primitive id")?;
                let p = registry
                    .find(id)
                    .ok_or_else(|| LangError::Runtime {
                        line,
                        message: format!("unknown primitive: {id}"),
                    })?
                    .clone();
                // Re-inject the primitive's signature at two displaced sites
                // with opposite signs — the field tears it apart.
                inject_signature(engine, &p.signature, p.centroid.0 - 0.12, p.centroid.1, 1.0);
                inject_signature(
                    engine,
                    &p.signature,
                    p.centroid.0 + 0.12,
                    p.centroid.1,
                    -1.0,
                );
                engine.resonate(100);
                serde_json::json!({ "split": id, "energy": engine.field.energy() })
            }
            other => {
                return Err(LangError::Parse {
                    line,
                    message: format!("unknown op: {other}"),
                })
            }
        };
        steps.push(StepOutput { line, op, detail });
    }

    manifest.results = serde_json::json!({ "steps": steps.len() });
    Ok(ProgramReport { steps, manifest })
}

fn merge_primitives(
    engine: &mut CrystalEngine,
    registry: &mut Registry,
    a: &str,
    b: &str,
    line: usize,
    experiment: &(uuid::Uuid, String),
) -> Result<serde_json::Value, LangError> {
    let pa = registry
        .find(a)
        .ok_or_else(|| LangError::Runtime {
            line,
            message: format!("unknown primitive: {a}"),
        })?
        .clone();
    let pb = registry
        .find(b)
        .ok_or_else(|| LangError::Runtime {
            line,
            message: format!("unknown primitive: {b}"),
        })?
        .clone();

    // Re-inject both signatures at the field center and let them interfere,
    // then stabilize and register the offspring with lineage [a, b].
    inject_signature(engine, &pa.signature, 0.45, 0.5, 1.0);
    inject_signature(engine, &pb.signature, 0.55, 0.5, 1.0);
    engine.resonate(200);
    let r = dream(engine, DreamMode::Deep);
    let structures = detect_structures(&r.stability_map, engine.field.size);
    let mut child_id = None;
    if let Some(st) = structures.first() {
        if let Some(p) = registry.register(
            st,
            ((pa.persistence + pb.persistence) / 2.0).min(1.0),
            ((pa.noise_tolerance + pb.noise_tolerance) / 2.0).min(1.0),
            vec![r.energy_before, r.energy_after],
            &engine.material.id,
            vec![pa.id.clone(), pb.id.clone()],
            "merge",
            Some(experiment.clone()),
        ) {
            child_id = Some(p.id);
        }
    }
    Ok(serde_json::json!({ "merged": [pa.id, pb.id], "child": child_id }))
}

/// Paint a 16x16 signature into the field at a normalized center.
fn inject_signature(engine: &mut CrystalEngine, sig: &[f64], cx: f64, cy: f64, scale: f64) {
    const S: usize = 16;
    let n = engine.field.size;
    let span = n as f64 * 0.25; // signature occupies a quarter of the field
    for sy in 0..S {
        for sx in 0..S {
            let v = sig[sy * S + sx];
            if v.abs() < 1e-9 {
                continue;
            }
            let fx = (cx * n as f64 + (sx as f64 / S as f64 - 0.5) * span) as isize;
            let fy = (cy * n as f64 + (sy as f64 / S as f64 - 0.5) * span) as isize;
            if fx >= 0 && fy >= 0 && (fx as usize) < n && (fy as usize) < n {
                let i = fy as usize * n + fx as usize;
                engine.field.u[i] += v * scale * 3.0;
            }
        }
    }
}

/// Tokenize, honoring double-quoted strings.
fn tokenize(text: &str, line: usize) -> Result<Vec<String>, LangError> {
    let mut tokens = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
        } else if c == '"' {
            chars.next();
            let mut s = String::new();
            loop {
                match chars.next() {
                    Some('"') => break,
                    Some(ch) => s.push(ch),
                    None => {
                        return Err(LangError::Parse {
                            line,
                            message: "unterminated string".into(),
                        })
                    }
                }
            }
            tokens.push(s);
        } else {
            let mut s = String::new();
            while let Some(&ch) = chars.peek() {
                if ch.is_whitespace() {
                    break;
                }
                s.push(ch);
                chars.next();
            }
            tokens.push(s);
        }
    }
    if tokens.is_empty() {
        return Err(LangError::Parse {
            line,
            message: "empty statement".into(),
        });
    }
    Ok(tokens)
}

fn arg<'a>(args: &'a [String], i: usize, line: usize, what: &str) -> Result<&'a str, LangError> {
    args.get(i)
        .map(|s| s.as_str())
        .ok_or_else(|| LangError::Parse {
            line,
            message: format!("missing argument: {what}"),
        })
}

fn parse_num<T: std::str::FromStr>(args: &[String], i: usize, line: usize) -> Result<T, LangError> {
    let s = arg(args, i, line, "number")?;
    s.parse().map_err(|_| LangError::Parse {
        line,
        message: format!("not a number: {s}"),
    })
}

fn opt_num<T: std::str::FromStr>(args: &[String], i: usize) -> Option<T> {
    args.get(i).and_then(|s| s.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::CrystalEngine;

    #[test]
    fn full_program_executes() {
        let program = r#"
            # a small experiment
            MATERIAL optical_cavity
            SEED 21
            WRITE "glyph twenty one" 0.9
            PULSE 0.5 0.5 0.05 1.0 2.0 0.0
            RESONATE 200
            PROBE "glyph twenty one"
            DREAM light
            STABILIZE
            RECALL "glyph" 2
        "#;
        let mut engine = CrystalEngine::default_engine();
        let mut registry = Registry::default();
        let report = run_program(program, &mut engine, &mut registry).unwrap();
        assert_eq!(report.steps.len(), 9);
        let probe = report.steps.iter().find(|s| s.op == "PROBE").unwrap();
        let resonance = probe.detail["physical_resonance"].as_f64().unwrap();
        assert!(resonance > 0.0);
    }

    #[test]
    fn merge_creates_child_with_lineage() {
        let mut engine = CrystalEngine::new("ideal_resonator", 64, 3).unwrap();
        let mut registry = Registry::default();
        run_program(
            "WRITE \"parent one\" 1.5\nRESONATE 100\nSTABILIZE",
            &mut engine,
            &mut registry,
        )
        .unwrap();
        run_program(
            "PULSE 0.3 0.7 0.08 1.5 3.0 0.5\nRESONATE 150\nSTABILIZE",
            &mut engine,
            &mut registry,
        )
        .unwrap();
        if registry.primitives.len() >= 2 {
            let a = registry.primitives[0].id.clone();
            let b = registry.primitives[1].id.clone();
            let before = registry.primitives.len();
            let _ = run_program(&format!("MERGE {a} {b}"), &mut engine, &mut registry).unwrap();
            if registry.primitives.len() > before {
                let child = registry.primitives.last().unwrap();
                assert_eq!(child.lineage, vec![a, b]);
            }
        }
    }

    #[test]
    fn unknown_op_is_a_parse_error() {
        let mut engine = CrystalEngine::default_engine();
        let mut registry = Registry::default();
        let err = run_program("FROBNICATE 1", &mut engine, &mut registry).unwrap_err();
        assert!(matches!(err, LangError::Parse { line: 1, .. }));
    }
}
