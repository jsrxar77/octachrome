#!/usr/bin/env python3
"""Recompresses PNGs (the bench writes them uncompressed) so that pictures kept in the repository are small.
Usage: python3 scripts/shrink-png.py <in-dir> <out-dir>"""
import pathlib, struct, sys, zlib

def chunks(data):
    i = 8
    while i < len(data):
        n, = struct.unpack(">I", data[i:i + 4])
        yield data[i + 4:i + 8], data[i + 8:i + 8 + n]
        i += 12 + n

def chunk(kind, body):
    return struct.pack(">I", len(body)) + kind + body + struct.pack(">I", zlib.crc32(kind + body))

src, dst = map(pathlib.Path, sys.argv[1:3])
dst.mkdir(parents=True, exist_ok=True)
for f in sorted(src.glob("*.png")):
    data = f.read_bytes()
    ihdr = idat = b""
    for kind, body in chunks(data):
        if kind == b"IHDR":
            ihdr = body
        elif kind == b"IDAT":
            idat += body
    out = data[:8] + chunk(b"IHDR", ihdr) + chunk(b"IDAT", zlib.compress(zlib.decompress(idat), 9)) + chunk(b"IEND", b"")
    (dst / f.name).write_bytes(out)
    print(f"{f.name}: {len(data)} -> {len(out)} bytes")
