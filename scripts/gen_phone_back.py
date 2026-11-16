#!/usr/bin/env python3
"""Generate the phone-back glyph diagram used by the app (assets/phone_back.png).

The geometry here is mirrored 1:1 in `src/lib.rs` (the zone hit-testing and the
"lit" overlay), so any change to these coordinates must be reflected there too.

Source image coordinates: 360 x 720.
"""

import os

from PIL import Image, ImageDraw, ImageFont

W, H = 360, 720

# Colors
BODY_FILL = (20, 21, 24, 255)          # near-black phone back
BODY_OUTLINE = (51, 54, 61, 255)
ZONE_FILL = (64, 69, 79, 255)          # dim "off" glyph colour
ZONE_OUTLINE = (88, 94, 105, 255)
CAMERA = (10, 11, 13, 255)
LENS = (30, 33, 40, 255)
LABEL = (154, 160, 172, 255)

img = Image.new("RGBA", (W, H), (0, 0, 0, 0))
d = ImageDraw.Draw(img)

# --- Phone body ------------------------------------------------------------
d.rounded_rectangle(
    (28, 55, 332, 665), radius=48, fill=BODY_FILL, outline=BODY_OUTLINE, width=2
)

# --- Glyph zones (drawn in their "off" state) -----------------------------
# A1 (top-left), A2 (top-right)
d.rounded_rectangle((50, 95, 132, 150), radius=16, fill=ZONE_FILL, outline=ZONE_OUTLINE, width=2)
d.rounded_rectangle((228, 95, 310, 150), radius=16, fill=ZONE_FILL, outline=ZONE_OUTLINE, width=2)

# Camera (dark disc) + B1 outer ring + C1 inner ring + lens
cx, cy = 180, 260
d.ellipse((cx - 52, cy - 52, cx + 52, cy + 52), fill=CAMERA)
# B1 ring (band 37..47)
d.ellipse((cx - 47, cy - 47, cx + 47, cy + 47), fill=ZONE_FILL, outline=ZONE_OUTLINE, width=2)
d.ellipse((cx - 37, cy - 37, cx + 37, cy + 37), fill=CAMERA)
# C1 ring (band 20..28)
d.ellipse((cx - 28, cy - 28, cx + 28, cy + 28), fill=ZONE_FILL, outline=ZONE_OUTLINE, width=2)
d.ellipse((cx - 20, cy - 20, cx + 20, cy + 20), fill=CAMERA)
# Lens
d.ellipse((cx - 12, cy - 12, cx + 12, cy + 12), fill=LENS)

# C small dots (C2..C6)
for x in (92, 136, 180, 224, 268):
    d.ellipse((x - 8, 352 - 8, x + 8, 352 + 8), fill=ZONE_FILL, outline=ZONE_OUTLINE, width=2)

# E1 diagonal bar with rounded caps
d.line((86, 450, 168, 512), fill=ZONE_FILL, width=14)
d.ellipse((86 - 7, 450 - 7, 86 + 7, 450 + 7), fill=ZONE_FILL, outline=ZONE_OUTLINE, width=2)
d.ellipse((168 - 7, 512 - 7, 168 + 7, 512 + 7), fill=ZONE_FILL, outline=ZONE_OUTLINE, width=2)

# D1 progress strip with 8 segment dividers
d.rounded_rectangle((50, 560, 310, 616), radius=14, fill=ZONE_FILL, outline=ZONE_OUTLINE, width=2)
for i in range(1, 8):
    x = 50 + (260 / 8) * i
    d.line((x, 563, x, 613), fill=(50, 54, 63, 255), width=2)

# --- Labels ----------------------------------------------------------------
try:
    font = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", 22)
except OSError:
    font = ImageFont.load_default(size=22)

labels = [
    ("A1", 90, 174),
    ("A2", 270, 174),
    ("B1", 98, 250),
    ("C1", 180, 322),
    ("C2–C6", 180, 388),
    ("E1", 198, 478),
    ("D1", 180, 640),
]
for text, x, y in labels:
    d.text((x, y), text, font=font, fill=LABEL, anchor="mm")

os.makedirs("assets", exist_ok=True)
img.save("assets/phone_back.png")
print(f"wrote assets/phone_back.png ({W}x{H})")
