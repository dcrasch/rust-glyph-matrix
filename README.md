# rust-glyph-matrix

An egui Android app for controlling the back LED (Glyph) lights on the **Nothing Phone (2)**.

Built with Rust + eframe/egui, targeting Android via `cargo-apk`. The same codebase also runs as a desktop binary for fast UI iteration.

## Features

- Toggle individual Glyph zones (A1, A2, B1, C1 ring, C2–C6 dots, E1 bar, D1 strip)
- Progress bar display on the D1/C1 LED strip
- Breathing animation mode
- All Off button

## Project layout

```
rust-glyph-matrix/
├── Cargo.toml             # workspace + Android library package
├── build.rs               # extracts AAR, compiles GlyphBridge.java, runs d8
├── src/lib.rs             # egui app + JNI bridge
├── desktop/               # desktop launcher
│   ├── Cargo.toml
│   └── src/main.rs
├── android/
│   └── src/main/java/dev/rustglyphmatrix/
│       └── GlyphBridge.java   # Java wrapper for the Glyph SDK
├── libs/
│   └── glyph-matrix-sdk-2.0.aar  # Nothing Glyph SDK
└── justfile
```

## Requirements

| Tool            | Version          |
|-----------------|------------------|
| Rust            | 1.85+            |
| Android NDK     | 30.0.16248370    |
| Android SDK     | API 35           |
| Android build-tools | 36+          |
| JDK             | 11+              |
| `cargo-apk`     | 0.10             |

Environment variables:
```sh
export ANDROID_HOME="$HOME/Android/Sdk"
export ANDROID_NDK_HOME="$ANDROID_HOME/ndk/30.0.16248370"
```

## Usage

```sh
just setup            # install Rust Android targets + cargo-apk
just desktop          # run desktop preview (no phone needed)
just build            # build APK + inject Glyph SDK dex
just run              # build, install, launch, follow logs
just logs             # stream logcat from the app
just stop             # force-stop on phone
just uninstall        # remove from phone
```

## How it works

`cargo-apk` builds the Rust shared library and wraps it in an APK that uses Android's `NativeActivity`. It does not support Java/AAR dependencies natively.

**`build.rs`** bridges the gap at build time:
1. Extracts `classes.jar` from `libs/glyph-matrix-sdk-2.0.aar`
2. Compiles `GlyphBridge.java` with `javac` (against `android.jar` and the SDK jar)
3. Runs `d8` to merge everything into `classes.dex`

**`just build`** then injects that `classes.dex` into the generated APK, re-aligns, and re-signs it.

At runtime, the Rust side calls `GlyphBridge.init(context)` via JNI to get a bridge object, then calls `toggle()`, `animate()`, `setProgress()`, and `turnOff()` on it. `GlyphBridge` wraps the Nothing `GlyphManager` and handles the async service-connection lifecycle.

## Nothing Phone (2) Glyph zones

| Zone     | LEDs                        | Bit |
|----------|-----------------------------|-----|
| A1       | Top-left rectangle          | 0   |
| A2       | Top-right rectangle         | 1   |
| B1       | Camera ring (outer)         | 2   |
| C1 ring  | Camera ring (16 segments)   | 3   |
| C2–C6    | Small dots                  | 4   |
| E1       | Diagonal bar                | 5   |
| D1 strip | Progress strip (8 segments) | 6   |

The SDK groups A1+A2 into a single `buildChannelA()` call and C1 ring + C dots into `buildChannelC()`.

## Notes

- The Glyph SDK only works on **Nothing devices running Android 14+**.
- Only **foreground** apps can control Glyphs.
- The `NothingKey = "test"` value in the manifest works for debug builds. For production, register at the Nothing developer portal.
- Enable SDK debug mode on-device: `adb shell settings put global nt_glyph_interface_debug_enable 1`
# rust-glyph-matrix
# rust-glyph-matrix
