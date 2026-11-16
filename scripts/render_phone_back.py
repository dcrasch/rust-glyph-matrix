#!/usr/bin/env python3
"""Render the Nothing Phone (2) back panel used by the app.

Source: glyphtones.firu.dev's phone2.svg
  https://raw.githubusercontent.com/firu11/nothing-glyphtones/main/static/phones/phone2.svg

The SVG contains the phone's 33 Glyph zones as individual <path> elements
(indexed 0..24 plus two "gradient" multi-LED strips: 3-18 = 16-segment
ring/arc, 25-32 = 8-segment USB line). The phone body itself is not in the
SVG (glyphtones draws it with CSS), so we add a body rect behind it.

Outputs (all RGBA, transparent background):
  assets/phone_back.png      base: body + every glyph at "off" brightness
  assets/glyph_<id>.png      lit overlay for each glyphId (white on transparent)

SVG path ids match the glyphId table for Nothing Phone (2) in
custom-nothing-glyph-tools (docs/4_First Composition/1b_glyphId Nothing Phone (2).md).
"""

import os
import subprocess
import xml.etree.ElementTree as ET

SVG = os.path.join(os.path.dirname(__file__), "..", "assets", "phone_back.svg")
OUT_DIR = os.path.join(os.path.dirname(__file__), "..", "assets")

BASE_OPACITY = "0.32"   # "off" brightness of the reference base image
LIT_OPACITY = "1.0"     # brightness of a lit zone overlay

# (glyphId, SVG path ids) in glyphId table order.
GLYPHS = [
    ("1",  "0"),        # Camera top
    ("2",  "1"),        # Camera bottom
    ("3",  "2"),        # Diagonal
    ("4",  "3-18"),     # Battery top right (4.1-4.16)
    ("5",  "19"),       # Battery top left
    ("6",  "20"),       # Battery top vertical (left)
    ("7",  "21"),       # Battery bottom left
    ("8",  "22"),       # Battery bottom right
    ("9",  "23"),       # Battery bottom vertical (right)
    ("10", "25-32"),    # USB Line (10.1-10.8)
    ("11", "24"),       # USB Dot
]

BODY_RECT = '    <rect x="3" y="3" width="176" height="376" rx="44" fill="#535150"/>\n'


def render(svg_text: str, name: str) -> str:
    """Write svg_text to a temp file, rasterize to PNG via rsvg-convert."""
    svg_path = os.path.join(OUT_DIR, f".{name}.svg")
    png_path = os.path.join(OUT_DIR, name)
    with open(svg_path, "w") as f:
        f.write(svg_text)
    subprocess.run(
        ["rsvg-convert", "-w", "728", "-h", "1528", "-o", png_path, svg_path],
        check=True,
    )
    os.remove(svg_path)
    return png_path


def set_attribute(element: ET.Element, attrs: dict) -> None:
    for k, v in attrs.items():
        element.set(k, v)


def build_svg(lit_ids: set[str]) -> str:
    """Return the SVG string with `lit_ids` bright and every other zone hidden.

    If lit_ids is None, all zones are shown at BASE_OPACITY (the base image).
    """
    tree = ET.parse(SVG)
    root = tree.getroot()

    paths = root.findall(".//{http://www.w3.org/2000/svg}path")
    stops = root.findall(".//{http://www.w3.org/2000/svg}stop")

    # Map gradient id -> list of its <stop> elements.
    grad_stops: dict[str, list] = {}
    for grad in root.findall(".//{http://www.w3.org/2000/svg}linearGradient"):
        first, last = (int(n) for n in grad.get("id").split("-")[1:])
        grad_stops[grad.get("id")] = [
            s for s in stops
            if first <= int(s.get("id", "grad-0").split("-")[1]) <= last
        ]

    if lit_ids is None:
        # Base image: every path/gradient stop at the off brightness.
        for p in paths:
            if p.get("opacity") is not None:
                p.set("opacity", BASE_OPACITY)
        for s in stops:
            s.set("stop-opacity", BASE_OPACITY)
    else:
        for p in paths:
            pid = p.get("id", "")
            if p.get("opacity") is not None:
                p.set("opacity", LIT_OPACITY if pid in lit_ids else "0")
            else:
                # Gradient path: control via its gradient's stops.
                grad_id = p.get("style", "").partition("url(#")[2].partition(")")[0]
                on = pid in lit_ids
                for s in grad_stops.get(grad_id, []):
                    s.set("stop-opacity", LIT_OPACITY if on else "0")
        # Non-gradient stops already handled per-path above; clear any
        # gradient whose path was not lit (defensive no-op handled above).
        for s in stops:
            # Stops already set only when their gradient path was processed.
            pass

    ET.indent(root)
    svg_text = ET.tostring(root, encoding="unicode")
    if lit_ids is not None:
        # Zone overlays must stay transparent outside the zone, or they cover the body and break hit-testing.
        return svg_text
    # Inject the body rectangle right after the opening <svg ...> tag.
    tag_end = svg_text.index(">") + 1
    return svg_text[:tag_end] + "\n" + BODY_RECT + svg_text[tag_end:]


def main() -> None:
    os.makedirs(OUT_DIR, exist_ok=True)

    # Base image
    render(build_svg(None), "phone_back.png")
    print("wrote assets/phone_back.png (728x1528)")

    # Per-zone lit overlays
    for gid, svg_ids in GLYPHS:
        out = render(build_svg({svg_ids}), f"glyph_{gid}.png")
        print(f"wrote {out}  <- glyphId {gid} (svg {svg_ids})")


if __name__ == "__main__":
    main()
