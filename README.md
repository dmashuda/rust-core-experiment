# rust-core-experiment

A Bazel 9 monorepo where one Rust core powers a web app, a macOS app and an
iOS app, with Kotlin bindings ready for Android. Everything is built and tested
by Bazel. The calculator is just the vehicle. **See [LOG.md](LOG.md) for the
research log and every gotcha hit along the way.**

```
core/calc_core          Rust: expression evaluator + stateful Calculator (pure, no FFI)
bindings/wasm           wasm-bindgen wrapper   → JS/TS (web)
bindings/uniffi         UniFFI wrapper         → Swift (Apple) + Kotlin (Android/JVM)
tools/uniffi            uniffi-bindgen(-swift) built from source as rust_binary
apps/web                TypeScript + esbuild, served by a tiny Node server
apps/apple/CalculatorUI SwiftUI view + model shared by macOS and iOS
apps/macos, apps/ios    macos_application / ios_application
```

| Command | What it does |
| --- | --- |
| `bazel test //...` | Rust unit tests, wasm-in-Node test, tsc typecheck, Swift XCTest (bindings + UI snapshot), Kotlin JUnit via JNA |
| `bazel run //apps/web:serve` | Web app on http://localhost:8080 |
| `bazel run //apps/macos:CalculatorMac` | Build and launch the macOS app |
| `bazel build //apps/ios:CalculatorIOS` | iOS simulator `.ipa` |
| `bazel build --config=release //apps/web:site` | Optimized web bundle (~105 KB wasm) |

Requirements: macOS with Xcode (for the Apple targets) and bazelisk. Bazel
downloads Rust, Node and the JDK itself.

| Web | macOS (offscreen snapshot test) |
| --- | --- |
| <img src="docs/web.png" width="280"> | <img src="docs/macos.png" width="280"> |
