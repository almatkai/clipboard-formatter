//! The production adapter compiles unchanged against the real pinned common.rs.
//! Only Tauri AppHandle/Emitter are stubbed; the router records fake requests.
extern crate self as tauri;

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct AppHandle;
pub trait Emitter {
    fn emit(&self, channel: &str, payload: Value) -> Result<(), String>;
}
impl Emitter for AppHandle {
    fn emit(&self, _: &str, _: Value) -> Result<(), String> {
        panic!("No Tauri application or OS effects may run in contract tests")
    }
}
mod native_core {
    // Unmodified Apache-2.0 host fixture; the production library does not use it.
    #[allow(dead_code, clippy::all)]
    pub mod common {
        include!("fixtures/beta_common.rs");
    }
}
#[path = "../tezbar_adapter.rs"]
mod adapter;

#[test]
fn manifest_obeys_actual_beta_schema_and_preserves_source_metadata_and_icon() {
    let schema: Value =
        serde_json::from_str(include_str!("fixtures/extension-v2.schema.json")).unwrap();
    let manifest: Value = serde_json::from_str(adapter::MANIFEST).unwrap();
    jsonschema::validator_for(&schema)
        .unwrap()
        .validate(&manifest)
        .unwrap();
    let v1: Value = serde_json::from_str(include_str!("fixtures/v1-package.json")).unwrap();
    assert_eq!(manifest["schemaVersion"], 2);
    assert_eq!(manifest["id"], "com.tezbar.native.clipboard-formatter");
    assert_eq!(manifest["replaces"], json!(["raycast.clipboard-formatter"]));
    assert_eq!(manifest["slug"], v1["name"]);
    assert_eq!(manifest["name"], v1["title"]);
    assert_eq!(manifest["version"], v1["version"]);
    assert_eq!(manifest["platforms"], v1["platforms"]);
    assert_eq!(manifest["runtime"], "rust");
    assert_eq!(
        manifest["icon"],
        "/extensions/icons/clipboard-formatter.png"
    );
    assert!(manifest.get("author").is_none());
    assert!(manifest.get("preferences").is_none());
    for name in ["joshtemple", "almatkai"] {
        assert!(manifest["description"].as_str().unwrap().contains(name));
    }
    assert_eq!(adapter::commands(), &["index"]);
    assert_eq!(
        manifest["commands"].as_array().unwrap().len(),
        adapter::commands().len()
    );
    for key in ["name", "title", "description", "mode", "platforms"] {
        assert_eq!(manifest["commands"][0][key], v1["commands"][0][key]);
    }
    assert_eq!(manifest["commands"][0]["runtime"], "rust");
    let verification = manifest["verification"].as_array().unwrap();
    assert_eq!(verification.len(), 2);
    assert_eq!(
        verification
            .iter()
            .map(|v| v["platform"].clone())
            .collect::<Vec<_>>(),
        vec![json!("macOS"), json!("Windows")]
    );
    assert!(verification.iter().all(|v| v["state"] == "not-run"));
    assert_eq!(adapter::ICON.len(), 32743);
    assert_eq!(
        format!("{:x}", Sha256::digest(adapter::ICON)),
        "43a642a96244a10f19d2203a3a562a27b99694ff6c3914b18be39c47e267a83d"
    );
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    for (name, expected) in [
        ("assets/extension_icon.png", adapter::ICON),
        (
            "src/index.ts",
            include_bytes!("fixtures/v1-index.ts").as_slice(),
        ),
        (
            "package.json",
            include_bytes!("fixtures/v1-package.json").as_slice(),
        ),
    ] {
        let path = root.join(name);
        if path.is_file() {
            assert_eq!(std::fs::read(path).unwrap(), expected);
        }
    }
}

#[test]
fn fixtures_are_byte_exact_pinned_copies_with_offline_oracle_and_license() {
    let files: &[(&str, &[u8], &str)] = &[
        (
            "beta_common.rs",
            include_bytes!("fixtures/beta_common.rs"),
            "280a0a2f0dd04a8570b320aafc0c3ece0f5de8ddb21c363645567d6a35d84da7",
        ),
        (
            "extension-v2.schema.json",
            include_bytes!("fixtures/extension-v2.schema.json"),
            "6d5afda1b652605982104bdf16d1547547b98dfd707a7b2969316406c49f32af",
        ),
        (
            "TEZBAR-LICENSE.txt",
            include_bytes!("fixtures/TEZBAR-LICENSE.txt"),
            "0ed6b1b2760294d4aaaa45be38e166fcc0168e1dd5190423c096fcbe1c94afbf",
        ),
        (
            "v1-index.ts",
            include_bytes!("fixtures/v1-index.ts"),
            "b47270a92b58fefd0f213c1b87e9d076c70318ed1c85a7156ff1b11f6e2eef30",
        ),
        (
            "v1-package.json",
            include_bytes!("fixtures/v1-package.json"),
            "3ee87c457e8b6572d43b8e825d79cd460fafbf52b3b3ae834c2195cd6e47225c",
        ),
    ];
    let provenance: Value = serde_json::from_str(include_str!("fixtures/provenance.json")).unwrap();
    assert_eq!(
        provenance["tezbarCommit"],
        "fd300e276356587726566026f072563eb9c34cb1"
    );
    assert_eq!(
        provenance["v1Commit"],
        "f5436c9aa2d5413741affcf3581242138b322bdf"
    );
    for (name, bytes, hash) in files {
        assert_eq!(&format!("{:x}", Sha256::digest(bytes)), hash, "{name}");
        let record = provenance["files"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["file"] == *name)
            .unwrap();
        assert_eq!(record["sha256"], *hash);
        assert_eq!(record["bytes"], bytes.len());
    }
    let oracle: Value = serde_json::from_str(include_str!("fixtures/v1-vectors.json")).unwrap();
    assert_eq!(
        oracle["sourceSha256"],
        "b47270a92b58fefd0f213c1b87e9d076c70318ed1c85a7156ff1b11f6e2eef30"
    );
    assert_eq!(oracle["vectors"].as_array().unwrap().len(), 60);
    let license = include_str!("../LICENSE");
    assert!(license.contains("Copyright (c) 2021 Raycast\nCopyright (c) 2026 almatkai\n"));
    assert!(include_str!("../Cargo.toml").contains("license = \"MIT AND Apache-2.0\""));
}

type Reply = Result<Value, String>;
fn context() -> native_core::common::CoreContext {
    use native_core::common::{CoreContext, CorePaths};
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/adapter-fixture");
    CoreContext::headless(CorePaths {
        data_dir: root.join("data"),
        config_dir: root.join("config"),
        resource_dir: root.join("resources"),
        temp_dir: root.join("tmp"),
        home_dir: root,
    })
    .with_event_sink(Arc::new(|_, _| panic!("Unexpected emitted event")))
}
async fn concrete_run(replies: Vec<Reply>, expected_calls: Value, expected_effects: Value) {
    let ctx = context();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let recorded = calls.clone();
    let replies = Arc::new(Mutex::new(VecDeque::from(replies)));
    let queue = replies.clone();
    ctx.set_router(Arc::new(move |route, args| {
        let recorded = recorded.clone();
        let queue = queue.clone();
        Box::pin(async move {
            recorded.lock().unwrap().push(json!([route, args]));
            queue
                .lock()
                .unwrap()
                .pop_front()
                .expect("Unplanned host route")
        })
    }))
    .unwrap();
    let mut state = json!({});
    let result = adapter::handle(
        &ctx,
        "index",
        "extension:run-command",
        &json!({}),
        &mut state,
    )
    .await
    .unwrap();
    assert_eq!(result["mode"], "no-view");
    assert_eq!(result["effects"], expected_effects);
    assert_eq!(json!(*calls.lock().unwrap()), expected_calls);
    assert!(replies.lock().unwrap().is_empty());
    assert_eq!(state, json!({}));
    assert_eq!(
        adapter::handle(
            &ctx,
            "index",
            "extension:dispose-session",
            &Value::Null,
            &mut state
        )
        .await
        .unwrap(),
        json!({"ok":true,"mode":"unchanged"})
    );
    assert!(adapter::handle(
        &ctx,
        "bad",
        "extension:run-command",
        &Value::Null,
        &mut state
    )
    .await
    .is_err());
    assert!(adapter::handle(
        &ctx,
        "index",
        "extension:invoke-action",
        &Value::Null,
        &mut state
    )
    .await
    .is_err());
    assert_eq!(json!(*calls.lock().unwrap()), expected_calls);
    assert!(!ctx.paths.data_dir.exists());
    assert!(!ctx.paths.config_dir.exists());
}

#[tokio::test]
async fn unchanged_production_wrapper_uses_real_corecontext_and_exact_beta_routes() {
    let oracle: Value = serde_json::from_str(include_str!("fixtures/v1-vectors.json")).unwrap();
    for name in [
        "empty",
        "unchanged",
        "space-only",
        "tags-only",
        "quoted-greater-than",
        "unicode",
    ] {
        let v = oracle["vectors"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["name"] == name)
            .unwrap();
        let mut replies = vec![Ok(v["input"].clone())];
        if name != "empty" {
            replies.push(Ok(json!(true)));
        }
        concrete_run(replies, v["calls"].clone(), v["effects"].clone()).await;
    }
    for fail_at in ["read", "write"] {
        for failure in [
            Err("denied".into()),
            Ok(json!({"ok":false,"error":"denied"})),
            Ok(json!({"ok":false,"message":"denied"})),
        ] {
            let replies = if fail_at == "read" {
                vec![failure]
            } else {
                vec![Ok(json!("<b>x</b>")), failure]
            };
            let calls = if fail_at == "read" {
                json!([["clipboard:read", []]])
            } else {
                json!([["clipboard:read", []], ["clipboard:write", ["x"]]])
            };
            concrete_run(
                replies,
                calls,
                json!([{"kind":"toast","style":"failure","title":"Error","message":"denied"}]),
            )
            .await;
        }
    }
}

#[tokio::test]
async fn uninitialized_real_core_router_yields_source_error_toast_without_success() {
    let ctx = context();
    let result = adapter::handle(
        &ctx,
        "index",
        "extension:run-command",
        &json!({}),
        &mut json!({}),
    )
    .await
    .unwrap();
    assert_eq!(
        result,
        json!({"ok":true,"mode":"no-view","effects":[{"kind":"toast","style":"failure","title":"Error","message":"Native core dispatcher is not initialized"}]})
    );
}
