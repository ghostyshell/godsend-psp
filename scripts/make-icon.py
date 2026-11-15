#!/usr/bin/env python3
"""Generate the GODsend PSP source icon (1024x1024 PNG), pure stdlib.

Design: dark rounded-square ink field, indigo->cyan laser diagonal with a
soft glow, and a minimal 'console d-pad + dot' mark. Mirrors the app's
laser-gradient design language.
"""

import struct
import zlib
import math

SIZE = 1024

# palette
INK = (5, 8, 15)
LASER_A = (86, 232, 245)   # cyan
LASER_B = (134, 120, 255)  # indigo


def lerp(a, b, t):
    return tuple(int(a[i] + (b[i] - a[i]) * t) for i in range(3))


def rounded_rect_mask(x, y, radius=200):
    """1 inside the rounded rect, 0 outside, with a 1px soft edge."""
    w = h = SIZE
    nx = min(x, w - 1 - x)
    ny = min(y, h - 1 - y)
    if nx >= radius or ny >= radius:
        return 1.0
    # corner circle test
    cx, cy = radius - nx, radius - ny
    d = math.hypot(cx, cy)
    if d <= radius - 1:
        return 1.0
    if d >= radius + 1:
        return 0.0
    return 0.5


def laser_t(x, y):
    """t along the 112deg-ish diagonal + radial glow boost."""
    t = (x / SIZE) * 0.55 + (y / SIZE) * 0.45
    return max(0.0, min(1.0, t))


def main():
    rows = []
    for y in range(SIZE):
        row = bytearray()
        row.append(0)  # filter type: none
        for x in range(SIZE):
            mask = rounded_rect_mask(x, y)
            if mask == 0.0:
                row.extend(bytes(INK) + b"\x00")
                continue
            # base ink field
            px = INK
            # laser band: diagonal stripe with glow falloff
            d = (x + y) / (2 * SIZE)  # 0..1 along the diagonal
            band_center = 0.62
            band_width = 0.16
            dist = abs(d - band_center) / band_width
            if dist < 1.0:
                glow = (1.0 - dist) ** 2
                color = lerp(LASER_A, LASER_B, laser_t(x, y))
                px = lerp(px, color, glow * 0.92)
            # inner mark: d-pad cross + dot (cut out of the glow, ink filled)
            cx, cy = SIZE / 2, SIZE / 2
            ux, uy = x - cx, y - cy
            arm_w, arm_l = 46, 190
            in_cross = (abs(ux) <= arm_w and abs(uy) <= arm_l) or (
                abs(uy) <= arm_w and abs(ux) <= arm_l
            )
            dot_d = math.hypot(ux - (cx * 0.0 + 250), uy - 250)
            in_dot = dot_d <= 58
            if in_cross or in_dot:
                px = INK
                # subtle edge highlight on the mark
            alpha = int(255 * mask)
            row.extend(bytes(px) + bytes([alpha]))
        rows.append(bytes(row))

    raw = b"".join(rows)

    def chunk(tag, data):
        payload = tag + data
        return (
            struct.pack(">I", len(data))
            + payload
            + struct.pack(">I", zlib.crc32(payload) & 0xFFFFFFFF)
        )

    png = b"\x89PNG\r\n\x1a\n"
    png += chunk(b"IHDR", struct.pack(">IIBBBBB", SIZE, SIZE, 8, 6, 0, 0, 0))
    png += chunk(b"IDAT", zlib.compress(raw, 9))
    png += chunk(b"IEND", b"")

    with open(SIZE_PATH := "app/src-tauri/icons/icon.png", "wb") as f:
        f.write(png)
    print(f"wrote {SIZE_PATH} ({len(png)} bytes)")


if __name__ == "__main__":
    import os

    os.makedirs("app/src-tauri/icons", exist_ok=True)
    main()
