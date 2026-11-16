package dev.rustglyphmatrix;

import android.content.ComponentName;
import android.content.Context;
import android.util.Log;

import com.nothing.ketchum.Common;
import com.nothing.ketchum.Glyph;
import com.nothing.ketchum.GlyphException; // thrown by openSession/closeSession/displayProgress
import com.nothing.ketchum.GlyphFrame;
import com.nothing.ketchum.GlyphManager;

/**
 * Thin wrapper around the Nothing Glyph SDK that exposes a simple integer-mask
 * API callable from Rust via JNI.
 *
 * Zone bitmask (matches Zone::ALL order in Rust):
 *   bit 0 — A1 / A2  (Zone A, both rects — SDK groups them)
 *   bit 1 — A2       (grouped with A1 above in SDK)
 *   bit 2 — B1       (camera ring outer)
 *   bit 3 — C1 ring  (16-segment ring)
 *   bit 4 — C small  (C2–C6, same buildChannelC call)
 *   bit 5 — E1       (diagonal bar)
 *   bit 6 — D1 strip (progress strip)
 */
public class GlyphBridge {
    private static final String TAG = "GlyphBridge";

    private final GlyphManager mGM;
    private boolean mSessionOpen = false;

    private GlyphBridge(GlyphManager gm) {
        mGM = gm;
    }

    public static GlyphBridge init(Context context) {
        GlyphManager gm = GlyphManager.getInstance(context);
        GlyphBridge bridge = new GlyphBridge(gm);

        gm.init(new GlyphManager.Callback() {
            @Override
            public void onServiceConnected(ComponentName name) {
                try {
                    if (Common.is22111()) gm.register(Glyph.DEVICE_22111);
                    else if (Common.is23111()) gm.register(Glyph.DEVICE_23111);
                    else if (Common.is23113()) gm.register(Glyph.DEVICE_23113);
                    else gm.register(Glyph.DEVICE_22111); // fallback
                    gm.openSession();
                    bridge.mSessionOpen = true;
                    Log.i(TAG, "Glyph session opened");
                } catch (GlyphException e) {
                    Log.e(TAG, "openSession failed: " + e.getMessage());
                }
            }

            @Override
            public void onServiceDisconnected(ComponentName name) {
                bridge.mSessionOpen = false;
                Log.i(TAG, "Glyph service disconnected");
            }
        });

        return bridge;
    }

    // -----------------------------------------------------------------------
    // Public API (called from Rust via JNI)
    // -----------------------------------------------------------------------

    /** Toggle the zones described by the bitmask on or off. */
    public void toggle(int mask) {
        if (!mSessionOpen) return;
        GlyphFrame frame = buildFrame(mask);
        if (frame != null) mGM.toggle(frame);
    }

    /** Start a breathing animation on the zones described by the bitmask. */
    public void animate(int mask) {
        if (!mSessionOpen) return;
        GlyphFrame.Builder b = mGM.getGlyphFrameBuilder();
        b = applyZones(b, mask);
        GlyphFrame frame = b.buildPeriod(1500).buildCycles(0).buildInterval(50).build();
        mGM.animate(frame);
    }

    /**
     * Display a progress value (0–100) on the D1/C1 strip.
     * The mask selects which other zones are on alongside the progress bar.
     */
    public void setProgress(int mask, int progress) {
        if (!mSessionOpen) return;
        try {
            GlyphFrame frame = buildFrame(mask);
            if (frame != null) mGM.displayProgress(frame, progress);
        } catch (GlyphException e) {
            Log.e(TAG, "setProgress: " + e.getMessage());
        }
    }

    /** Turn off all Glyph LEDs immediately. */
    public void turnOff() {
        if (!mSessionOpen) return;
        mGM.turnOff();
    }

    /** Release the session and unbind from the Glyph service. */
    public void destroy() {
        try {
            if (mSessionOpen) mGM.closeSession();
        } catch (GlyphException e) {
            Log.e(TAG, "closeSession: " + e.getMessage());
        }
        mGM.unInit();
        mSessionOpen = false;
    }

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /** Build a GlyphFrame from the zone bitmask. Returns null if no zone selected. */
    private GlyphFrame buildFrame(int mask) {
        if (mask == 0) return null;
        GlyphFrame.Builder b = mGM.getGlyphFrameBuilder();
        b = applyZones(b, mask);
        return b.build();
    }

    /**
     * Apply zone bits to the frame builder.
     *
     * Bit mapping (matches Zone::ALL in Rust):
     *   0 → A (bit covers A1; SDK buildChannelA activates both A1 and A2)
     *   1 → A (A2, same SDK call — activating bit 1 alone also calls buildChannelA)
     *   2 → B
     *   3 → C (C1 ring; buildChannelC activates C1_1..C1_16 + C2..C6)
     *   4 → C (C small; same buildChannelC)
     *   5 → E
     *   6 → D (D1 strip)
     */
    private GlyphFrame.Builder applyZones(GlyphFrame.Builder b, int mask) {
        if ((mask & 0b0000011) != 0) b = b.buildChannelA();
        if ((mask & 0b0000100) != 0) b = b.buildChannelB();
        if ((mask & 0b0011000) != 0) b = b.buildChannelC();
        if ((mask & 0b0100000) != 0) b = b.buildChannelE();
        if ((mask & 0b1000000) != 0) b = b.buildChannelD();
        return b;
    }
}
