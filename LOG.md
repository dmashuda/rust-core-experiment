# Research log: Bazel monorepo with a Rust core and native front ends

Goal: a single Bazel workspace where a Rust core is consumed by a web app,
a macOS app, an iOS app and an Android app, all built by Bazel. The calculator
is only a vehicle; the point is to learn how the build graph fits together.

Entries are chronological. "Finding" = something learned that would matter
to anyone repeating this.

---

## 0. Environment (2026-10-23)

| Tool | Version |
| --- | --- |
| macOS | 27.0.1, arm64 |
| Xcode | 27.0 |
| Bazel | 9.3.1 (via Homebrew bazelisk 1.27, pinned in `.bazelversion`) |
| Host cargo/rustc | Homebrew; **not used by the build**, Bazel downloads its own |

Bazel Central Registry (BCR) versions at the start:

| Module | Version | Purpose |
| --- | --- | --- |
| rules_rust | 0.74.0 | Rust toolchains, `rust_library` etc., crate_universe |
| rules_rust_wasm_bindgen | 0.74.0 | `rust_wasm_bindgen` (separate BCR module) |
| rules_apple | 5.3.1 | `macos_application`, `ios_application` |
| rules_swift | 4.2.2 | `swift_library` |
| apple_support | 2.10.1 | Apple CC toolchain + Apple platforms |
| aspect_rules_js | 3.5.2 | npm/pnpm and JS tooling |
| rules_android | 0.8.0 | Android (future) |
| rules_kotlin | 2.4.21 | Kotlin (future) |

**Finding: Bazel 9 is bzlmod-only.** `WORKSPACE` is gone, so every dependency
goes in `MODULE.bazel`. Bazel 9 also finished moving native rules into
Starlark: `cc_library`, `objc_library` and the rest must be `load()`ed from
`@rules_cc`, and `sh_*` rules from `@rules_shell`. Many older blog posts and
examples are WORKSPACE-era and do not apply directly.

## 1. Research: architecture

### How a Rust crate reaches each platform

| Front end | Mechanism | Rust artifact | Binding generator |
| --- | --- | --- | --- |
| Web | WebAssembly | `rust_shared_library` built for `wasm32-unknown-unknown` | `wasm-bindgen` (`rust_wasm_bindgen` rule) |
| macOS / iOS | C ABI, linked statically into the app | static lib (`rust_library` provides `CcInfo`) | UniFFI to Swift, plus a C header and modulemap |
| Android | JNI / JNA, `.so` per ABI | `rust_shared_library` (cdylib) | UniFFI to Kotlin |

Chosen layering:

```
core/calc_core       pure Rust logic, no FFI annotations, unit-tested
bindings/uniffi      thin crate: #[uniffi::export] wrappers over calc_core  (Swift, Kotlin)
bindings/wasm        thin crate: #[wasm_bindgen] wrappers over calc_core     (JS)
tools/uniffi         uniffi-bindgen built from source as a rust_binary
apps/macos           SwiftUI, macos_application
apps/web             static page + wasm-bindgen JS
```

Why separate binding crates instead of sprinkling `#[uniffi::export]` and
`#[wasm_bindgen]` across the core: UniFFI does not target the browser (only
experimental wasm via external projects), and wasm-bindgen does not target
Swift or Kotlin. Keeping the core free of either keeps it testable with plain
`rust_test` and lets each binding layer pick its own types.

### Findings from reading rules_rust sources (0.74.0)

- **`rust_library` provides `CcInfo`.** It can be a direct dependency of
  `cc_library`, `objc_library` or `swift_library`, and the Apple linker step
  links the rlib plus Rust std. You do not need `rust_static_library` unless
  you want a standalone `.a` (for example, to feed a tool). See
  `examples/ios` in the rules_rust repo.
- **Cross-compiling needs `extra_target_triples`.** Each target triple you
  build for must be listed in `rust.toolchain(...)`. rules_apple's platform
  transitions (for example to `ios_sim_arm64`) then select the matching Rust
  toolchain by constraint.
- **wasm-bindgen version lock-step.** The `wasm-bindgen` crate your code links
  must be *exactly* the version of the `wasm-bindgen` CLI that post-processes
  the `.wasm`. `rules_rust_wasm_bindgen` 0.74.0 ships both, pinned to
  0.2.121, and exposes the crate as
  `@rules_rust_wasm_bindgen//3rdparty:wasm_bindgen`. Use that label instead of
  pulling wasm-bindgen from your own crate_universe; otherwise you will hit
  the "schema version mismatch" error.
- **`rust_wasm_bindgen` applies its own transition.** It switches
  `--platforms` to `@rules_rust//rust/platform:wasm` (via `target_arch`), so
  you never pass `--platforms` on the command line for the web build.
  `--target` (`web`, `bundler` and so on) is a rule attribute.
- crate_universe supports `crate.spec()` + `crate.from_specs()` in
  `MODULE.bazel`, with no `Cargo.toml` at all. Several independent repos can
  coexist (`from_specs(name = "...")` plus `spec(repositories = [...])`).

### Findings from reading UniFFI sources (0.32.2)

- UniFFI 0.32 has a **proc-macro-only** workflow (`uniffi::setup_scaffolding!()`
  plus `#[uniffi::export]`, no `.udl` and no `build.rs`). Bindings are
  generated in **library mode**: the bindgen reads `UNIFFI_META_*` symbols out
  of the compiled library.
- For Swift, library mode accepts a **static `.a`**. For other languages, the
  CLI requires a cdylib.
- **The bindgen shells out to `cargo metadata` by default.** That breaks in a
  Bazel sandbox, where there is no Cargo workspace. It is behind the
  `cargo-metadata` cargo feature (on by default). Building the bindgen with
  `default-features = false, features = ["cli"]` removes it entirely
  (`#[cfg(feature = "cargo-metadata")]` in `uniffi_bindgen/src/bindings/swift/mod.rs`).
- `uniffi::uniffi_bindgen_swift()` is a ready-made `main` for a Swift-only
  bindgen with `--swift-sources`, `--headers`, `--modulemap` and `--module-name`.
- **Metadata format is version-coupled.** The runtime `uniffi` crate linked
  into the library and the bindgen must be the same version. Pin `=0.32.2` in
  both specs.

## 2. Milestone: Rust core under Bazel

`bazel test //core/...` passes, with a cold build in about 30 s, mostly
downloading the toolchain.

- The Rust toolchain is pinned to 1.98.0 (the rules_rust 0.74 default) with
  edition 2024, set once in `rust.toolchain(edition = ...)` so no BUILD file
  repeats it.
- Observed linker warning:
  `clang: overriding '-mmacosx-version-min=11.0.0' option with '-target arm64-apple-macosx27.0'`.
  rustc passes deployment target 11.0, while Bazel's auto-configured host C
  toolchain targets the running OS. To revisit after registering
  apple_support's toolchain.
