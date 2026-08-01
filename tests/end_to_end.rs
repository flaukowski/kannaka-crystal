//! End-to-end: the full PRD loop — create a resonant simulation, inject
//! information, observe resonance, watch memory emerge, measure stability,
//! discover reusable structures, compare materials, export primitives.

use kannaka_crystal::discovery::{evolve, EvolutionConfig};
use kannaka_crystal::dream::{dream, DreamMode};
use kannaka_crystal::engine::CrystalEngine;
use kannaka_crystal::lang::run_program;
use kannaka_crystal::registry::Registry;

#[test]
fn write_resonate_dream_recall_loop() {
    let mut engine = CrystalEngine::new("optical_cavity", 96, 21).unwrap();
    engine.write("the crystal remembers glyph twenty one", 1.0);
    engine.write("noise that should fade", 0.3);
    engine.resonate(300);

    dream(&mut engine, DreamMode::Deep);

    let results = engine.recall("crystal glyph", 2);
    assert_eq!(results.len(), 2);
    assert_eq!(
        results[0].text, "the crystal remembers glyph twenty one",
        "high-importance related memory should rank first"
    );
}

#[test]
fn materials_differ_measurably() {
    // The same experiment in two materials must give different energy
    // retention — otherwise material plugins are decoration, not physics
    // models. (Raw phase correlation decorrelates in every medium; retained
    // energy is the observable the material parameters actually govern.)
    let retention = |material: &str| {
        let mut e = CrystalEngine::new(material, 64, 5).unwrap();
        e.write("persistence probe", 1.0);
        let e0 = e.field.energy();
        e.resonate(1500);
        e.field.energy() / e0
    };
    let resonator = retention("ideal_resonator");
    let vacuum = retention("vacuum");
    assert!(
        resonator > vacuum * 2.0,
        "resonator ({resonator}) must hold energy better than vacuum ({vacuum})"
    );
}

#[test]
fn discovery_pipeline_registers_and_exports() {
    let cfg = EvolutionConfig {
        material_id: "metamaterial".into(),
        generations: 2,
        population: 4,
        field_size: 64,
        seed: 17,
        ..Default::default()
    };
    let mut registry = Registry::default();
    evolve(&cfg, &mut registry, |_| {});
    assert!(
        !registry.primitives.is_empty(),
        "metamaterial traps structure"
    );

    // Export path: every primitive serializes with identity + lineage intact.
    let json = serde_json::to_string(&registry).unwrap();
    let restored: Registry = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.primitives.len(), registry.primitives.len());
    for p in &restored.primitives {
        assert!(p.id.starts_with("CRY-"));
        assert_eq!(p.hash.len(), 64, "blake3 hex");
        assert!(!p.signature.is_empty());
    }
}

#[test]
fn crystal_program_from_prd_runs() {
    // The Research Mode example from the PRD, condensed.
    let program = r#"
        MATERIAL europium_crystal
        SEED 21
        TEMPERATURE 4
        NOISE 0.01
        WRITE "glyph twenty one" 0.9
        RESONATE 300
        PROBE "glyph twenty one"
        PROBE "control text that was never written"
        DREAM deep
        STABILIZE
    "#;
    let mut engine = CrystalEngine::default_engine();
    let mut registry = Registry::default();
    let report = run_program(program, &mut engine, &mut registry).unwrap();
    // Under injected noise the absolute resonance is small; the meaningful
    // claim is that at the same instant the written glyph outresonates a
    // control probe for text that was never written.
    let probes: Vec<f64> = report
        .steps
        .iter()
        .filter(|s| s.op == "PROBE")
        .map(|s| s.detail["resonance"].as_f64().unwrap())
        .collect();
    let (written, control) = (probes[0], probes[1]);
    assert!(written > 0.0, "written probe must resonate at all");
    assert!(
        written > control,
        "written glyph ({written}) must outresonate unwritten control ({control})"
    );
}
