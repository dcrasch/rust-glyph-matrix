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
build: _build_apk
    just _inject_dex {{apk_debug}}

# Build a release APK and inject the Glyph dex.
build-release: _build_apk_release
    just _inject_dex {{apk_release}}

# Build, install on the connected phone, and stream logs.
run: build _install_debug _launch _logs

# Build and install without streaming logs.
run-no-logcat: build _install_debug _launch

# Install an already-built debug APK.
install: _install_debug

# Clear logcat and stream this app's logs.
logs:
    adb logcat -c
    just _logs
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

# Inject classes.dex into an APK, re-align it, and re-sign it with the debug key.
# Usage: just _inject_dex <apk>
_inject_dex apk:
    #!/usr/bin/env bash
    set -euo pipefail
    DEX=$(find target -name "classes.dex" -path "*/dex/classes.dex" | head -1)
    if [ -z "$DEX" ]; then
        echo "WARNING: classes.dex not found — Glyph SDK will be absent from APK"
        exit 0
    fi
    python3 scripts/inject_dex.py "$DEX" "{{apk}}"
    BUILD_TOOLS="${ANDROID_HOME}/build-tools"
    ZIPALIGN=$(ls "$BUILD_TOOLS"/*/zipalign 2>/dev/null | sort -V | tail -1)
    APKSIGNER=$(ls "$BUILD_TOOLS"/*/apksigner 2>/dev/null | sort -V | tail -1)
    DEBUG_KS="${HOME}/.android/debug.keystore"
    "$ZIPALIGN" -f 4 "{{apk}}" "{{apk}}.aligned"
    mv "{{apk}}.aligned" "{{apk}}"
    "$APKSIGNER" sign --ks "$DEBUG_KS" --ks-pass pass:android \
        --key-pass pass:android --ks-key-alias androiddebugkey \
        "{{apk}}"
    echo "APK ready: {{apk}}"

_install_debug:
    adb install -r {{apk_debug}}

_launch:
    adb shell am start -n {{app_package}}/android.app.NativeActivity

_logs:
    adb logcat -s rust-glyph-matrix:V AndroidRuntime:E GlyphBridge:V
