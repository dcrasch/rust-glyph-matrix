//! Nothing Phone (2) Glyph LED controller — egui front-end.
//!
//! On Android the app calls into the Nothing Glyph SDK via JNI.
//! On desktop it displays the same UI without touching any LEDs (useful for
//! layout iteration without deploying to the phone).

pub use eframe;

use eframe::egui::{self, Color32, RichText, Slider, Ui};

// ---------------------------------------------------------------------------
// Glyph zone definitions (Nothing Phone 2 / 22111)
// ---------------------------------------------------------------------------

/// Named LED zones on the Nothing Phone (2) back panel.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Zone {
    A1,
    A2,
    B1,
    /// The 16-segment ring (C1_1 … C1_16).
    C1Ring,
    /// The five small circles (C2 … C6).
    CSmall,
    E1,
    /// The eight-segment progress strip (D1_1 … D1_8).
    D1Strip,
}

impl Zone {
    const ALL: &'static [Zone] = &[
        Zone::A1,
        Zone::A2,
        Zone::B1,
        Zone::C1Ring,
        Zone::CSmall,
        Zone::E1,
        Zone::D1Strip,
    ];

    fn label(self) -> &'static str {
        match self {
            Zone::A1 => "A1  (top-left rect)",
            Zone::A2 => "A2  (top-right rect)",
            Zone::B1 => "B1  (camera ring outer)",
            Zone::C1Ring => "C1  (camera ring 16-seg)",
            Zone::CSmall => "C2–C6  (small dots)",
            Zone::E1 => "E1  (diagonal bar)",
            Zone::D1Strip => "D1  (progress strip)",
        }
    }
}

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

pub struct GlyphApp {
    /// Which zones are currently switched on.
    zones: [bool; 7],
    /// Progress value shown on the D1 / C1 strip (0–100).
    progress: i32,
    /// Whether the breathing animation is active.
    animate: bool,
    /// Status line shown at the bottom of the UI.
    status: String,
    /// Platform-specific Glyph handle (None on desktop).
    #[cfg(target_os = "android")]
    glyph: glyph::GlyphHandle,
}

impl GlyphApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        #[cfg(target_os = "android")]
        let glyph = glyph::GlyphHandle::init();

        Self {
            zones: [false; 7],
            progress: 0,
            animate: false,
            status: "Ready".into(),
            #[cfg(target_os = "android")]
            glyph,
        }
    }

    fn apply(&mut self) {
        #[cfg(target_os = "android")]
        {
            let result = if self.animate {
                self.glyph.animate(&self.zones)
            } else {
                self.glyph.toggle(&self.zones)
            };
            self.status = match result {
                Ok(()) => "OK".into(),
                Err(e) => format!("Error: {e}"),
            };
        }
        #[cfg(not(target_os = "android"))]
        {
            self.status = format!(
                "Desktop: zones {:?} animate={}",
                self.zones, self.animate
            );
        }
    }

    fn apply_progress(&mut self) {
        #[cfg(target_os = "android")]
        {
            let result = self.glyph.set_progress(&self.zones, self.progress);
            self.status = match result {
                Ok(()) => format!("Progress {}%", self.progress),
                Err(e) => format!("Error: {e}"),
            };
        }
        #[cfg(not(target_os = "android"))]
        {
            self.status = format!("Desktop: progress {}%", self.progress);
        }
    }

    fn turn_off(&mut self) {
        self.zones = [false; 7];
        self.animate = false;

        #[cfg(target_os = "android")]
        {
            let result = self.glyph.turn_off();
            self.status = match result {
                Ok(()) => "All off".into(),
                Err(e) => format!("Error: {e}"),
            };
        }
        #[cfg(not(target_os = "android"))]
        {
            self.status = "Desktop: all off".into();
        }
    }
}

impl eframe::App for GlyphApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading(RichText::new("Nothing Phone (2)  •  Glyph Control").size(22.0));
            ui.add_space(12.0);

            zone_grid(ui, &mut self.zones);

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(6.0);

            ui.horizontal(|ui| {
                ui.label("Progress (D1 strip / C1 ring):");
                if ui
                    .add(Slider::new(&mut self.progress, 0..=100).suffix("%"))
                    .changed()
                {
                    self.apply_progress();
                }
            });

            ui.add_space(6.0);
            ui.checkbox(&mut self.animate, "Breathing animation");

            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui.button(RichText::new("Apply").size(18.0)).clicked() {
                    self.apply();
                }
                ui.add_space(8.0);
                if ui
                    .button(RichText::new("All Off").size(18.0).color(Color32::RED))
                    .clicked()
                {
                    self.turn_off();
                }
            });

            ui.add_space(12.0);
            ui.separator();
            ui.label(RichText::new(&self.status).color(Color32::LIGHT_BLUE));
        });
    }
}

/// Renders the zone toggle grid.
fn zone_grid(ui: &mut Ui, zones: &mut [bool; 7]) {
    egui::Grid::new("zones")
        .num_columns(2)
        .spacing([12.0, 8.0])
        .show(ui, |ui| {
            for (i, zone) in Zone::ALL.iter().enumerate() {
                let active = zones[i];
                let color = if active { Color32::LIGHT_GREEN } else { Color32::GRAY };
                if ui
                    .button(RichText::new(zone.label()).color(color).size(16.0))
                    .clicked()
                {
                    zones[i] = !zones[i];
                }
                if i % 2 == 1 {
                    ui.end_row();
                }
            }
        });
}

// ---------------------------------------------------------------------------
// Desktop entry point
// ---------------------------------------------------------------------------

#[cfg(not(target_os = "android"))]
pub fn run() -> eframe::Result {
    eframe::run_native(
        "Glyph Control",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([420.0, 680.0])
                .with_title("Glyph Control"),
            renderer: eframe::Renderer::Glow,
            ..Default::default()
        },
        Box::new(|cc| Ok(Box::new(GlyphApp::new(cc)))),
    )
}

// ---------------------------------------------------------------------------
// Android entry point
// ---------------------------------------------------------------------------

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub fn android_main(app: winit::platform::android::activity::AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
            .with_tag("rust-glyph-matrix"),
    );

    let mut options = eframe::NativeOptions {
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    options.android_app = Some(app);

    if let Err(err) = eframe::run_native(
        "Glyph Control",
        options,
        Box::new(|cc| Ok(Box::new(GlyphApp::new(cc)))),
    ) {
        log::error!("eframe error: {err}");
    }
}

// ---------------------------------------------------------------------------
// Android Glyph JNI bridge
// ---------------------------------------------------------------------------

#[cfg(target_os = "android")]
mod glyph {
    use jni::objects::{JObject, JValue};
    use jni::refs::Global;
    use ndk_context::android_context;

    pub struct GlyphHandle {
        bridge: Global<JObject<'static>>,
    }

    // SAFETY: the Global reference is valid across threads; JNI access is
    // serialized by attaching each calling thread before use.
    unsafe impl Send for GlyphHandle {}
    unsafe impl Sync for GlyphHandle {}

    impl GlyphHandle {
        pub fn init() -> Self {
            let ctx = android_context();
            // SAFETY: the JavaVM pointer is valid for the lifetime of the process.
            let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) };
            let bridge_ref = vm
                .attach_current_thread(|env| {
                    let activity =
                        unsafe { JObject::from_raw(env, ctx.context().cast()) };

                    // Load our classes.dex (containing GlyphBridge + Glyph SDK)
                    // via DexClassLoader, then call GlyphBridge.init via
                    // reflection so the class is resolved from our loader.
                    let bridge = load_dex_and_init_bridge(env, &activity)?;
                    env.new_global_ref(bridge)
                })
                .expect("GlyphBridge.init failed");

            Self { bridge: bridge_ref }
        }

        pub fn toggle(&self, zones: &[bool; 7]) -> Result<(), String> {
            let mask = zones_to_mask(zones);
            self.call_int_void(jni::jni_str!("toggle"), mask)
        }

        pub fn animate(&self, zones: &[bool; 7]) -> Result<(), String> {
            let mask = zones_to_mask(zones);
            self.call_int_void(jni::jni_str!("animate"), mask)
        }

        pub fn set_progress(&self, zones: &[bool; 7], progress: i32) -> Result<(), String> {
            let mask = zones_to_mask(zones);
            self.with_vm(|env| {
                env.call_method(
                    self.bridge.as_obj(),
                    jni::jni_str!("setProgress"),
                    jni::jni_sig!((int, int) -> void),
                    &[JValue::Int(mask), JValue::Int(progress)],
                )?;
                Ok(())
            })
        }

        pub fn turn_off(&self) -> Result<(), String> {
            self.with_vm(|env| {
                env.call_method(
                    self.bridge.as_obj(),
                    jni::jni_str!("turnOff"),
                    jni::jni_sig!(() -> void),
                    &[],
                )?;
                Ok(())
            })
        }

        fn call_int_void(
            &self,
            name: &jni::strings::JNIStr,
            arg: i32,
        ) -> Result<(), String> {
            self.with_vm(|env| {
                env.call_method(
                    self.bridge.as_obj(),
                    name,
                    jni::jni_sig!((int) -> void),
                    &[JValue::Int(arg)],
                )?;
                Ok(())
            })
        }

        fn with_vm<F>(&self, f: F) -> Result<(), String>
        where
            F: for<'e> FnOnce(&mut jni::Env<'e>) -> Result<(), jni::errors::Error>,
        {
            let ctx = android_context();
            let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) };
            vm.attach_current_thread(f).map_err(|e| e.to_string())
        }
    }

    impl Drop for GlyphHandle {
        fn drop(&mut self) {
            let _ = self.with_vm(|env| {
                env.call_method(
                    self.bridge.as_obj(),
                    jni::jni_str!("destroy"),
                    jni::jni_sig!(() -> void),
                    &[],
                )?;
                Ok(())
            });
        }
    }

    /// Loads `classes.dex` from the app's own APK via `DexClassLoader`, then
    /// calls `GlyphBridge.init(context)` via reflection so the class is
    /// resolved through our custom loader (not the system class loader, which
    /// does not include our dex because NativeActivity sets hasCode=false).
    fn load_dex_and_init_bridge<'e>(
        env: &mut jni::Env<'e>,
        context: &JObject<'_>,
    ) -> Result<JObject<'e>, jni::errors::Error> {
        // context.getPackageCodePath() → "/data/app/.../base.apk"
        let apk_path = env
            .call_method(
                context,
                jni::jni_str!("getPackageCodePath"),
                jni::jni_sig!(() -> java.lang.String),
                &[],
            )?
            .l()?;

        // context.getCacheDir().getAbsolutePath() → writable dex opt dir
        let cache_dir_file = env
            .call_method(
                context,
                jni::jni_str!("getCacheDir"),
                jni::jni_sig!(() -> java.io.File),
                &[],
            )?
            .l()?;
        let cache_path = env
            .call_method(
                &cache_dir_file,
                jni::jni_str!("getAbsolutePath"),
                jni::jni_sig!(() -> java.lang.String),
                &[],
            )?
            .l()?;

        // parent = context.getClassLoader()
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
            jni::jni_sig!(
                (java.lang.String, java.lang.String, java.lang.String, java.lang.ClassLoader)
            ),
            &[
                JValue::Object(&apk_path),
                JValue::Object(&cache_path),
                JValue::Object(&null),
                JValue::Object(&parent_loader),
            ],
        )?;
        log::info!("DexClassLoader created from APK");

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
        log::info!("GlyphBridge class loaded");

        // Use reflection: bridge_cls.getMethod("init", Context.class).invoke(null, context)
        // Easier: call the static method via env using the loaded class object as JClass.
        // JNI lets us call static methods on a jclass obtained from loadClass.
        // SAFETY: bridge_cls is a java.lang.Class object returned by loadClass.
        let bridge_jclass: jni::objects::JClass<'_> = unsafe {
            jni::objects::JClass::from_raw(env, bridge_cls.as_raw())
        };
        let bridge = env
            .call_static_method(
                &bridge_jclass,
                jni::jni_str!("init"),
                jni::jni_sig!(
                    type_map = { GlyphBridge => "dev.rustglyphmatrix.GlyphBridge" },
                    (android.content.Context) -> GlyphBridge
                ),
                &[JValue::Object(context)],
            )?
            .l()?;
        log::info!("GlyphBridge.init succeeded");
        Ok(bridge)
    }

    /// Packs 7 zone booleans into an integer bitmask sent to GlyphBridge.
    fn zones_to_mask(zones: &[bool; 7]) -> i32 {
        let mut mask: i32 = 0;
        for (i, &on) in zones.iter().enumerate() {
            if on {
                mask |= 1 << i;
            }
        }
        mask
    }
}
