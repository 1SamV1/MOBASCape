#!/usr/bin/env python3
"""MobaScape progress screen as a Matrix (2000) era BBS / scene .NFO.

Block-shaded logo, ░▒▓ fades, -=[ ]=- headers and digital rain behind it.
Writes <out>.txt (plain), <out>.ans (24-bit ANSI colour) and <out>.png.

Usage: python3 tools/matrix_bbs.py docs/PROGRESS_MATRIX
"""
import random
import sys

from PIL import Image, ImageDraw, ImageFont

W, H = 150, 62
out = sys.argv[1]

WHITE = (235, 255, 235)
HEAD = (200, 255, 200)
BRIGHT = (0, 255, 70)
MID = (0, 190, 45)
DIM = (0, 115, 28)
FAINT = (0, 52, 12)
AMBER = (210, 255, 120)
RED = (255, 90, 70)
CYAN = (110, 255, 210)
BLACK = (0, 0, 0)

grid = [[(" ", FAINT) for _ in range(W)] for _ in range(H)]


def put(x, y, ch, fg):
    if 0 <= x < W and 0 <= y < H:
        grid[y][x] = (ch, fg)


def text(x, y, s, fg=BRIGHT):
    for i, ch in enumerate(s):
        put(x + i, y, ch, fg)


def center(y, s, fg=BRIGHT):
    text((W - len(s)) // 2, y, s, fg)


def clear(x0, y0, x1, y1):
    for y in range(y0, y1 + 1):
        for x in range(x0, x1 + 1):
            put(x, y, " ", BLACK)


# --- digital rain -----------------------------------------------------------
rng = random.Random(1999)
GLYPHS = "0123456789ABCDEF:.=*+-<>|^~$#@%&?!ZQXJ7kT" + "ｱｲｳｴｵｶｷｸｹｺ"
GLYPHS = "".join(c for c in GLYPHS if ord(c) < 0x2000)
for x in range(W):
    if rng.random() < 0.22:
        continue
    for _ in range(rng.choice([1, 2, 2])):
        head = rng.randrange(-4, H + 8)
        length = rng.randrange(6, 30)
        for k in range(length):
            y = head - k
            if not 0 <= y < H:
                continue
            col = HEAD if k == 0 else BRIGHT if k < 3 else MID if k < length * 0.45 else DIM if k < length * 0.8 else FAINT
            put(x, y, rng.choice(GLYPHS), col)


# --- NFO frame pieces -------------------------------------------------------
def fade_bar(y, x0, x1):
    """░▒▓█ ... █▓▒░ rule across a section."""
    clear(x0, y, x1, y)
    ends = "░▒▓"
    for i, ch in enumerate(ends):
        put(x0 + i, y, ch, [DIM, MID, BRIGHT][i])
        put(x1 - i, y, ch, [DIM, MID, BRIGHT][i])
    for x in range(x0 + 3, x1 - 2):
        put(x, y, "▄" if (x // 2) % 2 else "▀", MID if (x // 7) % 2 else DIM)


def header(y, label, x0=4, x1=W - 5):
    clear(x0, y, x1, y)
    s = f"-=[ {label} ]=-"
    mid = (x0 + x1) // 2
    sx = mid - len(s) // 2
    for x in range(x0, x1 + 1):
        put(x, y, "─", DIM)
    put(x0, y, "■", MID)
    put(x1, y, "■", MID)
    text(sx - 2, y, "  ", BLACK)
    text(sx, y, s, HEAD)
    text(sx + len(s), y, "  ", BLACK)


def box(x0, y0, x1, y1, label):
    clear(x0, y0, x1, y1)
    for x in range(x0, x1 + 1):
        put(x, y0, "▀", MID)
        put(x, y1, "▄", MID)
    for y in range(y0, y1 + 1):
        put(x0, y, "█", DIM)
        put(x1, y, "█", DIM)
    put(x0, y0, "▄", BRIGHT)
    put(x1, y0, "▄", BRIGHT)
    tag = f"▐ {label} ▌"
    text(x0 + 3, y0, tag, HEAD)


def elite(s):
    """sCeNe cAsE: lower vowels, upper the rest."""
    return "".join(c.lower() if c.lower() in "aeiou" else c.upper() for c in s)


# --- the logo: 7-row block font with a ▓▒░ drop shadow -----------------------
LOGO = {
    "M": ["██▄   ▄██", "███▄ ▄███", "██ ▀█▀ ██", "██     ██", "██     ██", "██     ██"],
    "O": [" ▄█████▄ ", "██     ██", "██     ██", "██     ██", "██     ██", " ▀█████▀ "],
    "B": ["███████▄ ", "██     ██", "███████▀ ", "██     ██", "██     ██", "███████▀ "],
    "A": [" ▄█████▄ ", "██     ██", "█████████", "██     ██", "██     ██", "██     ██"],
    "S": [" ▄██████▄", "██       ", " ▀█████▄ ", "       ██", "       ██", "▀██████▀ "],
    "C": [" ▄██████▄", "██       ", "██       ", "██       ", "██       ", " ▀██████▀"],
    "P": ["███████▄ ", "██     ██", "███████▀ ", "██       ", "██       ", "██       "],
    "E": ["█████████", "██       ", "███████  ", "██       ", "██       ", "█████████"],
}
LOGO_ROWS = 6
LOGO_GRAD = [WHITE, HEAD, BRIGHT, BRIGHT, MID, DIM]


def logo(y, word):
    width = len(word) * 10 - 1
    x0 = (W - width) // 2
    clear(x0 - 4, y - 1, x0 + width + 4, y + LOGO_ROWS)
    # Shadow first, one right and one down, then the face over it.
    for pass_ in ("shadow", "face"):
        for i, ch in enumerate(word):
            for r in range(LOGO_ROWS):
                for c, px in enumerate(LOGO[ch][r]):
                    if px == " ":
                        continue
                    x = x0 + i * 10 + c
                    if pass_ == "shadow":
                        if grid[y + r + 1][x + 1][0] == " ":
                            put(x + 1, y + r + 1, "░", DIM)
                    else:
                        put(x, y + r, px, LOGO_GRAD[r])
    return x0, width


# ============================================================================
def bevel_logo(y, word):
    """The block logo with a bevel: lit left edge, ▓ body, ▒ right edge."""
    width = len(word) * 10 - 1
    x0 = (W - width) // 2
    clear(x0 - 5, y - 1, x0 + width + 5, y + LOGO_ROWS + 1)
    for i, ch in enumerate(word):
        rows = LOGO[ch]
        for r in range(LOGO_ROWS):
            for c, px in enumerate(rows[r]):
                if px == " ":
                    continue
                x = x0 + i * 10 + c
                left = c == 0 or rows[r][c - 1] == " "
                right = c + 1 >= len(rows[r]) or rows[r][c + 1] == " "
                if px != "█":
                    put(x, y + r, px, LOGO_GRAD[r])
                elif left:
                    put(x, y + r, "█", WHITE if r < 2 else HEAD)
                elif right:
                    put(x, y + r, "▒", MID)
                else:
                    put(x, y + r, "▓", LOGO_GRAD[r])
                if grid[y + r + 1][x + 1][0] == " ":
                    put(x + 1, y + r + 1, "░", FAINT)
    # Corner brackets, the old group-logo way.
    for dx, dy, ch in [(-4, -1, "▄▄▄"), (width + 2, -1, "▄▄▄"), (-4, LOGO_ROWS, "▀▀▀"), (width + 2, LOGO_ROWS, "▀▀▀")]:
        text(x0 + dx, y + dy, ch, BRIGHT)
    for r in range(LOGO_ROWS):
        put(x0 - 4, y + r, "█", DIM)
        put(x0 + width + 4, y + r, "█", DIM)


fade_bar(1, 2, W - 3)
bevel_logo(3, "MOBASCAPE")
center(11, "  ░▒▓█  r i F T   c R e W   p R e S e N T s  █▓▒░  ", AMBER)
center(12, "  " + elite("phase.1 closed :: the light is data") + "  ", WHITE)
fade_bar(13, 2, W - 3)

# --- release info -------------------------------------------------------------
header(15, elite("release info"))
clear(4, 16, W - 5, 19)
info = [
    ("rELEASE dATE", "2026-11-23", "cOMMIT", "e28cf54  ->  sam/main"),
    ("tYPE", "rENDERER / dATA / mINIMAP", "tESTS", "383 PASS // 0 FAIL   (+5)"),
    ("rELOAD kEY", "F5 :: shader + lighting.json", "cLIPPY", "-D warnings :: CLEAN"),
    ("rULE", "ONLY OSRS CACHE ASSETS", "sIZE", "9 knobs / 3 uniforms / 0 rebuilds"),
]
for i, (k1, v1, k2, v2) in enumerate(info):
    y = 16 + i
    text(8, y, f"{k1:.<16}", DIM)
    text(25, y, v1, AMBER if k1 == "rULE" else BRIGHT)
    text(76, y, f"{k2:.<12}", DIM)
    text(89, y, v2, HEAD)

# --- left: hex dump of the new data file ---------------------------------------
box(4, 21, 76, 39, elite("data/lighting.json // hexview"))
knobs = [
    ("terrain_ambient", 1.30, 4.0, "1.64"),
    ("unit_ambient", 1.48, 4.0, "1.74"),
    ("sun_strength", 0.90, 4.0, "0.72"),
    ("shadow_floor", 0.36, 1.0, "0.52"),
    ("hidden_tint", "[.40 .42 .47]", None, "--"),
    ("hidden_saturation", 0.45, 1.0, "--"),
    ("grass_saturation", 1.12, 3.0, "1.30"),
    ("grass_brightness", 0.58, 2.0, "0.66"),
    ("grass_mottle", 0.30, 2.0, "0.30"),
]
text(7, 23, "OFFS  KEY                  VALUE    WAS    RANGE", DIM)
for i, (key, value, high, was) in enumerate(knobs):
    y = 24 + i
    text(7, y, f"{i * 0x18:04X}", FAINT if i % 2 else DIM)
    text(13, y, key, CYAN)
    shown = value if isinstance(value, str) else f"{value:.2f}"
    text(34, y, shown if len(shown) < 9 else "", WHITE)
    if len(shown) >= 9:
        text(34, y, shown, WHITE)
        continue
    text(43, y, was, DIM if was == "--" else RED)
    filled = int(round(value / high * 14))
    text(50, y, "[", DIM)
    for j in range(14):
        put(51 + j, y, "▓" if j < filled else "░", BRIGHT if j < filled else FAINT)
    text(65, y, "]", DIM)
    text(67, y, f"0..{high:g}", FAINT)
text(7, 34, "> F5  -> naga validates shader, serde validates json", AMBER)
text(7, 35, "> bad key? 'sun_strenght' -> REFUSED, old values stay", AMBER)
text(7, 36, "> test: extreme file dropped frame luma 75 -> 59", BRIGHT)
text(7, 37, "> test: shipped file == built-in defaults (bit exact)", BRIGHT)

# --- right: the camera footprint -----------------------------------------------
box(78, 21, W - 5, 39, elite("frustum.com // minimap view"))
eye_x, eye_y = 100, 23
text(eye_x - 1, eye_y, "(@)", WHITE)
text(eye_x + 4, eye_y, "camera eye", DIM)
# Four rays fanning to the ground trapezoid.
far_l, far_r, near_l, near_r = 82, 118, 92, 108
ground_far, ground_near = 30, 35
for step in range(1, ground_far - eye_y):
    t = step / (ground_far - eye_y)
    put(int(round(eye_x + (far_l - eye_x) * t)), eye_y + step, "/", MID)
    put(int(round(eye_x + 1 + (far_r - eye_x - 1) * t)), eye_y + step, "\\", MID)
for step in range(1, ground_near - eye_y):
    t = step / (ground_near - eye_y)
    put(int(round(eye_x + (near_l - eye_x) * t)), eye_y + step, ":", DIM)
    put(int(round(eye_x + 1 + (near_r - eye_x - 1) * t)), eye_y + step, ":", DIM)
for x in range(far_l, far_r + 1):
    put(x, ground_far, "═", BRIGHT)
for x in range(near_l, near_r + 1):
    put(x, ground_near, "═", BRIGHT)
for y in range(ground_far + 1, ground_near):
    t = (y - ground_far) / (ground_near - ground_far)
    put(int(round(far_l + (near_l - far_l) * t)), y, "\\", BRIGHT)
    put(int(round(far_r + (near_r - far_r) * t)), y, "/", BRIGHT)
    for x in range(int(round(far_l + (near_l - far_l) * t)) + 1, int(round(far_r + (near_r - far_r) * t))):
        put(x, y, "░", DIM)
text(far_l - 1, ground_far - 1, "TL", HEAD)
text(far_r, ground_far - 1, "TR", HEAD)
text(near_l - 1, ground_near + 1, "BL", HEAD)
text(near_r, ground_near + 1, "BR", HEAD)
text(94, 33, "screen", WHITE)
text(80, 37, "corner rays hit y = focus; horizon rays capped at 60k", DIM)
text(80, 38, "drawn unclamped, clipped by the map frame", DIM)
# A tiny minimap with the trapezoid in it.
mx, my = 124, 24
for py in range(11):
    for px in range(17):
        edge = py in (0, 10) or px in (0, 16)
        put(mx + px, my + py, "█" if edge else "░", DIM if edge else FAINT)
for px in range(2, 10):
    put(mx + px, my + 6, "▀", BRIGHT)
for px in range(4, 8):
    put(mx + px, my + 9, "▄", BRIGHT)
put(mx + 2, my + 7, "\\", BRIGHT)
put(mx + 3, my + 8, "\\", BRIGHT)
put(mx + 9, my + 7, "/", BRIGHT)
put(mx + 8, my + 8, "/", BRIGHT)
put(mx + 5, my + 8, "@", AMBER)
put(mx + 13, my + 2, "■", RED)
text(mx, my + 12, "minimap.view", CYAN)

# --- roadmap -------------------------------------------------------------------
header(41, elite("roadmap // the oracle says"))
clear(4, 42, W - 5, 49)
phases = [
    ("PHASE 1", "MAP LIGHT + COLOUR", 9, "fog, light, grass, lighting.json DONE // floor ids open"),
    ("PHASE 2", "MINIMAP", 4, "veil, rings, trapezoid DONE // portraits, sharp bake"),
    ("PHASE 3", "SHOP", 0, "tiers, icon grid, recipes A+B>C, sell, undo"),
    ("PHASE 4", "HERO SELECT", 1, "lanes, team columns, battle spells, stone panels"),
    ("PHASE 5", "ARENA SET DRESSING", 0, "cache cliffs, plank bridges, god castles"),
    ("PHASE 6", "HUD POLISH", 0, "objectives timers, ally rows, slim bar"),
    ("PHASE 7", "POST-MATCH", 0, "victory board, K/D/A, items, damage"),
]
for i, (p, name, done, note) in enumerate(phases):
    y = 42 + i
    text(8, y, p, WHITE)
    text(18, y, name, CYAN)
    text(40, y, "[", DIM)
    for j in range(10):
        lit_cell = j < done
        put(41 + j, y, "▓" if lit_cell else "░", BRIGHT if lit_cell else FAINT)
    text(51, y, "]", DIM)
    text(53, y, f"{done * 10:>3}%", HEAD if done else FAINT)
    text(59, y, note, BRIGHT if "DONE" in note else MID)

# --- greets + footer -----------------------------------------------------------
header(51, elite("greetz + memberz"))
clear(4, 52, W - 5, 55)
center(52, "cODE: lighting.rs  *  gFX: scene.wgsl  *  mAP: minimap.rs  *  dATA: lighting.json  *  sFX: the osrs synth", MID)
center(53, "gREETZ: tHE OSRS CACHE  *  sERDE  *  NAGA  *  wgpu  *  tHE BOT LANERS WHO NEVER COMPLAIN", MID)
center(54, "nOT AFFILIATED WITH JAGEX. cACHE ASSETS STAY OUT OF GIT. aLWAYS.", DIM)

fade_bar(57, 2, W - 3)
clear(14, 58, W - 15, 60)
center(58, "  w h a t   i s   t h e   m a t r i x ?   a   j s o n   f i l e   a n d   a n   F 5   k e y .  ", DIM)
center(59, "  t h e r e   i s   n o   r e b u i l d .  ", HEAD)
center(60, "-=[ mObAsCaPe // 2026 // nODE 1: tHE cONsTrUcT // 14.4k-56k // sYSoP: rIFT // 24hRs ]=-", AMBER)

# --- outputs ------------------------------------------------------------------
with open(out + ".txt", "w") as f:
    for row in grid:
        f.write("".join(c for c, _ in row).rstrip() + "\n")

with open(out + ".ans", "w") as f:
    for row in grid:
        last = None
        line = []
        for ch, fg in row:
            if fg != last:
                line.append(f"\x1b[38;2;{fg[0]};{fg[1]};{fg[2]}m")
                last = fg
            line.append(ch)
        f.write("".join(line) + "\x1b[0m\n")

font = ImageFont.truetype("/usr/share/fonts/TTF/DejaVuSansMono.ttf", 15)
bold = ImageFont.truetype("/usr/share/fonts/TTF/DejaVuSansMono-Bold.ttf", 15)
cw = int(round(font.getlength("M")))
ch_h = 18
img = Image.new("RGB", (W * cw + 16, H * ch_h + 16), BLACK)
d = ImageDraw.Draw(img)
for y, row in enumerate(grid):
    for x, (c, fg) in enumerate(row):
        if c == " ":
            continue
        px, py = 8 + x * cw, 8 + y * ch_h
        # Draw block elements as rectangles so they tile without gaps.
        if c in "█▀▄▌▐■":
            boxes = {
                "█": (0, 0, 1, 1), "▀": (0, 0, 1, 0.5), "▄": (0, 0.5, 1, 1),
                "▌": (0, 0, 0.5, 1), "▐": (0.5, 0, 1, 1), "■": (0.2, 0.3, 0.8, 0.75),
            }[c]
            d.rectangle(
                (px + boxes[0] * cw, py + boxes[1] * ch_h, px + boxes[2] * cw - 1, py + boxes[3] * ch_h - 1),
                fill=fg,
            )
        elif c in "░▒▓":
            density = {"░": 0.25, "▒": 0.5, "▓": 0.75}[c]
            for sy in range(0, ch_h, 2):
                for sx in range(0, cw, 2):
                    if ((sx // 2) + (sy // 2)) % 4 < density * 4:
                        d.rectangle((px + sx, py + sy, px + sx + 1, py + sy + 1), fill=fg)
        else:
            f = bold if fg in (HEAD, WHITE, BRIGHT, AMBER, CYAN) else font
            d.text((px, py), c, fill=fg, font=f)
img.save(out + ".png")
print(f"wrote {out}.txt/.ans/.png ({img.size[0]}x{img.size[1]})")
