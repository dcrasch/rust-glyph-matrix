# rust-glyph-matrix

Control the back Glyph LEDs on the **Nothing Phone (2)** from Rust.

The app is an egui interface on Android, built with `cargo-apk`. It also
runs as a desktop binary, which shows the same UI without touching the LEDs,
for quick layout work.

## Features

- Toggle individual Glyph zones by clicking them on the phone diagram
- Progress fill on the camera ring (glyph 4) and the USB strip (glyph 10)
- Breathing animation
- Demo mode that steps through every zone
- All Off

## Using the library

The Glyph API is in `src/glyph.rs` as `GlyphController`. It's a standalone
module, so other apps can depend on this crate and use it directly:

```rust
use rust_glyph_matrix::{GLYPH_COUNT, glyph::GlyphController};

let glyphs = GlyphController::new()?;
let mut flags = [false; GLYPH_COUNT];
flags[0] = true;               // glyphId 1, camera top
glyphs.show(&flags)?;
glyphs.set_progress(&flags, 40)?;
glyphs.turn_off()?;
```

Run `cargo doc --no-deps --target aarch64-linux-android --open` for the full
API reference.

Glyph index `i` is glyphId `i + 1`. See the zone table below.

## Project layout

```
rust-glyph-matrix/
├── Cargo.toml                # workspace root + Android package metadata
├── build.rs                  # compiles GlyphBridge.java and dexes it (Android only)
├── src/
│   ├── lib.rs                # egui app
│   └── glyph.rs              # GlyphController: JNI wrapper around the Glyph SDK
├── desktop/                  # desktop launcher
├── android/src/main/java/dev/rustglyphmatrix/
│   └── GlyphBridge.java      # Java side: integer-mask API over the Glyph SDK
├── assets/
│   ├── phone_back.svg        # phone diagram with all zones
│   └── glyph_<id>.png        # lit overlay per glyphId (generated)
├── libs/
│   └── glyph-matrix-sdk-2.0.aar  # Nothing Glyph SDK
├── scripts/
│   ├── render_phone_back.py  # regenerates assets/glyph_<id>.png from the SVG
│   └── inject_dex.py         # adds classes.dex to a built APK
└── justfile
```

## Requirements

| Tool                | Version        |
|---------------------|----------------|
| Rust                | 1.85+          |
| Android NDK         | 30.0.16248370  |
| Android SDK         | API 35         |
| Android build-tools | 36+            |
| JDK                 | 11+            |
| `cargo-apk`         | 0.10           |

```sh
export ANDROID_HOME="$HOME/Android/Sdk"
export ANDROID_NDK_HOME="$ANDROID_HOME/ndk/30.0.16248370"
```

## Usage

```sh
just setup            # install Rust Android target and cargo-apk
just desktop          # run the desktop preview (no phone needed)
just build            # build the debug APK and inject the Glyph dex
just build-release    # same, for release
just run              # build, install, launch, and stream logs
just run-no-logcat    # build, install, and launch
just logs             # stream this app's logcat
just stop             # force-stop the app on the phone
just uninstall        # remove the app from the phone
just doctor           # print environment info
```

Set `GLYPH_DEMO=1` before `just desktop` to start in demo mode.

## How it works

`cargo-apk` builds the Rust shared library and wraps it in an APK that uses
Android's `NativeActivity`. It can't package Java or AAR dependencies, so the
Java side is added in two steps.

1. **`build.rs`** runs during the Android build. It extracts `classes.jar`
   from the Glyph AAR, compiles `GlyphBridge.java` with `javac`, and runs
   `d8` to produce `classes.dex`.
2. **`just build`** injects that `classes.dex` into the APK, re-aligns it,
   and re-signs it.

At runtime, `GlyphController::new()` loads `GlyphBridge` from the APK through
`DexClassLoader`, because `NativeActivity` doesn't expose the app's classes
to the default class loader. It then calls `GlyphBridge.init(context)`. The
Java bridge connects to the Glyph service, and the Rust methods call
`show`, `animate`, `setProgress`, and `turnOff` through JNI.

## Nothing Phone (2) Glyph zones

Glyph index is the position in the Rust array (`glyphs[i]`). The bit is the
position in the integer mask sent to `GlyphBridge`.

| Index | glyphId | Bit | Zone                              | SDK channels |
|-------|---------|-----|-----------------------------------|--------------|
| 0     | 1       | 0   | Camera top                        | 0            |
| 1     | 2       | 1   | Camera bottom                     | 1            |
| 2     | 3       | 2   | Diagonal                          | 2            |
| 3     | 4       | 3   | Battery top right: ring, 16 segs  | 3–18         |
| 4     | 5       | 4   | Battery top left                  | 19           |
| 5     | 6       | 5   | Battery top vertical (left)       | 20           |
| 6     | 7       | 6   | Battery bottom left               | 21           |
| 7     | 8       | 7   | Battery bottom right              | 22           |
| 8     | 9       | 8   | Battery bottom vertical (right)   | 23           |
| 9     | 10      | 9   | USB line: strip, 8 segs           | 25–32        |
| 10    | 11      | 10  | USB dot                           | 24           |

The SDK's channel names don't match the physical layout. For example, channel
24 is labeled `E1` but lights the USB dot. `GlyphBridge.java` maps glyphs to
channels, and the table above follows it.

`set_progress` fills glyph 4 (ring) and glyph 10 (strip) in proportion to the
value, and ignores their bits in the mask.

## Notes

- The Glyph SDK only works on **Nothing devices running Android 14+**.
- Only the **foreground** app can control the Glyphs.
- `NothingKey = "test"` in the manifest works for debug builds. For a
  release build, register with the Nothing developer portal.
- Turn on SDK debug logging on the device with
  `adb shell settings put global nt_glyph_interface_debug_enable 1`.
- Calls made before the Glyph service connects are ignored. Watch for
  `Glyph session opened` in `just logs` before you test.
