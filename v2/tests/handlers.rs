use serde_json::{json, Value};
use std::collections::VecDeque;
use std::sync::Mutex;
use tezbar_clipboard_formatter_v2::{handle, transform, Host, HostFuture};

type Reply = std::result::Result<Value, String>;
struct RecordingHost {
    replies: Mutex<VecDeque<Reply>>,
    calls: Mutex<Vec<Value>>,
}
impl RecordingHost {
    fn new(replies: Vec<Reply>) -> Self {
        Self {
            replies: Mutex::new(replies.into()),
            calls: Mutex::new(vec![]),
        }
    }
    fn calls(&self) -> Value {
        json!(*self.calls.lock().unwrap())
    }
    fn exhausted(&self) {
        assert!(self.replies.lock().unwrap().is_empty());
    }
}
impl Host for RecordingHost {
    fn request<'a>(&'a self, route: &'a str, args: Vec<Value>) -> HostFuture<'a> {
        Box::pin(async move {
            self.calls.lock().unwrap().push(json!([route, args]));
            self.replies
                .lock()
                .unwrap()
                .pop_front()
                .expect("Unexpected OS request")
        })
    }
}
fn oracle() -> Value {
    serde_json::from_str(include_str!("fixtures/v1-vectors.json")).unwrap()
}
async fn run(host: &RecordingHost) -> Value {
    let mut state = json!({"unrelated":"preserved"});
    let result = handle(
        host,
        "index",
        "extension:run-command",
        &json!({}),
        &mut state,
    )
    .await
    .unwrap();
    assert_eq!(state, json!({"unrelated":"preserved"}));
    host.exhausted();
    result
}
fn assert_notice(result: &Value, expected: &Value) {
    assert_eq!(result["ok"], true);
    assert_eq!(result["mode"], "no-view");
    assert_eq!(&result["effects"], expected);
    assert!(result.get("root").is_none());
    assert!(result.get("sessionId").is_none());
    if expected[0]["kind"] == "hud" {
        assert_eq!(result["message"], expected[0]["title"]);
    } else {
        assert!(result.get("message").is_none());
    }
}

#[tokio::test]
async fn original_js_oracle_matches_transformation_notices_and_exact_route_order() {
    for vector in oracle()["vectors"].as_array().unwrap() {
        let input = vector["input"].as_str().unwrap();
        let mut replies = vec![Ok(json!(input))];
        if !input.is_empty() {
            // Successful beta clipboard writes may return a boolean.
            replies.push(Ok(json!(true)));
            assert_eq!(
                transform(input),
                vector["calls"][1][1][0],
                "{}",
                vector["name"]
            );
        }
        let host = RecordingHost::new(replies);
        let result = run(&host).await;
        assert_notice(&result, &vector["effects"]);
        assert_eq!(host.calls(), vector["calls"], "{}", vector["name"]);
        // No input leaks into the returned result. Its only text is the notice.
        let allowed = if input.is_empty() {
            json!({"ok":true,"mode":"no-view","effects":vector["effects"]})
        } else {
            json!({"ok":true,"mode":"no-view","message":"Removed formatting from clipboard","effects":vector["effects"]})
        };
        assert_eq!(result, allowed);
    }
}

#[tokio::test]
async fn read_and_write_errors_and_both_failure_envelopes_match_v1_without_success() {
    for vector in oracle()["failures"].as_array().unwrap() {
        for envelope in ["err", "error", "message", "missing-message"] {
            let message = if vector["kind"] == "error" {
                format!("{} denied", vector["failAt"].as_str().unwrap())
            } else {
                "Unknown error encountered".into()
            };
            let failure = match envelope {
                "err" => Err(message.clone()),
                "error" => Ok(json!({"ok":false,"error":message})),
                "message" => Ok(json!({"ok":false,"message":message})),
                _ if vector["kind"] == "non-error" => Ok(json!({"ok":false})),
                _ => continue,
            };
            let replies = if vector["failAt"] == "read" {
                vec![failure]
            } else {
                vec![Ok(vector["input"].clone()), failure]
            };
            let host = RecordingHost::new(replies);
            let result = run(&host).await;
            assert_notice(&result, &vector["effects"]);
            assert_eq!(host.calls(), vector["calls"]);
            assert!(!result.to_string().contains("Removed formatting"));
        }
    }
}

#[tokio::test]
async fn malformed_clipboard_results_fail_without_write_and_error_field_has_precedence() {
    for value in [
        Value::Null,
        json!(false),
        json!({"text":"unsafe"}),
        json!(["unsafe"]),
    ] {
        let host = RecordingHost::new(vec![Ok(value)]);
        let result = run(&host).await;
        assert_notice(
            &result,
            &json!([{"kind":"toast","style":"failure","title":"Error","message":"Clipboard did not return text."}]),
        );
        assert_eq!(host.calls(), json!([["clipboard:read", []]]));
    }
    let host = RecordingHost::new(vec![Ok(
        json!({"ok":false,"error":"primary","message":"secondary"}),
    )]);
    assert_eq!(run(&host).await["effects"][0]["message"], "primary");
}

#[tokio::test]
async fn rejects_unknown_commands_and_operations_and_disposes_without_any_host_io() {
    let host = RecordingHost::new(vec![]);
    for state in [Value::Null, json!({}), json!({"foreign":"preserved"})] {
        let mut current = state.clone();
        for command in ["index", "unknown"] {
            for operation in [
                "extension:dispose-session",
                "extension:refresh-session",
                "extension:invoke-action",
                "extension:search-text-changed",
                "extension:load-more",
                "run-command",
                "",
                "unknown",
            ] {
                let result = handle(
                    &host,
                    command,
                    operation,
                    &json!({"text":"ignored"}),
                    &mut current,
                )
                .await;
                if command == "index" && operation == "extension:dispose-session" {
                    assert_eq!(result.unwrap(), json!({"ok":true,"mode":"unchanged"}));
                } else {
                    assert!(result.is_err());
                }
                assert_eq!(current, state);
            }
        }
        assert!(handle(
            &host,
            "unknown",
            "extension:run-command",
            &Value::Null,
            &mut current
        )
        .await
        .is_err());
    }
    assert_eq!(host.calls(), json!([]));
}

#[tokio::test]
async fn payload_and_arbitrary_state_cannot_override_clipboard_content_or_add_io() {
    for mut state in [Value::Null, json!([]), json!({"input":"forged"})] {
        let initial = state.clone();
        let host = RecordingHost::new(vec![Ok(json!("<b>clipboard</b>")), Ok(json!({"ok":true}))]);
        let result = handle(
            &host,
            "index",
            "extension:run-command",
            &json!({"text":"forged","formValues":{"text":"forged"},"preferences":{"paste":true}}),
            &mut state,
        )
        .await
        .unwrap();
        assert_eq!(state, initial);
        assert_eq!(
            host.calls(),
            json!([["clipboard:read", []], ["clipboard:write", ["clipboard"]]])
        );
        assert_eq!(result["effects"][0]["kind"], "hud");
        host.exhausted();
    }
}
