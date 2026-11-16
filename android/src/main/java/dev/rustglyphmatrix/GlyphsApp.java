package dev.rustglyphmatrix;

import android.app.Application;
import android.util.Log;

/**
 * Custom Application subclass.
 *
 * Its only purpose is to give the manifest an android:name so that Android
 * loads classes.dex (and therefore the Glyph SDK + GlyphBridge) before
 * NativeActivity starts. Without a named Application, hasCode=true alone is
 * not sufficient on some Android versions.
 */
public class GlyphsApp extends Application {
    private static final String TAG = "GlyphsApp";

    @Override
    public void onCreate() {
        super.onCreate();
        Log.i(TAG, "GlyphsApp started");
    }
}
