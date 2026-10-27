# Clipboard Formatter — Native V2 Rust

A standalone Rust library implementing **Strip Clipboard Formatting** (`index`,
no-view). Original extension: **joshtemple**, copyright (c) 2021 Raycast, MIT.
Tezbar port: **almatkai**, copyright (c) 2026. V1 TypeScript, package metadata,
lockfile, icon and root license are unchanged.

Reference API: **Tezbar beta `fd300e276356587726566026f072563eb9c34cb1`**,
read-only `git show` of `docs/NATIVE_V2_FORMAT.md`, the schema, `v2_sdk.rs`,
`v2_runtime.rs`, `native.rs`, `v2_perplexity.rs`, and `common.rs`.
The crate is compiled into the host; it has no executable target, stdin protocol,
WASM, custom loader, preferences, form or saved session data.

## Build and test

```sh
cargo build --locked
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
cargo +1.85.0 test --locked
cargo check --target x86_64-pc-windows-gnu --all-targets --locked
cargo clippy --target x86_64-pc-windows-gnu --all-targets --locked -- -D warnings
cargo package --allow-dirty --locked
```

Run from `v2/`. Minimum Rust is **1.85**, with `Cargo.lock` included. The sole
runtime dependency is `serde_json`; SHA-256, JSON Schema, Tokio and the real beta
CoreContext support are test-only. Package contents include all tests, fixtures,
icon, adapter and licenses. `target/` and `node_modules/` are ignored/excluded.
The `.crate` archive can be extracted and tested with `cargo test --locked`
without the V1 tree or Node. Cross-target checking requires the Rust Windows
standard library; it does not run Windows binaries or certify Windows UI.

The test-only V1 oracle runs the entire original command through VM with a fake
Clipboard, preserving both regex expressions and JS truthiness. It covers 60
bounded examples: Unicode, all ECMAScript trim characters, FEFF versus U+0085,
CR/LF runs, internal Unicode line separators, multiline/nested/unclosed tags,
quoted `>`, entities, tags-only and whitespace-only text. Four source error
cases verify original toast structure. Rust tests also cover transport errors,
both failure envelope fields, malformed read results, exact effect order,
unchanged-text writes, command/operation rejection, harmless dispose, schema,
metadata, pinned fixture hashes, icon bytes and the real CoreContext wrapper.
All clipboard requests are in-memory fakes. No real clipboard or UI action runs.

Optional oracle regeneration (built-in Node modules only):

```sh
node tests/generate_v1_vectors.mjs
```

V1 checks should run on disposable copies under `target/v1-check` at the repository
root. Copy the original source, tests, metadata, TypeScript configuration and npm
lockfile there; run `pnpm import`, `pnpm install --frozen-lockfile --ignore-scripts`,
`pnpm test`, and `pnpm typecheck` in that copy. This keeps V1 files and its lockfile
unchanged and avoids installing into the source tree.

## API and behavior

```rust,ignore
pub fn commands() -> &'static [&'static str]; // ["index"]
pub const ICON: &[u8];
pub const MANIFEST: &str;
pub fn transform(text: &str) -> String;
pub async fn handle<H: Host + ?Sized>(
    host: &H,
    command: &str,
    operation: &str,
    payload: &serde_json::Value,
    state: &mut serde_json::Value,
) -> Result<serde_json::Value, String>;
```

`Host::request(route, args)` returns a Send boxed asynchronous future.
`tezbar_adapter.rs` supplies the concrete beta CoreContext wrapper with the
unchanged host handler signature. Supported operations:

- `extension:run-command`: await `clipboard:read` with `[]`, which returns a
  string. If it is nonempty, apply V1's `/<[^>]*>?/gm` removal and ECMAScript
  `trim()`, then `/[\r\n]+/gm` replacement with one LF and `trim()` again.
  Await `clipboard:write` with `[plainText]`, then return the no-view success
  message and `{kind:"hud", title:"Removed formatting from clipboard"}` effect.
- Empty original text returns a no-view failure toast titled **No text content
  found**, message **Please copy some text or HTML content to the clipboard.**
  It performs no write. Whitespace-only and tags-only original text are truthy
  and **write an empty string with success**. Unchanged plaintext is also always
  written to remove rich clipboard formats.
- Request errors and `{ok:false,error/message}` envelopes return a no-view
  `{kind:"toast",style:"failure",title:"Error",message:originalError}` effect.
  A missing envelope error/message uses V1's **Unknown error encountered**.
  A non-string clipboard response gives **Clipboard did not return text.**
  Read failure never writes; write failure never returns the success HUD/message.
  `ok:true` on a toast response means the command was handled; its failure
  style carries the source failure, using beta's existing effects format.
- `extension:dispose-session`: return `{ok:true,mode:"unchanged"}` with no
  requests or state changes. Unknown commands and operations return `Err`.

The regex intentionally consumes the remainder after an unmatched `<`, stops
at the first `>` even inside quoted attributes, and does not decode entities.
Only CR/LF runs collapse; internal U+2028/U+2029 remain. FEFF trims; U+0085 does
not. Payload cannot replace clipboard input. Text remains local to each call;
no clipboard content is put in returned metadata, session state, storage or logs.
The host owns cancellation and installed-command checks. There are no explicit
hide, paste or browser requests, matching the original source.

## Register in the pinned beta host

These are later host integration instructions; this repository does not edit
Tezbar or the installed profile. **A host rebuild is required.** The current
store cannot directly install executable Rust from this repository; a manifest
or `native-v2.json` marker alone cannot add compiled dispatch.

1. Vendor the contents of `v2/` at
   `src-tauri/native-extensions/clipboard-formatter`. Retain its licenses,
   Cargo.lock and provenance. Add to **`src-tauri/Cargo.toml`**:

   ```toml
   tezbar-clipboard-formatter-v2 = { path = "native-extensions/clipboard-formatter" }
   ```

   Update/review the host's own Cargo.lock. Copy **`tezbar_adapter.rs` unchanged**
   to **`src-tauri/src/native_core/extensions/v2_clipboard_formatter.rs`**.

2. Add `mod v2_clipboard_formatter;` in
   **`src-tauri/src/native_core/extensions/mod.rs`**. In
   **`src-tauri/src/native_core/extensions/v2_runtime.rs`**, allow
   `"clipboard-formatter"` in `slug()`, and add to `commands()` and `handle()`:

   ```rust,ignore
   "clipboard-formatter" => super::v2_clipboard_formatter::commands(),
   "clipboard-formatter" => super::v2_clipboard_formatter::handle(
       ctx, command, operation, payload, state
   ).await,
   ```

   Keep the existing canonical ID, installed-command and lifecycle checks.
   The canonical ID is `com.tezbar.native.clipboard-formatter`, replacing
   `raycast.clipboard-formatter`.

3. Append this crate's **manifest object** from `manifest.json` to
   **`src-tauri/src/native_core/extensions/v2_catalog.json`**. Validate it with
   **`schemas/extension-v2.schema.json`**. It declares exactly `index`, no-view,
   Rust, macOS + Windows, and no preferences (as in V1). Author attribution is
   in description because the actual beta schema rejects an `author` field.
   Update tests depending on the previous catalog size and assert compiled
   `index` coverage on both declared platforms.

4. Copy **`assets/extension_icon.png` byte-for-byte** to
   **`src/renderer/public/extensions/icons/clipboard-formatter.png`**.
   This is beta's Vite `publicDir`; the manifest URL must remain exactly
   `/extensions/icons/clipboard-formatter.png`. `ICON` embeds the same 32743
   original bytes; SHA-256 and pinned URL are in [icon provenance](assets/PROVENANCE.md).
   Beta's catalog installer only writes JSON. If an installed-package fallback
   icon is needed, add this asset write there before marker/database success:

   ```rust,ignore
   if slug == "clipboard-formatter" {
       std::fs::create_dir_all(root.join("assets")).map_err(|e| e.to_string())?;
       std::fs::write(
           root.join("assets/extension_icon.png"),
           super::v2_clipboard_formatter::ICON,
       ).map_err(|e| e.to_string())?;
   }
   ```

   The managed fallback belongs under
   `extensions/packages/com.tezbar.native.clipboard-formatter/assets/extension_icon.png`;
   resolve it explicitly if used. Keep the catalog URL and original V1 icon.

5. Rebuild/test the host, inspect actual compiled coverage using its V2 CLI,
   and install the canonical ID using the host's normal Native V2 catalog route.
   Only after registration/rebuild can the catalog execute this Rust handler.
   Test denied clipboard access, empty text, rich-text-only/no text, whitespace,
   malformed tags, Unicode, and removal of rich formats even for unchanged text
   on real macOS and Windows. Verify the HUD/toasts and focus behavior in the UI.

**Not run:** full Tezbar integration/rebuild, real macOS/Windows clipboard, UI,
permissions or rich-format/focus checks. Both platform verification entries
remain `not-run`; declarations and cross-compilation are not real OS QA.

See [fixture provenance](tests/fixtures/PROVENANCE.md) for byte hashes and
licenses. The original/root V1 license is preserved. V2's MIT license has actual
newlines and both Raycast/almatkai copyright notices; the included beta fixtures
retain their original Apache-2.0 license (`MIT AND Apache-2.0` package SPDX).
