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
fade_bar(1, 2, W - 3)
x0, lw = logo(3, "MOBASCAPE")
center(10, "  ▓▒░  p  r  e  s  e  n  t  s   ░▒▓  ", AMBER)
center(11, "  " + elite("phase.1 :: the fog of war") + "  ", WHITE)
fade_bar(12, 2, W - 3)

# --- release info -------------------------------------------------------------
header(14, elite("release info"))
clear(4, 15, W - 5, 19)
info = [
    ("rELEASE dATE", "2026-11-23", "sUPPLIER", "MobaScape Movement System Branch"),
    ("tYPE", "eNGINE / rENDERER / hUD", "cOMMITS", "1438692  +  28e98c7  ->  sam/main"),
    ("tESTS", "378 PASS // 0 FAIL", "cLIPPY", "-D warnings :: CLEAN"),
    ("sOURCE", "VIDEO STUDY: 70 STILLS / 15 s", "rULE", "ONLY OSRS CACHE ASSETS. NO EXCEPTIONS."),
]
for i, (k1, v1, k2, v2) in enumerate(info):
    y = 15 + i
    text(8, y, f"{k1:.<16}", DIM)
    text(25, y, v1, BRIGHT)
    text(76, y, f"{k2:.<12}", DIM)
    text(89, y, v2, HEAD if k2 != "rULE" else AMBER)

# --- left box: what got cracked ------------------------------------------------
box(4, 21, 74, 40, elite("what got cracked"))
done = [
    ("[+]", "ROADMAPPLAN.md", "7 phases: light, minimap, shop,"),
    ("", "", "select, set dressing, HUD, post-match"),
    ("[+]", "MINIMAP VEIL", "team LOS sampled 127x126, live"),
    ("[+]", "HERO RINGS", "team colour, player in gold"),
    ("[+]", "WORLD FOG OF WAR", "LOS grid -> 512^2 R8 texture"),
    ("", "", "5-tap soft edge, binds 8 + 9"),
    ("", "", "unseen: 40% light, greyed out"),
    ("[+]", "LIGHT RETUNE", "ambient 1.64 -> 1.30"),
    ("", "", "sun 0.72 -> 0.90  shadow 0.52 -> 0.36"),
    ("", "", "post saturation 0.95 -> 0.86"),
    ("[+]", "GRASS PALETTE", "lime 'dry' tint -> olive, boost 1.3->1.12"),
    ("[+]", "TESTS", "+4: veil runs, rings, texel packing x2"),
]
for i, (tag, k, v) in enumerate(done):
    y = 23 + i
    if tag:
        text(7, y, tag, WHITE)
    text(11, y, k, CYAN)
    text(30, y, v, BRIGHT)
text(7, 36, "wALLS AND TREES NOW CAST SIGHT SHADOWS ON THE MAP.", AMBER)
text(7, 37, "mENUS + DEMOS KEEP THE OLD CIRCLE (clear_vision_grid).", DIM)
text(7, 38, "sTILLS OF THE REFERENCE VIDEO ARE NOT COMMITTED.", DIM)

# --- right box: an ASCII minimap of the fog ------------------------------------
box(76, 21, W - 5, 40, elite("minimap.exe // live los"))
mx, my, mw, mh = 80, 23, 32, 15
rng2 = random.Random(7)


def lit(px, py):
    # Lit patches: blue fountain bottom-left, towers and waves along lanes.
    spots = [(2, 13, 4.5), (9, 13, 2.5), (2, 7, 2.2), (8, 9, 2.8), (17, 13, 2.4), (12, 6, 1.6)]
    for sx, sy, r in spots:
        if (px - sx) ** 2 + ((py - sy) * 1.9) ** 2 <= (r * 1.9) ** 2:
            return True
    return False


for py in range(mh):
    for px in range(mw):
        lane = py == 1 or px == 1 or py == mh - 2 or px == mw - 2 or abs((mw - 1 - px) * mh / mw - py) < 0.8
        river = abs(px * mh / mw - py) < 0.7
        if lit(px, py):
            ch, col = ("█", BRIGHT) if lane else ("▓", MID)
            if river:
                ch, col = "≈", CYAN
        else:
            ch, col = ("▒", DIM) if lane else ("░", FAINT)
            if river:
                ch, col = "~", DIM
        put(mx + px, my + py, ch, col)
put(mx + 2, my + 12, "@", AMBER)          # the player, gold ring
for tx, ty in [(9, 13), (2, 7), (17, 13)]:
    put(mx + tx, my + ty, "■", CYAN)      # allied towers
for tx, ty in [(29, 1), (22, 1), (30, 6), (25, 5)]:
    put(mx + tx, my + ty, "■", RED)       # enemy towers, remembered
text(mx + mw + 2, my + 0, "LEGEND", HEAD)
text(mx + mw + 2, my + 2, "▓█ SEEN", BRIGHT)
text(mx + mw + 2, my + 3, "░▒ FOG", DIM)
text(mx + mw + 2, my + 4, "@  YOU", AMBER)
text(mx + mw + 2, my + 5, "■  ALLY", CYAN)
text(mx + mw + 2, my + 6, "■  ENEMY", RED)
text(mx + mw + 2, my + 8, "REFRESH", HEAD)
text(mx + mw + 2, my + 9, "0.15 s", BRIGHT)
text(mx + mw + 2, my + 11, "CELLS", HEAD)
text(mx + mw + 2, my + 12, "127x126", BRIGHT)

# --- roadmap -------------------------------------------------------------------
header(42, elite("roadmap // the oracle says"))
clear(4, 43, W - 5, 50)
phases = [
    ("PHASE 1", "MAP LIGHT + COLOUR", "▓▓▓▓▓▓▓▓░░", "fog, light, grass DONE // lighting.json + floor ids open"),
    ("PHASE 2", "MINIMAP", "▓▓▓░░░░░░░", "veil + rings DONE // portraits, trapezoid, sharp bake"),
    ("PHASE 3", "SHOP", "░░░░░░░░░░", "tiers, icon grid, recipes A+B>C, sell, undo"),
    ("PHASE 4", "HERO SELECT", "▓░░░░░░░░░", "lanes, team columns, battle spells, stone panels"),
    ("PHASE 5", "ARENA SET DRESSING", "░░░░░░░░░░", "cache cliffs, plank bridges, god castles"),
    ("PHASE 6", "HUD POLISH", "░░░░░░░░░░", "objectives timers, ally rows, slim bar"),
    ("PHASE 7", "POST-MATCH", "░░░░░░░░░░", "victory board, K/D/A, items, damage"),
]
for i, (p, name, bar, note) in enumerate(phases):
    y = 43 + i
    text(8, y, p, WHITE)
    text(18, y, name, CYAN)
    text(40, y, "[", DIM)
    for j, ch in enumerate(bar):
        put(41 + j, y, ch, BRIGHT if ch == "▓" else FAINT)
    text(51, y, "]", DIM)
    text(55, y, note, BRIGHT if "DONE" in note else MID)

# --- greets + footer -----------------------------------------------------------
header(52, elite("greetz"))
clear(4, 53, W - 5, 55)
center(53, "tHE OSRS CACHE  *  rUNELITE sOUNDeFFECTiD  *  wgpu  *  tHE rUST cREW  *  aLL bOT lANERS", MID)
center(54, "nOT AFFILIATED WITH JAGEX. cACHE ASSETS STAY OUT OF GIT. aLWAYS.", DIM)

fade_bar(57, 2, W - 3)
clear(20, 58, W - 21, 60)
center(58, "  y o u   t a k e   t h e   b l u e   p i l l  :  t h e   m a p   s t a y s   l i m e  ", DIM)
center(59, "  y o u   t a k e   t h e   r e d   p i l l  :  y o u   s e e   o n l y   w h a t   y o u r   t e a m   s e e s  ", HEAD)
center(60, "-=[ mObAsCaPe // 2026 // cAlL tHe cOnStRuCt: 555-0199 // 28.8k-56k // 24hRs ]=-", AMBER)

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
