# rust-glyph-matrix — Nothing Phone (2) Glyph LED controller
# Usage: just <recipe>

export ANDROID_HOME        := env_var_or_default("ANDROID_HOME", home_directory() + "/Android/Sdk")
export ANDROID_NDK_HOME    := env_var_or_default("ANDROID_NDK_HOME", ANDROID_HOME + "/ndk/30.0.16248370")
export JAVA_HOME           := env_var_or_default("JAVA_HOME", home_directory() + "/android-studio/jbr")
export PATH                := JAVA_HOME + "/bin:" + ANDROID_HOME + "/platform-tools:" + env_var("PATH")

app_package := "dev.rustglyphmatrix.app"
apk_debug   := "target/debug/apk/rust-glyph-matrix.apk"
apk_release := "target/release/apk/rust-glyph-matrix.apk"

# Install Rust Android targets and cargo-apk.
setup:
    rustup target add aarch64-linux-android
    cargo install cargo-apk --locked

# Run desktop preview (no Android needed).
desktop:
    cargo run -p rust-glyph-matrix-desktop

# Build the debug APK and inject the Glyph dex.
build: _build_apk _inject_dex_debug

# Build a release APK and inject the Glyph dex.
build-release: _build_apk_release _inject_dex_release

# Build, install on the connected phone, and stream logs.
run: build _install_debug _launch _logs

# Build and install without streaming logs.
run-no-logcat: build _install_debug _launch

# Install an already-built debug APK.
install: _install_debug

# Stream this app's logcat.
logs:
    adb logcat -c
    adb logcat -s rust-glyph-matrix:V AndroidRuntime:E GlyphBridge:V

# Force-stop the app.
stop:
    adb shell am force-stop {{app_package}}

# Uninstall the app from the phone.
uninstall:
    adb shell pm uninstall {{app_package}} || true

# Print environment info.
doctor:
    @echo "ANDROID_HOME     = ${ANDROID_HOME}"
    @echo "ANDROID_NDK_HOME = ${ANDROID_NDK_HOME}"
    @adb version
    @rustup target list --installed | grep android

# ---------------------------------------------------------------------------
# Internal recipes
# ---------------------------------------------------------------------------

_build_apk:
    cargo apk build

_build_apk_release:
    cargo apk build --release

# Locate the classes.dex produced by build.rs and inject it into the APK.
_inject_dex_debug:
    #!/usr/bin/env bash
    set -euo pipefail
    DEX=$(find target -name "classes.dex" -path "*/dex/classes.dex" | head -1)
    if [ -z "$DEX" ]; then
        echo "WARNING: classes.dex not found — Glyph SDK will be absent from APK"
        exit 0
    fi
    python3 scripts/inject_dex.py "$DEX" "{{apk_debug}}"
    BUILD_TOOLS="${ANDROID_HOME}/build-tools"
    ZIPALIGN=$(ls "$BUILD_TOOLS"/*/zipalign 2>/dev/null | sort -V | tail -1)
    APKSIGNER=$(ls "$BUILD_TOOLS"/*/apksigner 2>/dev/null | sort -V | tail -1)
    DEBUG_KS="${HOME}/.android/debug.keystore"
    "$ZIPALIGN" -f 4 "{{apk_debug}}" "{{apk_debug}}.aligned"
    mv "{{apk_debug}}.aligned" "{{apk_debug}}"
    "$APKSIGNER" sign --ks "$DEBUG_KS" --ks-pass pass:android \
        --key-pass pass:android --ks-key-alias androiddebugkey \
        "{{apk_debug}}"
    echo "APK ready: {{apk_debug}}"

_inject_dex_release:
    #!/usr/bin/env bash
    set -euo pipefail
    DEX=$(find target -name "classes.dex" -path "*/dex/classes.dex" | head -1)
    if [ -z "$DEX" ]; then
        echo "WARNING: classes.dex not found — Glyph SDK will be absent from APK"
        exit 0
    fi
    python3 scripts/inject_dex.py "$DEX" "{{apk_release}}"
    BUILD_TOOLS="${ANDROID_HOME}/build-tools"
    ZIPALIGN=$(ls "$BUILD_TOOLS"/*/zipalign 2>/dev/null | sort -V | tail -1)
    APKSIGNER=$(ls "$BUILD_TOOLS"/*/apksigner 2>/dev/null | sort -V | tail -1)
    DEBUG_KS="${HOME}/.android/debug.keystore"
    "$ZIPALIGN" -f 4 "{{apk_release}}" "{{apk_release}}.aligned"
    mv "{{apk_release}}.aligned" "{{apk_release}}"
    "$APKSIGNER" sign --ks "$DEBUG_KS" --ks-pass pass:android \
        --key-pass pass:android --ks-key-alias androiddebugkey \
        "{{apk_release}}"
    echo "APK ready: {{apk_release}}"

_install_debug:
    adb install -r {{apk_debug}}

_launch:
    adb shell am start -n {{app_package}}/android.app.NativeActivity

_logs:
    adb logcat -s rust-glyph-matrix:V AndroidRuntime:E GlyphBridge:V
