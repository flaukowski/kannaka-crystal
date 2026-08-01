//! REST API + embedded Observatory.
//!
//! Endpoints (PRD "API"):
//!   POST /crystal/write   {"text": "...", "importance": 0.8}
//!   POST /crystal/pulse   {"x":0.5,"y":0.5,"radius":0.05,"amplitude":1,"frequency":0,"phase":0}
//!   POST /crystal/step    {"steps": 100}
//!   POST /crystal/dream   {"mode": "deep"|"light"}
//!   POST /crystal/probe   {"text": "..."}
//!   POST /crystal/recall  {"query": "...", "top_k": 5}
//!   POST /crystal/evolve  {"generations": 5, "population": 8, ...}
//!   POST /crystal/run     {"program": "WRITE \"x\"\nRESONATE 100"}
//!   GET  /primitives            GET /primitives/{id}
//!   GET  /materials
//!   GET  /crystal/state    — downsampled field + energy timeline (Observatory feed)
//!   GET  /                 — the Observatory single-page UI
//!
//! Single engine behind a mutex: this is a research instrument, not a
//! high-concurrency service. Evolution runs synchronously with a bounded
//! generation cap so a request can't wedge the server for hours.

use crate::discovery::{evolve, EvolutionConfig};
use crate::dream::{dream, DreamMode};
use crate::engine::CrystalEngine;
use crate::lang::run_program;
use crate::material::builtin_materials;
use crate::pulse::Pulse;
use crate::registry::Registry;
use serde::Deserialize;
use serde_json::json;
use std::sync::Mutex;
use tiny_http::{Header, Method, Response, Server};

const OBSERVATORY_HTML: &str = include_str!("../static/observatory.html");
/// Hard cap on synchronous evolution work per request.
const MAX_GENERATIONS_PER_REQUEST: usize = 50;

struct AppState {
    engine: Mutex<CrystalEngine>,
    registry: Mutex<Registry>,
}

pub fn serve(bind: &str, material_id: &str, field_size: usize, seed: u64) -> Result<(), String> {
    let engine = CrystalEngine::new(material_id, field_size, seed)?;
    let registry = Registry::load().map_err(|e| e.to_string())?;
    let state = AppState {
        engine: Mutex::new(engine),
        registry: Mutex::new(registry),
    };

    let server = Server::http(bind).map_err(|e| format!("bind {bind}: {e}"))?;
    println!("kannaka-crystal observatory: http://{bind}/");
    println!("REST API base:               http://{bind}/crystal/*");

    for mut request in server.incoming_requests() {
        let method = request.method().clone();
        let url = request.url().to_string();
        let mut body = String::new();
        let _ = request.as_reader().read_to_string(&mut body);

        let (status, payload) = route(&state, &method, &url, &body);
        let content_type = if payload.starts_with("<!") || payload.starts_with("<html") {
            "text/html; charset=utf-8"
        } else {
            "application/json"
        };
        let response = Response::from_string(payload)
            .with_status_code(status)
            .with_header(Header::from_bytes("Content-Type", content_type).unwrap())
            .with_header(Header::from_bytes("Access-Control-Allow-Origin", "*").unwrap());
        let _ = request.respond(response);
    }
    Ok(())
}

fn route(state: &AppState, method: &Method, url: &str, body: &str) -> (u16, String) {
    let path = url.split('?').next().unwrap_or(url);
    match (method, path) {
        (Method::Get, "/") | (Method::Get, "/observatory") => (200, OBSERVATORY_HTML.to_string()),
        (Method::Get, "/crystal/state") => get_state(state),
        (Method::Get, "/materials") => (200, serde_json::to_string(&builtin_materials()).unwrap()),
        (Method::Get, "/primitives") => {
            let reg = state.registry.lock().unwrap();
            (200, serde_json::to_string(&reg.primitives).unwrap())
        }
        (Method::Get, p) if p.starts_with("/primitives/") => {
            let id = p.trim_start_matches("/primitives/");
            let reg = state.registry.lock().unwrap();
            match reg.find(id) {
                Some(prim) => (200, serde_json::to_string(prim).unwrap()),
                None => (404, error_json(&format!("unknown primitive: {id}"))),
            }
        }
        (Method::Post, "/crystal/write") => post_write(state, body),
        (Method::Post, "/crystal/pulse") => post_pulse(state, body),
        (Method::Post, "/crystal/step") => post_step(state, body),
        (Method::Post, "/crystal/dream") => post_dream(state, body),
        (Method::Post, "/crystal/probe") => post_probe(state, body),
        (Method::Post, "/crystal/recall") => post_recall(state, body),
        (Method::Post, "/crystal/evolve") => post_evolve(state, body),
        (Method::Post, "/crystal/run") => post_run(state, body),
        _ => (404, error_json("no such route")),
    }
}

fn error_json(msg: &str) -> String {
    json!({ "error": msg }).to_string()
}

fn parse<T: for<'de> Deserialize<'de>>(body: &str) -> Result<T, (u16, String)> {
    serde_json::from_str(body).map_err(|e| (400, error_json(&format!("bad request: {e}"))))
}

fn get_state(state: &AppState) -> (u16, String) {
    let engine = state.engine.lock().unwrap();
    let reg = state.registry.lock().unwrap();
    let payload = json!({
        "material": engine.material,
        "temperature_k": engine.temperature_k,
        "noise_amp": engine.noise_amp,
        "step": engine.field.step_count,
        "energy": engine.field.energy(),
        "field": engine.field.downsample_abs(64),
        "field_size": 64,
        "energy_timeline": engine.energy_timeline,
        "writes": engine.writes,
        "primitive_count": reg.primitives.len(),
    });
    (200, payload.to_string())
}

#[derive(Deserialize)]
struct WriteReq {
    text: String,
    #[serde(default = "default_importance")]
    importance: f64,
}
fn default_importance() -> f64 {
    1.0
}

fn post_write(state: &AppState, body: &str) -> (u16, String) {
    let req: WriteReq = match parse(body) {
        Ok(r) => r,
        Err(e) => return e,
    };
    let mut engine = state.engine.lock().unwrap();
    engine.write(&req.text, req.importance);
    (
        200,
        json!({ "written": req.text, "step": engine.field.step_count }).to_string(),
    )
}

fn post_pulse(state: &AppState, body: &str) -> (u16, String) {
    let pulse: Pulse = match parse(body) {
        Ok(p) => p,
        Err(e) => return e,
    };
    let mut engine = state.engine.lock().unwrap();
    engine.pulse(&pulse);
    (
        200,
        json!({ "pulsed": pulse, "energy": engine.field.energy() }).to_string(),
    )
}

#[derive(Deserialize)]
struct StepReq {
    steps: u64,
}

fn post_step(state: &AppState, body: &str) -> (u16, String) {
    let req: StepReq = match parse(body) {
        Ok(r) => r,
        Err(e) => return e,
    };
    let steps = req.steps.min(100_000);
    let mut engine = state.engine.lock().unwrap();
    engine.resonate(steps);
    (
        200,
        json!({ "stepped": steps, "energy": engine.field.energy() }).to_string(),
    )
}

#[derive(Deserialize)]
struct DreamReq {
    #[serde(default)]
    mode: Option<String>,
}

fn post_dream(state: &AppState, body: &str) -> (u16, String) {
    let req: DreamReq = if body.trim().is_empty() {
        DreamReq { mode: None }
    } else {
        match parse(body) {
            Ok(r) => r,
            Err(e) => return e,
        }
    };
    let mode = match req.mode.as_deref() {
        Some("light") => DreamMode::Light,
        Some("deep") | None => DreamMode::Deep,
        Some(other) => return (400, error_json(&format!("unknown dream mode: {other}"))),
    };
    let mut engine = state.engine.lock().unwrap();
    let report = dream(&mut engine, mode);
    (200, serde_json::to_string(&report).unwrap())
}

#[derive(Deserialize)]
struct ProbeReq {
    text: String,
}

fn post_probe(state: &AppState, body: &str) -> (u16, String) {
    let req: ProbeReq = match parse(body) {
        Ok(r) => r,
        Err(e) => return e,
    };
    let mut engine = state.engine.lock().unwrap();
    (
        200,
        serde_json::to_string(&engine.probe(&req.text)).unwrap(),
    )
}

#[derive(Deserialize)]
struct RecallReq {
    query: String,
    #[serde(default = "default_top_k")]
    top_k: usize,
}
fn default_top_k() -> usize {
    5
}

fn post_recall(state: &AppState, body: &str) -> (u16, String) {
    let req: RecallReq = match parse(body) {
        Ok(r) => r,
        Err(e) => return e,
    };
    let mut engine = state.engine.lock().unwrap();
    (
        200,
        serde_json::to_string(&engine.recall(&req.query, req.top_k.min(50))).unwrap(),
    )
}

#[derive(Deserialize)]
struct EvolveReq {
    #[serde(default)]
    material_id: Option<String>,
    #[serde(default)]
    generations: Option<usize>,
    #[serde(default)]
    population: Option<usize>,
    #[serde(default)]
    seed: Option<u64>,
}

fn post_evolve(state: &AppState, body: &str) -> (u16, String) {
    let req: EvolveReq = if body.trim().is_empty() {
        EvolveReq {
            material_id: None,
            generations: None,
            population: None,
            seed: None,
        }
    } else {
        match parse(body) {
            Ok(r) => r,
            Err(e) => return e,
        }
    };
    let engine_material = state.engine.lock().unwrap().material.id.clone();
    let cfg = EvolutionConfig {
        material_id: req.material_id.unwrap_or(engine_material),
        generations: req
            .generations
            .unwrap_or(5)
            .min(MAX_GENERATIONS_PER_REQUEST),
        population: req.population.unwrap_or(8).clamp(2, 32),
        seed: req.seed.unwrap_or(0),
        ..Default::default()
    };
    if crate::material::find_material(&cfg.material_id).is_none() {
        return (
            400,
            error_json(&format!("unknown material: {}", cfg.material_id)),
        );
    }
    let mut registry = state.registry.lock().unwrap();
    let report = evolve(&cfg, &mut registry, |_| {});
    if let Err(e) = registry.save() {
        return (500, error_json(&format!("registry save failed: {e}")));
    }
    (200, serde_json::to_string(&report).unwrap())
}

#[derive(Deserialize)]
struct RunReq {
    program: String,
}

fn post_run(state: &AppState, body: &str) -> (u16, String) {
    let req: RunReq = match parse(body) {
        Ok(r) => r,
        Err(e) => return e,
    };
    let mut engine = state.engine.lock().unwrap();
    let mut registry = state.registry.lock().unwrap();
    match run_program(&req.program, &mut engine, &mut registry) {
        Ok(report) => {
            if let Err(e) = registry.save() {
                return (500, error_json(&format!("registry save failed: {e}")));
            }
            (200, serde_json::to_string(&report).unwrap())
        }
        Err(e) => (400, error_json(&e.to_string())),
    }
}
