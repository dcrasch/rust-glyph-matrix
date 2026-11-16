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
 * Mask bit i (0-based) selects glyphId i + 1 from the Nothing Phone (2) glyphId table:
 *   bit 0  — 1  Camera top
 *   bit 1  — 2  Camera bottom
 *   bit 2  — 3  Diagonal
 *   bit 3  — 4  Battery top right (4.1–4.16)
 *   bit 4  — 5  Battery top left
 *   bit 5  — 6  Battery top vertical (left)
 *   bit 6  — 7  Battery bottom left
 *   bit 7  — 8  Battery bottom right
 *   bit 8  — 9  Battery bottom vertical (right)
 *   bit 9  — 10 USB line (10.1–10.8)
 *   bit 10 — 11 USB dot
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

    /** Toggle the glyphs described by the bitmask on or off. */
    public void toggle(int mask) {
        if (!mSessionOpen) return;
        GlyphFrame frame = buildFrame(mask);
        if (frame != null) mGM.toggle(frame);
    }

    /** Start a breathing animation on the glyphs described by the bitmask. */
    public void animate(int mask) {
        if (!mSessionOpen) return;
        GlyphFrame.Builder b = mGM.getGlyphFrameBuilder();
        b = applyGlyphs(b, mask);
        GlyphFrame frame = b.buildPeriod(1500).buildCycles(0).buildInterval(50).build();
        mGM.animate(frame);
    }

    /**
     * Light the first segments of glyph 4 (16 segments) and glyph 10 (8 segments)
     * in proportion to progress (0–100). Other glyphs in the mask are lit as usual.
     */
    public void setProgress(int mask, int progress) {
        if (!mSessionOpen) return;
        int p = Math.max(0, Math.min(100, progress));
        int ring = (p * GLYPH_CHANNELS[3].length + 50) / 100;
        int strip = (p * GLYPH_CHANNELS[9].length + 50) / 100;

        GlyphFrame.Builder b = applyGlyphs(mGM.getGlyphFrameBuilder(), mask & ~PROGRESS_MASK);
        b = fillSegments(b, GLYPH_CHANNELS[3], ring);
        b = fillSegments(b, GLYPH_CHANNELS[9], strip);

        if ((mask & ~PROGRESS_MASK) == 0 && ring == 0 && strip == 0) {
            mGM.turnOff();
        } else {
            mGM.toggle(b.build());
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

    /** Build a GlyphFrame from the glyph bitmask. Returns null if no glyph selected. */
    private GlyphFrame buildFrame(int mask) {
        if (mask == 0) return null;
        GlyphFrame.Builder b = mGM.getGlyphFrameBuilder();
        b = applyGlyphs(b, mask);
        return b.build();
    }

    /**
     * SDK channel indexes for each glyphId, indexed by bit. The SDK's channel
     * names don't match the physical layout for glyphs 3 and 11: channel 24
     * (named E1) lights the USB dot, and channel 2 (named B1) lights the diagonal.
     */
    private static final int[][] GLYPH_CHANNELS = {
        {0},                                              // 1  Camera top
        {1},                                              // 2  Camera bottom
        {2},                                              // 3  Diagonal
        {3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18}, // 4 Battery top right
        {19},                                             // 5  Battery top left
        {20},                                             // 6  Battery top vertical (left)
        {21},                                             // 7  Battery bottom left
        {22},                                             // 8  Battery bottom right
        {23},                                             // 9  Battery bottom vertical (right)
        {25, 26, 27, 28, 29, 30, 31, 32},                 // 10 USB line
        {24},                                             // 11 USB dot
    };

    private static final int MAX_BRIGHTNESS = 4096;

    /** Bits for glyph 4 (ring) and glyph 10 (strip), which the progress slider drives. */
    private static final int PROGRESS_MASK = (1 << 3) | (1 << 9);

    /** Apply glyph bits to the frame builder, lighting each selected glyph's channels. */
    private GlyphFrame.Builder applyGlyphs(GlyphFrame.Builder b, int mask) {
        for (int bit = 0; bit < GLYPH_CHANNELS.length; bit++) {
            if ((mask & (1 << bit)) == 0) continue;
            for (int channel : GLYPH_CHANNELS[bit]) {
                b = b.buildChannel(channel, MAX_BRIGHTNESS);
            }
        }
        return b;
    }

    /** Light the first `count` channels of a segmented glyph. */
    private GlyphFrame.Builder fillSegments(GlyphFrame.Builder b, int[] channels, int count) {
        for (int i = 0; i < count && i < channels.length; i++) {
            b = b.buildChannel(channels[i], MAX_BRIGHTNESS);
        }
        return b;
    }
}
