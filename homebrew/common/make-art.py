#!/usr/bin/env python3
"""Generate ICON0 (144x80, transparent) and PIC1 (480x272) art for the
GODsend PSP apps.

ICON0 is a transparent glyph so it sits on the XMB wave instead of looking
like a pasted rectangle. PIC1 is a soft indigo/cyan backdrop with the wave
motif and a faint version of the same glyph. Both come from one glyph layer,
so the two always match.

    python3 homebrew/common/make-art.py
"""
import math
import os
import struct
import zlib

WHITE = (255, 255, 255, 1.0)
CYAN = (34, 211, 238, 1.0)
CYAN_DEEP = (6, 182, 212, 1.0)
SHADOW = (2, 6, 23, 0.55)
SS = 3  # supersampling

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
        sa = alpha * a
        if sa <= 0:
            return
        dr, dg, db, da = self.px[y][x]
        oa = sa + da * (1 - sa)
        if oa <= 0:
            return
        self.px[y][x] = (
            (r * sa + dr * da * (1 - sa)) / oa,
            (g * sa + dg * da * (1 - sa)) / oa,
            (b * sa + db * da * (1 - sa)) / oa,
            oa,
        )

    def clear(self, x, y):
        if 0 <= x < self.w and 0 <= y < self.h:
            self.px[y][x] = (0.0, 0.0, 0.0, 0.0)

    # ---- shapes -------------------------------------------------------
    def disc(self, cx, cy, r, color, alpha=1.0):
        for y in range(int(cy - r) - 1, int(cy + r) + 2):
            for x in range(int(cx - r) - 1, int(cx + r) + 2):
                if (x + 0.5 - cx) ** 2 + (y + 0.5 - cy) ** 2 <= r * r:
                    self.blend(x, y, color, alpha)

    def ring(self, cx, cy, r, t, color, alpha=1.0, a0=0.0, a1=360.0):
        outer, inner = r + t / 2, r - t / 2
        for y in range(int(cy - outer) - 1, int(cy + outer) + 2):
            for x in range(int(cx - outer) - 1, int(cx + outer) + 2):
                dx, dy = x + 0.5 - cx, y + 0.5 - cy
                d2 = dx * dx + dy * dy
                if inner * inner <= d2 <= outer * outer:
                    ang = (math.degrees(math.atan2(dy, dx)) + 360.0) % 360.0
                    if a0 <= ang <= a1:
                        self.blend(x, y, color, alpha)

    def rect(self, x0, y0, x1, y1, color, alpha=1.0):
        for y in range(int(y0), int(y1) + 1):
            for x in range(int(x0), int(x1) + 1):
                self.blend(x, y, color, alpha)

    def rrect(self, x0, y0, x1, y1, rad, color, alpha=1.0):
        for y in range(int(y0) - 1, int(y1) + 2):
            for x in range(int(x0) - 1, int(x1) + 2):
                dx = min(max(x0 + rad - x, 0), max(x - (x1 - rad), 0))
                dy = min(max(y0 + rad - y, 0), max(y - (y1 - rad), 0))
                if dx * dx + dy * dy <= rad * rad:
                    self.blend(x, y, color, alpha)

    def clear_rrect(self, x0, y0, x1, y1, rad):
        for y in range(int(y0) - 1, int(y1) + 2):
            for x in range(int(x0) - 1, int(x1) + 2):
                dx = min(max(x0 + rad - x, 0), max(x - (x1 - rad), 0))
                dy = min(max(y0 + rad - y, 0), max(y - (y1 - rad), 0))
                if dx * dx + dy * dy <= rad * rad:
                    self.clear(x, y)

    def line(self, x0, y0, x1, y1, t, color, alpha=1.0):
        steps = int(max(abs(x1 - x0), abs(y1 - y0)) * 2) + 1
        for i in range(steps + 1):
            u = i / steps
            self.disc(x0 + (x1 - x0) * u, y0 + (y1 - y0) * u, t / 2, color, alpha)

    def poly(self, pts, color, alpha=1.0):
        ys = [p[1] for p in pts]
        for y in range(int(min(ys)) - 1, int(max(ys)) + 2):
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


def composite(dst, src, dx, dy, alpha=1.0):
    for y in range(src.h):
        for x in range(src.w):
            r, g, b, a = src.px[y][x]
            if a > 0:
                dst.blend(x + dx, y + dy, (r, g, b, 1.0), a * alpha)


def glyph_layer(kind, s):
    """Draw the glyph on a transparent square of side s, centred."""
    c = Canvas(s, s)
    cx = cy = s / 2
    u = s * 0.30  # glyph unit

    if kind == "broom":
        # shadow pass
        c.line(cx + u * 0.46, cy - u * 0.98, cx + u * 0.02, cy + u * 0.16, s * 0.070, SHADOW, 0.9)
        # handle
        c.line(cx + u * 0.44, cy - u * 1.00, cx, cy + u * 0.14, s * 0.060, WHITE)
        # binding band
        c.rrect(cx - u * 0.52, cy + u * 0.10, cx + u * 0.40, cy + u * 0.30, s * 0.03, CYAN_DEEP)
        # bristles
        for i in (-1, 0, 1):
            off = i * u * 0.30
            c.poly([
                (cx + off - u * 0.115, cy + u * 0.26),
                (cx + off + u * 0.115, cy + u * 0.26),
                (cx + off * 1.35 + u * 0.135, cy + u * 1.00),
                (cx + off * 1.35 - u * 0.135, cy + u * 1.00),
            ], WHITE)
        # sparkles
        for sx, sy, r in ((-0.86, -0.74, 0.085), (-1.0, -0.3, 0.05)):
            c.disc(cx + u * sx, cy + u * sy, u * r, CYAN)

    elif kind == "floppy":
        c.rrect(cx - u * 0.92, cy - u * 0.92, cx + u * 0.92, cy + u * 0.92, s * 0.075, SHADOW, 0.85)
        c.rrect(cx - u * 0.90, cy - u * 0.94, cx + u * 0.90, cy + u * 0.90, s * 0.07, WHITE)
        # shutter cut top-right
        c.clear_rrect(cx + u * 0.20, cy - u * 0.94, cx + u * 0.90, cy - u * 0.30, s * 0.035)
        c.rrect(cx + u * 0.20, cy - u * 0.94, cx + u * 0.90, cy - u * 0.30, s * 0.035, CYAN_DEEP)
        c.disc(cx + u * 0.55, cy - u * 0.62, u * 0.11, WHITE)
        # label window
        c.clear_rrect(cx - u * 0.62, cy + u * 0.02, cx + u * 0.66, cy + u * 0.62, s * 0.03)
        c.rrect(cx - u * 0.62, cy + u * 0.02, cx + u * 0.66, cy + u * 0.62, s * 0.03, WHITE, 0.92)
        c.rect(cx - u * 0.44, cy + u * 0.16, cx + u * 0.48, cy + u * 0.26, CYAN_DEEP, 0.9)
        c.rect(cx - u * 0.44, cy + u * 0.36, cx + u * 0.20, cy + u * 0.44, CYAN_DEEP, 0.55)

    elif kind == "info":
        c.ring(cx, cy, u * 0.92, s * 0.075, SHADOW, 0.8)
        c.ring(cx, cy, u * 0.92, s * 0.070, WHITE)
        c.disc(cx, cy - u * 0.42, u * 0.115, CYAN)
        c.rrect(cx - u * 0.085, cy - u * 0.05, cx + u * 0.085, cy + u * 0.55, s * 0.035, WHITE)

    elif kind == "wifi":
        for i, rr in enumerate((0.36, 0.63, 0.90)):
            w = s * 0.062
            c.ring(cx, cy + u * 0.62, u * rr, w, SHADOW, 0.75, 205, 335)
            c.ring(cx, cy + u * 0.62, u * rr, w, WHITE, 1.0 - i * 0.12, 205, 335)
        c.disc(cx, cy + u * 0.72, u * 0.12, CYAN)

    return c


def icon0(kind, path):
    w, h = 144, 80
    out = Canvas(w * SS, h * SS)
    g = glyph_layer(kind, int(min(w, h) * SS * 0.80))
    composite(out, g, (out.w - g.w) // 2, (out.h - g.h) // 2, 1.0)
    write_png(path, downsample(out))
    print(f"{path} (144x80 transparent, {os.path.getsize(path)} bytes)")


def pic1(kind, path):
    w, h = 480, 272
    c = Canvas(w * SS, h * SS)
    W, H = c.w, c.h

    # Base: deep indigo with a cyan corner glow.
    for y in range(H):
        for x in range(W):
            ty = y / H
            tx = x / W
            r = 10 + 26 * ty + 8 * tx
            g = 15 + 22 * ty + 10 * tx
            b = 40 + 46 * ty + 26 * tx
            dx, dy = x - W * 0.78, y - H * 0.85
            glow = max(0.0, 1 - math.hypot(dx, dy) / (0.62 * min(W, H)))
            r += 8 * glow
            g += 74 * glow
            b += 86 * glow
            c.blend(x, y, (min(255, r), min(255, g), min(255, b), 1.0))

    # XMB-style wave lines along the lower third.
    for k, (amp, base, thick, alpha) in enumerate(((16, 0.70, 2.4, 0.11), (11, 0.78, 1.8, 0.07), (7, 0.86, 1.2, 0.045))):
        phase = k * 1.1
        for x in range(W):
            yy = H * base + math.sin(x / W * math.pi * 2.3 + phase) * amp * SS
            c.disc(x, yy, thick * SS / 2, CYAN, alpha)

    # Faint glyph as texture, small and right of centre so UI text stays clean.
    g = glyph_layer(kind, int(min(W, H) * SS * 0.50))
    composite(c, g, int(W * 0.80 - g.w / 2), int(H * 0.50 - g.h / 2), 0.10)

    # Vignette.
    for y in range(H):
        for x in range(W):
            r, gg, b, a = c.px[y][x]
            d = math.hypot(x - W / 2, y - H / 2) / (0.75 * max(W, H))
            v = 1 - 0.45 * min(1.0, d)
            c.px[y][x] = (r * v, gg * v, b * v, a)

    write_png(path, downsample(c))
    print(f"{path} (480x272, {os.path.getsize(path)} bytes)")


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
                out.px[y][x] = (0.0, 0.0, 0.0, 0.0)
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


def preview():
    """Contact sheet of the icons on the XMB backdrop, for eyeballing."""
    pad, scale = 16, 2
    cell_w, cell_h = 144 * scale, 80 * scale
    c = Canvas((cell_w + pad) * 4 + pad, cell_h + pad * 2)
    for y in range(c.h):
        for x in range(c.w):
            c.blend(x, y, (11, 16, 32, 1.0))
    for i, (app, kind) in enumerate(APPS.items()):
        g = glyph_layer(kind, int(80 * scale * 0.80))
        composite(c, g, pad + i * (cell_w + pad) + (cell_w - g.w) // 2,
                  pad + (cell_h - g.h) // 2, 1.0)
    out = "/tmp/godsend-icons-preview.png"
    write_png(out, downsample(c))
    print(out)


def main():
    here = os.path.dirname(os.path.abspath(__file__))
    root = os.path.dirname(here)
    for app, kind in APPS.items():
        d = os.path.join(root, app)
        icon0(kind, os.path.join(d, "icon0.png"))
        pic1(kind, os.path.join(d, "pic1.png"))
    if "--preview" in os.sys.argv:
        preview()


if __name__ == "__main__":
    main()
