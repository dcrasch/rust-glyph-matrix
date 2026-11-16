//! Safe wrapper around the Nothing Glyph SDK on Android.
//!
//! The SDK is driven through `GlyphBridge` (see `android/.../GlyphBridge.java`),
//! which is packaged in `classes.dex`, loaded from the APK at runtime via
//! `DexClassLoader`, and called over JNI. Requires `ndk-context` to be
//! initialized, which the Android activity glue does before `main` runs.
//!
//! Only Nothing devices running Android 14+ expose the Glyph service, and only
//! the foreground app can drive the LEDs. Calls made before the service has
//! connected are silently ignored; see [`GlyphController::new`].
//!
//! # Example
//!
//! ```ignore
//! use rust_glyph_matrix::glyph::GlyphController;
//!
//! let glyphs = GlyphController::new()?;
//! let mut flags = [false; rust_glyph_matrix::GLYPH_COUNT];
//! flags[0] = true; // glyphId 1, camera top
//! glyphs.show(&flags)?;
//! ```

use std::fmt;

use jni::objects::{JObject, JValue};
use jni::refs::Global;
use jni::{Env, JavaVM};
use ndk_context::android_context;

use crate::GLYPH_COUNT;

/// Error returned by [`GlyphController`] methods.
///
/// Wraps the JNI failure message. The variant is opaque so the underlying
/// `jni` crate can change without breaking callers.
#[derive(Debug)]
pub struct GlyphError(String);

impl fmt::Display for GlyphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for GlyphError {}

/// Controls the Nothing Phone (2) Glyph LEDs.
///
/// Glyphs are addressed by index: `glyphs[i]` selects glyphId `i + 1`, so
/// index 0 is the camera top and index 10 is the USB dot. The mapping is
/// documented on the Java bridge class `GlyphBridge`.
///
/// Dropping the controller closes the Glyph session and unbinds from the
/// service. Create one controller per app and reuse it; creating several
/// holds several Glyph sessions open at once.
pub struct GlyphController {
    bridge: Global<JObject<'static>>,
}

// SAFETY: the global reference is valid on any thread, and every JNI call
// attaches the calling thread first.
unsafe impl Send for GlyphController {}
unsafe impl Sync for GlyphController {}

impl GlyphController {
    /// Loads the bridge from the APK and starts connecting to the Glyph service.
    ///
    /// This returns before the session is open. The SDK connects
    /// asynchronously, so until it does, `show`, `animate`, `set_progress`, and
    /// `turn_off` succeed but do nothing. Wait a moment after construction
    /// before the first call, or check `logcat` for `Glyph session opened`
    /// (tag `GlyphBridge`).
    ///
    /// Returns an error if the `GlyphBridge` class cannot be loaded from the
    /// APK, for example when `classes.dex` was not injected by `just build`.
    pub fn new() -> Result<Self, GlyphError> {
        let ctx = android_context();
        let bridge = with_env(|env| {
            // SAFETY: the context pointer is valid for the lifetime of the activity.
            let activity = unsafe { JObject::from_raw(env, ctx.context().cast()) };
            let bridge = load_bridge(env, &activity)?;
            env.new_global_ref(bridge)
        })?;
        Ok(Self { bridge })
    }

    /// Lights the selected glyphs with the SDK's `toggle` call.
    ///
    /// When no glyph is selected, the bridge sends nothing and the LEDs keep
    /// their previous state. Use [`turn_off`](Self::turn_off) to clear them.
    pub fn show(&self, glyphs: &[bool; GLYPH_COUNT]) -> Result<(), GlyphError> {
        let mask = glyphs_to_mask(glyphs);
        self.call(|env, bridge| {
            env.call_method(
                bridge,
                jni::jni_str!("toggle"),
                jni::jni_sig!((int) -> void),
                &[JValue::Int(mask)],
            )
            .map(|_| ())
        })
    }

    /// Starts a breathing animation on the selected glyphs.
    ///
    /// The bridge uses a 1500 ms period with cycles set to 0 and a 50 ms
    /// interval. Call [`show`](Self::show) or [`turn_off`](Self::turn_off) to
    /// stop it. With no glyph selected, the bridge does nothing.
    pub fn animate(&self, glyphs: &[bool; GLYPH_COUNT]) -> Result<(), GlyphError> {
        let mask = glyphs_to_mask(glyphs);
        self.call(|env, bridge| {
            env.call_method(
                bridge,
                jni::jni_str!("animate"),
                jni::jni_sig!((int) -> void),
                &[JValue::Int(mask)],
            )
            .map(|_| ())
        })
    }

    /// Shows `progress` as a fill on glyph 4 (the 16-segment camera ring) and
    /// glyph 10 (the 8-segment USB strip), alongside any other selected glyphs.
    ///
    /// `progress` is clamped to 0–100 on the Java side. Glyphs 4 and 10 ignore
    /// their bits in `glyphs`, since the progress fill drives them. If nothing
    /// else is selected and the fill is empty, the LEDs are turned off.
    pub fn set_progress(
        &self,
        glyphs: &[bool; GLYPH_COUNT],
        progress: i32,
    ) -> Result<(), GlyphError> {
        let mask = glyphs_to_mask(glyphs);
        self.call(|env, bridge| {
            env.call_method(
                bridge,
                jni::jni_str!("setProgress"),
                jni::jni_sig!((int, int) -> void),
                &[JValue::Int(mask), JValue::Int(progress)],
            )
            .map(|_| ())
        })
    }

    /// Turns off every glyph immediately.
    pub fn turn_off(&self) -> Result<(), GlyphError> {
        self.call(|env, bridge| {
            env.call_method(
                bridge,
                jni::jni_str!("turnOff"),
                jni::jni_sig!(() -> void),
                &[],
            )
            .map(|_| ())
        })
    }

    /// Attaches the current thread and runs `f` against the bridge object.
    fn call<F>(&self, f: F) -> Result<(), GlyphError>
    where
        F: for<'e> FnOnce(&mut Env<'e>, &JObject<'static>) -> Result<(), jni::errors::Error>,
    {
        let bridge = self.bridge.as_obj();
        with_env(|env| f(env, bridge))
    }
}

impl Drop for GlyphController {
    /// Closes the Glyph session and unbinds from the service. Errors are ignored.
    fn drop(&mut self) {
        let _ = self.call(|env, bridge| {
            env.call_method(
                bridge,
                jni::jni_str!("destroy"),
                jni::jni_sig!(() -> void),
                &[],
            )
            .map(|_| ())
        });
    }
}

/// Attaches the current thread to the JVM and runs `f`.
fn with_env<T, F>(f: F) -> Result<T, GlyphError>
where
    F: for<'e> FnOnce(&mut Env<'e>) -> Result<T, jni::errors::Error>,
{
    // SAFETY: the JavaVM pointer is valid for the lifetime of the process.
    let vm = unsafe { JavaVM::from_raw(android_context().vm().cast()) };
    vm.attach_current_thread(f)
        .map_err(|e| GlyphError(e.to_string()))
}

/// Loads `classes.dex` from the app's APK with `DexClassLoader` and calls
/// `GlyphBridge.init(context)` through that loader. The system class loader
/// cannot see the dex because `NativeActivity` sets `hasCode=false`.
fn load_bridge<'e>(
    env: &mut Env<'e>,
    context: &JObject<'_>,
) -> Result<JObject<'e>, jni::errors::Error> {
    // e.g. "/data/app/.../base.apk"
    let apk_path = env
        .call_method(
            context,
            jni::jni_str!("getPackageCodePath"),
            jni::jni_sig!(() -> java.lang.String),
            &[],
        )?
        .l()?;

    // Writable directory for the optimized dex output.
    let cache_dir = env
        .call_method(
            context,
            jni::jni_str!("getCacheDir"),
            jni::jni_sig!(() -> java.io.File),
            &[],
        )?
        .l()?;
    let cache_path = env
        .call_method(
            &cache_dir,
            jni::jni_str!("getAbsolutePath"),
            jni::jni_sig!(() -> java.lang.String),
            &[],
        )?
        .l()?;

    let parent_loader = env
        .call_method(
            context,
            jni::jni_str!("getClassLoader"),
            jni::jni_sig!(() -> java.lang.ClassLoader),
            &[],
        )?
        .l()?;

    // new DexClassLoader(apkPath, cacheDir, null, parent)
    let dcl_cls = env.find_class(jni::jni_str!("dalvik/system/DexClassLoader"))?;
    let null = JObject::null();
    let loader = env.new_object(
        &dcl_cls,
        jni::jni_sig!((
            java.lang.String,
            java.lang.String,
            java.lang.String,
            java.lang.ClassLoader
        )),
        &[
            JValue::Object(&apk_path),
            JValue::Object(&cache_path),
            JValue::Object(&null),
            JValue::Object(&parent_loader),
        ],
    )?;

    // loader.loadClass("dev.rustglyphmatrix.GlyphBridge")
    let class_name = env.new_string("dev.rustglyphmatrix.GlyphBridge")?;
    let bridge_cls = env
        .call_method(
            &loader,
            jni::jni_str!("loadClass"),
            jni::jni_sig!((java.lang.String) -> java.lang.Class),
            &[JValue::Object(&class_name)],
        )?
        .l()?;

    // SAFETY: bridge_cls is a java.lang.Class returned by loadClass.
    let bridge_jclass: jni::objects::JClass<'_> =
        unsafe { jni::objects::JClass::from_raw(env, bridge_cls.as_raw()) };
    env.call_static_method(
        &bridge_jclass,
        jni::jni_str!("init"),
        jni::jni_sig!(
            type_map = { GlyphBridge => "dev.rustglyphmatrix.GlyphBridge" },
            (android.content.Context) -> GlyphBridge
        ),
        &[JValue::Object(context)],
    )?
    .l()
}

/// Packs the glyph flags into a bitmask: bit i is glyphId i + 1.
fn glyphs_to_mask(glyphs: &[bool; GLYPH_COUNT]) -> i32 {
    let mut mask: i32 = 0;
    for (i, &on) in glyphs.iter().enumerate() {
        if on {
            mask |= 1 << i;
        }
    }
    mask
}
