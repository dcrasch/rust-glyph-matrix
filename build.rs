//! Build script for rust-glyph-matrix.
//!
//! When building for Android, this script:
//!   1. Extracts `classes.jar` from `libs/glyph-matrix-sdk-2.0.aar`.
//!   2. Compiles `android/src/main/java/dev/rustglyphmatrix/GlyphBridge.java`
//!      against the SDK jar and the Android platform `android.jar`.
//!   3. Runs `d8` to merge everything into `classes.dex` and places it in
//!      `$OUT_DIR/dex/classes.dex`.
//!
//! cargo-apk does not inject a `classes.dex` automatically; a `justfile`
//! target (`just inject-dex`) patches the APK after `cargo apk build`.
//!
//! On non-Android targets the script is a no-op.

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=libs/glyph-matrix-sdk-2.0.aar");
    println!("cargo:rerun-if-changed=android/src/main/java/dev/rustglyphmatrix/GlyphBridge.java");

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os != "android" {
        return;
    }

    let android_home = match env::var("ANDROID_HOME")
        .or_else(|_| env::var("ANDROID_SDK_ROOT"))
    {
        Ok(h) => PathBuf::from(h),
        Err(_) => {
            println!("cargo:warning=ANDROID_HOME not set; skipping Java/dex compilation");
            return;
        }
    };

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // Locate build-tools with d8.
    let build_tools = find_build_tools(&android_home);
    let d8 = build_tools.join("d8");
    if !d8.exists() {
        println!(
            "cargo:warning=d8 not found at {}; skipping dex step",
            d8.display()
        );
        return;
    }

    // Locate the highest available android.jar.
    let android_jar = find_android_jar(&android_home);
    if !android_jar.exists() {
        println!(
            "cargo:warning=android.jar not found; skipping Java compilation"
        );
        return;
    }

    // Extract classes.jar from the AAR.
    let aar = PathBuf::from("libs/glyph-matrix-sdk-2.0.aar");
    if !aar.exists() {
        println!(
            "cargo:warning=libs/glyph-matrix-sdk-2.0.aar not found; \
             run: curl -L -o libs/glyph-matrix-sdk-2.0.aar \
             https://github.com/Nothing-Developer-Programme/Glyph-Developer-Kit/raw/main/sdk/glyph-matrix-sdk-2.0.aar"
        );
        return;
    }
    let sdk_jar = out_dir.join("glyph-sdk-classes.jar");
    extract_classes_jar(&aar, &sdk_jar);

    // Compile GlyphBridge.java.
    let java_src =
        PathBuf::from("android/src/main/java/dev/rustglyphmatrix/GlyphBridge.java");
    let javac_out = out_dir.join("javac");
    fs::create_dir_all(&javac_out).unwrap();

    let javac = find_javac();
    let javac_status = Command::new(&javac)
        .args([
            "--release",
            "11",
            "-cp",
            &format!(
                "{}:{}",
                android_jar.display(),
                sdk_jar.display()
            ),
            "-d",
            javac_out.to_str().unwrap(),
            java_src.to_str().unwrap(),
        ])
        .status()
        .unwrap_or_else(|e| panic!("failed to run javac ({javac:?}): {e}"));

    if !javac_status.success() {
        panic!("javac failed — see output above");
    }

    // Run d8 to produce classes.dex from our bridge + the SDK jar.
    let dex_out = out_dir.join("dex");
    fs::create_dir_all(&dex_out).unwrap();

    // Collect all .class files from javac output.
    let class_files = collect_class_files(&javac_out);

    // d8 is a shell wrapper that calls `java`; ensure the JRE is on PATH.
    let java_bin = find_javac()
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("/usr/bin"));
    let path_with_java = match env::var("PATH") {
        Ok(p) => format!("{}:{p}", java_bin.display()),
        Err(_) => java_bin.display().to_string(),
    };

    let mut d8_cmd = Command::new(&d8);
    d8_cmd
        .env("PATH", &path_with_java)
        .arg("--output")
        .arg(&dex_out)
        .arg("--lib")
        .arg(&android_jar)
        .arg(&sdk_jar); // include SDK classes in the dex
    for f in &class_files {
        d8_cmd.arg(f);
    }

    let d8_status = d8_cmd.status().expect("d8 not found");
    if !d8_status.success() {
        panic!("d8 failed — see output above");
    }

    // Write the dex path so the justfile can find it.
    let dex_path = dex_out.join("classes.dex");
    println!("cargo:rustc-env=GLYPH_DEX_PATH={}", dex_path.display());
    println!(
        "cargo:warning=Glyph dex built at {}",
        dex_path.display()
    );
}

/// Returns the path to the newest `build-tools/<version>/` that contains `d8`.
fn find_build_tools(android_home: &Path) -> PathBuf {
    let dir = android_home.join("build-tools");
    if !dir.exists() {
        return dir.join("unknown");
    }
    let mut versions: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.join("d8").exists())
        .collect();
    versions.sort();
    versions.into_iter().last().unwrap_or(dir.join("unknown"))
}

/// Returns the path to `android.jar` for the highest installed platform.
fn find_android_jar(android_home: &Path) -> PathBuf {
    let dir = android_home.join("platforms");
    if !dir.exists() {
        return dir.join("unknown").join("android.jar");
    }
    let mut jars: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path().join("android.jar"))
        .filter(|p| p.exists())
        .collect();
    jars.sort();
    jars.into_iter().last().unwrap_or_else(|| dir.join("unknown").join("android.jar"))
}

/// Recursively collect all `.class` files under `dir`.
fn collect_class_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                out.extend(collect_class_files(&path));
            } else if path.extension().map_or(false, |e| e == "class") {
                out.push(path);
            }
        }
    }
    out
}

/// Finds the `javac` binary.
///
/// Search order:
///   1. `$JAVA_HOME/bin/javac`
///   2. Android Studio's bundled JBR at `~/android-studio/jbr/bin/javac`
///   3. Plain `javac` on `$PATH`
fn find_javac() -> PathBuf {
    // 1. JAVA_HOME
    if let Ok(java_home) = env::var("JAVA_HOME") {
        let p = PathBuf::from(java_home).join("bin").join("javac");
        if p.exists() {
            return p;
        }
    }

    // 2. Android Studio bundled JBR
    let candidates = [
        "android-studio/jbr/bin/javac",
        "Android/android-studio/jbr/bin/javac",
    ];
    if let Ok(home) = env::var("HOME") {
        for rel in &candidates {
            let p = PathBuf::from(&home).join(rel);
            if p.exists() {
                return p;
            }
        }
    }

    // 3. Fall back to PATH
    PathBuf::from("javac")
}

/// Extracts `classes.jar` from the given AAR file into `dest`.
fn extract_classes_jar(aar: &Path, dest: &Path) {
    let status = Command::new("unzip")
        .args(["-o", aar.to_str().unwrap(), "classes.jar", "-d"])
        .arg(dest.parent().unwrap())
        .status()
        .expect("unzip not found");
    if !status.success() {
        panic!("Failed to extract classes.jar from AAR");
    }
    // unzip puts it at <parent>/classes.jar; rename if dest differs.
    let extracted = dest.parent().unwrap().join("classes.jar");
    if extracted != *dest {
        fs::rename(&extracted, dest).unwrap();
    }
}
