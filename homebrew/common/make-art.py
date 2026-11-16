#!/usr/bin/env python3
"""Generate ICON0 (144x80) and PIC1 (480x272) art for the GODsend PSP apps.

XMB shows ICON0 in the game list and PIC1 as the highlighted background.
Art uses the app's indigo-to-cyan family: a diagonal gradient, a soft glow,
and a white glyph per app. No fonts, no external deps.

    python3 homebrew/common/make-art.py            # write into each app folder
"""
import math
import os
import struct
import zlib

INDIGO = (79, 70, 229)
CYAN = (6, 182, 212)
SS = 2  # supersampling factor

APPS = {
    "godsend-cleaner": "broom",
    "godsend-saves": "floppy",
    "godsend-sysinfo": "info",
    "godsend-netcheck": "wifi",
}


class Canvas:
    def __init__(self, w, h):
        self.w, self.h = w, h
        self.px = [[(0.0, 0.0, 0.0, 0.0)] * w for _ in range(h)]

    def blend(self, x, y, color, alpha=1.0):
        if x < 0 or y < 0 or x >= self.w or y >= self.h or alpha <= 0:
            return
        r, g, b, a = color
        dr, dg, db, da = self.px[y][x]
        sa = alpha
        oa = sa + da * (1 - sa)
        if oa <= 0:
            return
        self.px[y][x] = (
            (r * sa + dr * da * (1 - sa)) / oa,
            (g * sa + dg * da * (1 - sa)) / oa,
            (b * sa + db * da * (1 - sa)) / oa,
            oa,
        )

    def disc(self, cx, cy, r, color, alpha=1.0):
        for y in range(int(cy - r), int(cy + r) + 1):
            for x in range(int(cx - r), int(cx + r) + 1):
                if (x + 0.5 - cx) ** 2 + (y + 0.5 - cy) ** 2 <= r * r:
                    self.blend(x, y, color, alpha)

    def ring(self, cx, cy, r, t, color, alpha=1.0):
        outer, inner = r + t / 2, r - t / 2
        for y in range(int(cy - outer), int(cy + outer) + 1):
            for x in range(int(cx - outer), int(cx + outer) + 1):
                d2 = (x + 0.5 - cx) ** 2 + (y + 0.5 - cy) ** 2
                if inner * inner <= d2 <= outer * outer:
                    self.blend(x, y, color, alpha)

    def rect(self, x0, y0, x1, y1, color, alpha=1.0):
        for y in range(int(y0), int(y1) + 1):
            for x in range(int(x0), int(x1) + 1):
                self.blend(x, y, color, alpha)

    def rrect(self, x0, y0, x1, y1, rad, color, alpha=1.0):
        for y in range(int(y0), int(y1) + 1):
            for x in range(int(x0), int(x1) + 1):
                dx = min(max(x0 + rad - x, 0), max(x - (x1 - rad), 0))
                dy = min(max(y0 + rad - y, 0), max(y - (y1 - rad), 0))
                if dx * dx + dy * dy <= rad * rad:
                    self.blend(x, y, color, alpha)

    def line(self, x0, y0, x1, y1, t, color, alpha=1.0):
        steps = int(max(abs(x1 - x0), abs(y1 - y0)) * 2) + 1
        for i in range(steps + 1):
            u = i / steps
            self.disc(x0 + (x1 - x0) * u, y0 + (y1 - y0) * u, t / 2, color, alpha)

    def poly(self, pts, color, alpha=1.0):
        ys = [p[1] for p in pts]
        for y in range(int(min(ys)), int(max(ys)) + 1):
            xs = []
            for i in range(len(pts)):
                ax, ay = pts[i]
                bx, by = pts[(i + 1) % len(pts)]
                if (ay <= y < by) or (by <= y < ay):
                    xs.append(ax + (y - ay) * (bx - ax) / (by - ay))
            xs.sort()
            for i in range(0, len(xs) - 1, 2):
                for x in range(int(xs[i]), int(xs[i + 1]) + 1):
                    self.blend(x, y, color, alpha)


def background(c, top=INDIGO, bottom=CYAN):
    w, h = c.w, c.h
    for y in range(h):
        for x in range(w):
            t = min(1.0, 0.68 * (x / w) + 0.32 * (y / h))
            r = top[0] + (bottom[0] - top[0]) * t
            g = top[1] + (bottom[1] - top[1]) * t
            b = top[2] + (bottom[2] - top[2]) * t
            dx, dy = x - w * 0.24, y - h * 0.12
            glow = max(0.0, 1 - math.hypot(dx, dy) / (0.75 * min(w, h)))
            r, g, b = r + 46 * glow, g + 46 * glow, b + 46 * glow
            cx, cy = x - w / 2, y - h / 2
            vig = 1 - 0.38 * min(1.0, math.hypot(cx, cy) / (0.72 * max(w, h)))
            c.blend(x, y, (min(255, r) * vig, min(255, g) * vig, min(255, b) * vig, 1.0), 1.0)


def glyph(c, kind, cx, cy, s):
    """s is the short side in supersampled pixels; glyph fills about 72%."""
    u = s * 0.36  # glyph half-height
    shadow = (0, 0, 0, 0.28)
    white = (255, 255, 255, 1.0)
    off = max(1.0, s * 0.012)

    def paint(fn, alpha=0.95):
        fn(cx + off, cy + off, shadow, 0.22)
        fn(cx, cy, white, alpha)

    if kind == "broom":
        # handle, three flared bristle blades, binding band, sparkles
        paint(lambda x, y, col, a: c.line(x + u * 0.42, y - u * 0.95, x - u * 0.06, y + u * 0.2, s * 0.055, col, a))
        for i in (-1, 0, 1):
            off = i * u * 0.26
            paint(lambda x, y, col, a, off=off: c.poly([
                (x + off - u * 0.105, y + u * 0.22),
                (x + off + u * 0.105, y + u * 0.22),
                (x + off * 1.45 + u * 0.15, y + u * 0.95),
                (x + off * 1.45 - u * 0.15, y + u * 0.95),
            ], col, a))
        paint(lambda x, y, col, a: c.rrect(x - u * 0.44, y + u * 0.14, x + u * 0.44, y + u * 0.32, s * 0.02, col, a))
        for sx, sy, r in ((-0.82, -0.62, 0.07), (-0.94, -0.2, 0.045), (0.74, -0.78, 0.055)):
            paint(lambda x, y, col, a, sx=sx, sy=sy, r=r: c.disc(x + u * sx, y + u * sy, s * r, col, a))

    elif kind == "floppy":
        paint(lambda x, y, col, a: c.rrect(x - u * 0.92, y - u * 0.92, x + u * 0.92, y + u * 0.92, s * 0.06, col, a))
        # shutter notch, label window, hub
        inner = (24, 32, 44, 1.0)
        c.rect(cx + u * 0.34 - off, cy - u * 0.92, cx + u * 0.92, cy - u * 0.34, inner, 0.9)
        c.rect(cx - u * 0.62, cy - u * 0.9, cx + u * 0.3, cy - u * 0.36, white, 0.95)
        c.rect(cx - u * 0.62, cy + u * 0.08, cx + u * 0.62, cy + u * 0.62, white, 0.95)
        c.disc(cx + u * 0.36, cy - u * 0.6, s * 0.035, inner, 0.9)

    elif kind == "info":
        paint(lambda x, y, col, a: c.ring(x, y, u * 0.95, s * 0.075, col, a))
        paint(lambda x, y, col, a: c.disc(x, y - u * 0.44, s * 0.062, col, a))
        paint(lambda x, y, col, a: c.rect(x - s * 0.042, y - u * 0.1, x + s * 0.042, y + u * 0.5, col, a))

    elif kind == "wifi":
        for i, rr in enumerate((0.34, 0.62, 0.9)):
            alpha = 0.95 - i * 0.16
            paint(lambda x, y, col, a, rr=rr, alpha=alpha: c.ring(x, y + u * 0.5, u * rr, s * 0.062, col, min(a, alpha)))
        paint(lambda x, y, col, a: c.disc(x, y + u * 0.62, s * 0.072, col, a))


def downsample(c):
    out = Canvas(c.w // SS, c.h // SS)
    for y in range(out.h):
        for x in range(out.w):
            rs = gs = bs = as_ = 0.0
            for dy in range(SS):
                for dx in range(SS):
                    r, g, b, a = c.px[y * SS + dy][x * SS + dx]
                    rs += r * a
                    gs += g * a
                    bs += b * a
                    as_ += a
            n = SS * SS
            if as_ > 0:
                out.px[y][x] = (rs / as_, gs / as_, bs / as_, as_ / n)
            else:
                out.px[y][x] = (0, 0, 0, 0)
    return out


def write_png(path, c):
    w, h = c.w, c.h
    raw = bytearray()
    for y in range(h):
        raw.append(0)
        for x in range(w):
            r, g, b, a = c.px[y][x]
            raw += bytes((max(0, min(255, int(round(r)))), max(0, min(255, int(round(g)))),
                          max(0, min(255, int(round(b)))), max(0, min(255, int(round(a * 255))))))

    def chunk(tag, data):
        return struct.pack(">I", len(data)) + tag + data + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)

    png = b"\x89PNG\r\n\x1a\n"
    png += chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0))
    png += chunk(b"IDAT", zlib.compress(bytes(raw), 9))
    png += chunk(b"IEND", b"")
    with open(path, "wb") as f:
        f.write(png)


def build(kind, w, h, path):
    c = Canvas(w * SS, h * SS)
    background(c)
    glyph(c, kind, w * SS / 2, h * SS / 2, min(w, h) * SS)
    write_png(path, downsample(c))
    print(f"{path} ({w}x{h}, {os.path.getsize(path)} bytes)")


def main():
    here = os.path.dirname(os.path.abspath(__file__))
    root = os.path.dirname(here)
    for app, kind in APPS.items():
        d = os.path.join(root, app)
        build(kind, 144, 80, os.path.join(d, "icon0.png"))
        build(kind, 480, 272, os.path.join(d, "pic1.png"))


if __name__ == "__main__":
    main()
