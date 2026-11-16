#!/usr/bin/env python3
"""Replace or append classes.dex in an existing APK (zip file).

Usage: inject_dex.py <dex> <apk>

Rewrites the APK, replacing any existing classes.dex entry with the given
file so zipalign sees no duplicates.
"""
import sys
import zipfile
import os
import tempfile

dex_path, apk_path = sys.argv[1], sys.argv[2]

with open(dex_path, "rb") as f:
    dex_data = f.read()

apk_dir = os.path.dirname(os.path.abspath(apk_path))
fd, tmp_path = tempfile.mkstemp(dir=apk_dir, suffix=".apk")
try:
    with os.fdopen(fd, "wb") as tmp_file:
        with zipfile.ZipFile(apk_path, "r") as src, \
             zipfile.ZipFile(tmp_file, "w", allowZip64=True) as dst:
            for item in src.infolist():
                if item.filename == "classes.dex":
                    continue  # will be replaced below
                dst.writestr(item, src.read(item.filename),
                             compress_type=item.compress_type)
            # Add our dex uncompressed (required for zipalign / ART mmap).
            dst.writestr("classes.dex", dex_data, compress_type=zipfile.ZIP_STORED)
    os.replace(tmp_path, apk_path)
except Exception:
    os.unlink(tmp_path)
    raise

print(f"Injected {dex_path} into {apk_path}")
