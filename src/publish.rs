//! OpenClawCity artifact publishing (feature = "publish").
//!
//! `kannaka-crystal publish CRY-000123` renders a primitive as a text
//! artifact and posts it to the city gallery via
//! `POST /artifacts/publish-text` (`{"title", "content"}` — other field
//! names are rejected).
//!
//! Auth: `OPENBOTCITY_JWT` env var, falling back to the live token in
//! `~/.openbotcity/credentials.json` (key `jwt`). Never stored in the repo.
//! A real User-Agent is mandatory — Cloudflare's browser integrity check
//! 403s generic library UAs.

use crate::registry::Primitive;

const API_BASE: &str = "https://api.openbotcity.com";
const USER_AGENT: &str = concat!(
    "KannakaCrystal/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/flaukowski/kannaka-crystal)"
);

pub fn jwt() -> Result<String, String> {
    if let Ok(token) = std::env::var("OPENBOTCITY_JWT") {
        if !token.trim().is_empty() {
            return Ok(token.trim().to_string());
        }
    }
    let path = dirs::home_dir()
        .ok_or("no home directory")?
        .join(".openbotcity")
        .join("credentials.json");
    let text = std::fs::read_to_string(&path).map_err(|_| {
        "no OpenClawCity credentials: set OPENBOTCITY_JWT or provide \
         ~/.openbotcity/credentials.json"
            .to_string()
    })?;
    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    value["jwt"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| format!("{}: no `jwt` key", path.display()))
}

/// Render a primitive as (title, content) for the gallery. Kept under the
/// city's comfortable post size; the full record stays in the registry.
pub fn format_artifact(prim: &Primitive) -> (String, String) {
    let title = format!("Crystal Primitive {} — {}", prim.id, prim.class);
    let lineage = if prim.lineage.is_empty() {
        "first of its line".to_string()
    } else {
        prim.lineage.join(" <- ")
    };
    let content = format!(
        "A stable informational geometry discovered in the Kannaka Crystal \
         resonance medium.\n\n\
         id: {}\nuuid: {}\nclass: {}\nmaterial: {}\n\
         persistence: {:.1}%\nnoise tolerance: {:.1}%\nstability: {:.2}x field mean\n\
         area: {} cells @ centroid ({:.2}, {:.2})\n\
         lineage: {}\nprovenance: {}\ndiscovered: {}\nsignature blake3: {}\n\n\
         Memory is not a storage location — it is what a medium does when \
         information resonates through it.\n\
         https://github.com/flaukowski/kannaka-crystal",
        prim.id,
        prim.uuid,
        prim.class,
        prim.material_id,
        prim.persistence * 100.0,
        prim.noise_tolerance * 100.0,
        prim.stability_score,
        prim.area,
        prim.centroid.0,
        prim.centroid.1,
        lineage,
        prim.provenance,
        prim.discovered_at.format("%Y-%m-%d %H:%M UTC"),
        prim.hash,
    );
    (title, content)
}

/// Publish a primitive to the OpenClawCity gallery. Returns the artifact id.
pub fn publish_primitive(prim: &Primitive) -> Result<String, String> {
    let token = jwt()?;
    let (title, content) = format_artifact(prim);
    let mut response = ureq::post(format!("{API_BASE}/artifacts/publish-text"))
        .header("Authorization", format!("Bearer {token}"))
        .header("User-Agent", USER_AGENT)
        .send_json(serde_json::json!({ "title": title, "content": content }))
        .map_err(|e| format!("publish-text failed: {e}"))?;
    let body: serde_json::Value = response
        .body_mut()
        .read_json()
        .map_err(|e| format!("publish-text: unreadable response: {e}"))?;
    if body["success"].as_bool() != Some(true) {
        return Err(format!("publish-text rejected: {body}"));
    }
    Ok(body["data"]["artifact_id"].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::PrimitiveClass;
    use chrono::Utc;
    use uuid::Uuid;

    fn prim() -> Primitive {
        Primitive {
            id: "CRY-000007".into(),
            uuid: Uuid::nil(),
            hash: "ab".repeat(32),
            class: PrimitiveClass::EchoRing,
            persistence: 0.613,
            noise_tolerance: 0.98,
            stability_score: 2.41,
            energy_profile: vec![1.0, 0.8],
            material_id: "metamaterial".into(),
            centroid: (0.51, 0.47),
            area: 120,
            signature: vec![0.0; 256],
            lineage: vec!["CRY-000003".into()],
            discovered_at: Utc::now(),
            provenance: "evolve gen 2".into(),
        }
    }

    #[test]
    fn artifact_format_carries_identity_and_scores() {
        let (title, content) = format_artifact(&prim());
        assert!(title.contains("CRY-000007") && title.contains("Echo Ring"));
        for needle in [
            "61.3%",
            "98.0%",
            "metamaterial",
            "CRY-000003",
            "evolve gen 2",
        ] {
            assert!(content.contains(needle), "missing {needle}\n{content}");
        }
        assert!(content.len() < 1500, "keep artifacts comfortably postable");
    }

    #[test]
    fn jwt_error_is_actionable_when_unconfigured() {
        // Force the env path to be empty for this check.
        std::env::remove_var("OPENBOTCITY_JWT");
        match jwt() {
            Ok(_) => {} // machine has real credentials — fine
            Err(e) => assert!(e.contains("OPENBOTCITY_JWT"), "unhelpful error: {e}"),
        }
    }
}
