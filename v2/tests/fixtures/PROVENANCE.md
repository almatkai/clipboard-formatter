# Test fixture provenance

Host reference: [Tezbar beta fd300e276356587726566026f072563eb9c34cb1](https://github.com/almatkai/Tezbar/tree/fd300e276356587726566026f072563eb9c34cb1).
All host files were read with `git show` at that commit; no current host files
were used or modified. `beta_common.rs`, the manifest schema and the Apache-2.0
license below are unchanged byte-for-byte. No host `NOTICE` file exists at the
pinned commit. `TEZBAR-LICENSE.txt` includes the original copyright notice.

V1 reference: [clipboard-formatter f5436c9aa2d5413741affcf3581242138b322bdf](https://github.com/almatkai/clipboard-formatter/tree/f5436c9aa2d5413741affcf3581242138b322bdf).
The MIT V1 source and package fixture are unchanged copies, with original
Raycast upstream commit `6734d85992290066fd31a7f5bee3cca3d3545a1a` recorded
in the package. Original extension: joshtemple; Tezbar adaptation: almatkai.

| Fixture | Original path | Bytes | SHA-256 |
| --- | --- | ---: | --- |
| `beta_common.rs` | `src-tauri/src/native_core/common.rs` | 7627 | `280a0a2f0dd04a8570b320aafc0c3ece0f5de8ddb21c363645567d6a35d84da7` |
| `extension-v2.schema.json` | `schemas/extension-v2.schema.json` | 2420 | `6d5afda1b652605982104bdf16d1547547b98dfd707a7b2969316406c49f32af` |
| `TEZBAR-LICENSE.txt` | `LICENSE` | 10750 | `0ed6b1b2760294d4aaaa45be38e166fcc0168e1dd5190423c096fcbe1c94afbf` |
| `v1-index.ts` | `src/index.ts` | 1123 | `b47270a92b58fefd0f213c1b87e9d076c70318ed1c85a7156ff1b11f6e2eef30` |
| `v1-package.json` | `package.json` | 1616 | `3ee87c457e8b6572d43b8e825d79cd460fafbf52b3b3ae834c2195cd6e47225c` |

`v1-vectors.json` is generated from the complete V1 command by
`../generate_v1_vectors.mjs`, using a fake Clipboard and VM. Only import/export
and the catch type annotation are removed for execution. Regex, trim, truthiness
and error expressions are untouched. It contains 60 bounded transformation
cases and four error cases, plus the source hash. Node is optional for regeneration;
all Rust tests and packaged tests run offline without Node or the V1 tree.

The adapter contract test compiles the production `tezbar_adapter.rs` unchanged
against this actual `common.rs`. Only Tauri AppHandle/Emitter are substituted;
the real CoreContext routes to an in-memory recorder. No OS clipboard, window,
paste or browser calls run. The included common.rs configuration unit tests
use disposable temporary directories, never a user profile.

The library and original extension are MIT; the included Tezbar fixtures are
Apache-2.0. Cargo therefore declares `MIT AND Apache-2.0`. Both licenses are
packaged. The repository root license remains untouched.
