//! MIT translation of joshtemple's Clipboard Formatter; Tezbar port by almatkai.
//! OS effects belong to the injected host. See LICENSE and fixture provenance.
use serde_json::{json, Value};
use std::{future::Future, pin::Pin};

pub const ICON: &[u8] = include_bytes!("../assets/extension_icon.png");
pub const MANIFEST: &str = include_str!("../manifest.json");
pub type Result = std::result::Result<Value, String>;
pub type HostFuture<'a> = Pin<Box<dyn Future<Output = Result> + Send + 'a>>;

/// Positional request ABI of beta CoreContext. No clipboard implementation here.
pub trait Host: Sync {
    fn request<'a>(&'a self, route: &'a str, args: Vec<Value>) -> HostFuture<'a>;
}

pub fn commands() -> &'static [&'static str] {
    &["index"]
}

// ECMAScript WhiteSpace + LineTerminator: FEFF is included, U+0085 is not.
fn js_whitespace(c: char) -> bool {
    matches!(c, '\u{0009}'..='\u{000d}' | '\u{0020}' | '\u{00a0}' | '\u{1680}' |
        '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' |
        '\u{205f}' | '\u{3000}' | '\u{feff}')
}

/// Exactly V1's /<[^>]*>?/gm + trim, then /[\r\n]+/gm + trim.
/// This intentionally treats quoted '>', nested '<', and unterminated tags as
/// regex text. Entities remain encoded; only runs of CR and LF are collapsed.
pub fn transform(text: &str) -> String {
    let mut stripped = String::with_capacity(text.len());
    let mut in_tag = false;
    for c in text.chars() {
        if in_tag {
            if c == '>' {
                in_tag = false;
            }
        } else if c == '<' {
            in_tag = true;
        } else {
            stripped.push(c);
        }
    }
    let stripped = stripped.trim_matches(js_whitespace);
    let mut collapsed = String::with_capacity(stripped.len());
    let mut in_newlines = false;
    for c in stripped.chars() {
        if matches!(c, '\r' | '\n') {
            if !in_newlines {
                collapsed.push('\n');
            }
            in_newlines = true;
        } else {
            collapsed.push(c);
            in_newlines = false;
        }
    }
    collapsed.trim_matches(js_whitespace).to_owned()
}

async fn request<H: Host + ?Sized>(host: &H, route: &str, args: Vec<Value>) -> Result {
    let value = host.request(route, args).await?;
    if value["ok"] == false {
        return Err(value["error"]
            .as_str()
            .or_else(|| value["message"].as_str())
            .unwrap_or("Unknown error encountered")
            .to_owned());
    }
    Ok(value)
}

fn failure(title: &str, message: &str) -> Value {
    // ok describes a handled command; the toast style describes its failure.
    json!({"ok":true,"mode":"no-view","effects":[{
        "kind":"toast","style":"failure","title":title,"message":message
    }]})
}

async fn run<H: Host + ?Sized>(host: &H) -> Result {
    let content = request(host, "clipboard:read", vec![]).await?;
    let text = content.as_str().ok_or("Clipboard did not return text.")?;
    if text.is_empty() {
        return Ok(failure(
            "No text content found",
            "Please copy some text or HTML content to the clipboard.",
        ));
    }
    // V1 tests truthiness BEFORE transforming. Even empty/unchanged results
    // must be written, since writing plaintext clears rich clipboard formats.
    request(host, "clipboard:write", vec![json!(transform(text))]).await?;
    Ok(json!({"ok":true,"mode":"no-view",
        "message":"Removed formatting from clipboard",
        "effects":[{"kind":"hud","title":"Removed formatting from clipboard"}]}))
}

/// Actual beta operations. Clipboard text never enters state or logs.
pub async fn handle<H: Host + ?Sized>(
    host: &H,
    command: &str,
    operation: &str,
    _payload: &Value,
    _state: &mut Value,
) -> Result {
    if !commands().contains(&command) {
        return Err("Unknown Clipboard Formatter command".into());
    }
    match operation {
        "extension:run-command" => match run(host).await {
            Ok(value) => Ok(value),
            Err(message) => Ok(failure("Error", &message)),
        },
        "extension:dispose-session" => Ok(json!({"ok":true,"mode":"unchanged"})),
        _ => Err("Unsupported Clipboard Formatter operation".into()),
    }
}
