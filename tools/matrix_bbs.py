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
# Optional per-cell background colours (.ans and .png; .txt has none).
bg = {}


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
            bg.pop((x, y), None)


def shade(x0, y0, x1, y1, colour):
    """Give cells a background colour."""
    for y in range(y0, y1 + 1):
        for x in range(x0, x1 + 1):
            if 0 <= x < W and 0 <= y < H:
                bg[(x, y)] = colour


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
STONE = (206, 192, 156)
ORANGE = (255, 152, 31)
YELLOW = (255, 255, 0)
TIP = (255, 255, 160)
INK_RED = (190, 40, 30)
PARCH = (150, 128, 96)
CODE_GRAD = [WHITE, WHITE, HEAD, HEAD, BRIGHT, BRIGHT, BRIGHT, BRIGHT, MID, MID, (0, 160, 40), (0, 160, 40)]
CODE_BG = [(0, 84 - 5 * r, 22 - r) for r in range(12)]
DENSE = "#@$%&8BWM0QXZ"
SPARSE = ".:'`,-"


def code_logo(y, word):
    """The word written in the rain itself (Cypher: "I don't even see the
    code"): dense glyphs inside the letters, sparse ones outside, so it
    reads in plain text too. Every column is a falling stream whose head
    whitens what it passes. Each block-font row is two mask rows, ▀ the top
    half and ▄ the bottom."""
    width = len(word) * 10 - 1
    x0 = (W - width) // 2
    rows = LOGO_ROWS * 2
    mask = set()
    for i, letter in enumerate(word):
        for r in range(LOGO_ROWS):
            for c, px in enumerate(LOGO[letter][r]):
                x = x0 + i * 10 + c
                if px in "█▀":
                    mask.add((x, 2 * r))
                if px in "█▄":
                    mask.add((x, 2 * r + 1))
    clear(x0 - 3, y, x0 + width + 2, y + rows - 1)
    lrng = random.Random(18)
    for x in range(x0 - 3, x0 + width + 3):
        head = lrng.randrange(-4, rows + 8)
        length = lrng.randrange(4, 12)
        for r in range(rows):
            k = head - r
            streak = 0 <= k < length
            if (x, r) in mask:
                col = WHITE if k == 0 else HEAD if streak and k < 3 else CODE_GRAD[r]
                put(x, y + r, lrng.choice(DENSE), col)
                shade(x, y + r, x, y + r, (0, 120, 30) if k == 0 else CODE_BG[r])
            elif 0 <= k < 3:
                # Outside the letters only the stream's head shows.
                put(x, y + r, ":" if k == 0 else lrng.choice(SPARSE), MID if k == 0 else DIM)
            elif lrng.random() < 0.03:
                put(x, y + r, ".", FAINT)
    return x0, width


GOLD = (255, 210, 90)
PURPLE = (190, 110, 255)
ICE = (150, 220, 255)
GREEN = (90, 230, 90)
FERN = (70, 170, 60)
WATER = (70, 150, 255)
SARA = (235, 240, 255)
ZAM = (255, 70, 50)
PILL_BLUE = (60, 120, 255)
PILL_RED = (235, 40, 40)
S_BG = (46, 39, 31)
CHIP_BG = (26, 22, 18)
CHIP_FG = (236, 226, 200)
S_RULE = (110, 92, 66)
# CastlePalette, exactly as in crates/client/src/scene.rs.
SARA_TOP = (234, 232, 223)
SARA_BAND = (64, 104, 196)
SARA_MERLON = (226, 184, 72)
SARA_POOL = (92, 146, 214)
ZAM_TOP = (104, 96, 98)
ZAM_DARK = (58, 53, 56)
ZAM_BAND = (168, 34, 28)
ZAM_POOL = (150, 34, 30)


def pills(x0, y0):
    """Morpheus' open hands: the blue pill left, the red pill right."""
    clear(x0 - 1, y0, x0 + 21, y0 + 11)
    for j in range(20):
        put(x0 + j, y0, "▄", MID)
        put(x0 + j, y0 + 8, "▄", MID)
    for k in range(1, 9):
        put(x0, y0 + k, "█", DIM)
        put(x0 + 19, y0 + k, "█", DIM)
    text(x0 + 2, y0 + 1, "> choose. ", DIM)
    text(x0 + 2, y0 + 2, "SARA", PILL_BLUE)
    text(x0 + 13, y0 + 2, "ZAM", PILL_RED)
    for k, row in enumerate([" ▄██▄ ", "▐████▌", " ▀██▀ "]):
        text(x0 + 2, y0 + 3 + k, row, PILL_BLUE)
        text(x0 + 12, y0 + 3 + k, row, PILL_RED)
    text(x0 + 8, y0 + 4, " or ", DIM)
    text(x0 + 2, y0 + 6, "blue", SARA_BAND)
    text(x0 + 7, y0 + 6, "gold", SARA_MERLON)
    text(x0 + 12, y0 + 6, "blood", ZAM_BAND)
    text(x0 + 2, y0 + 7, "> both. ok.", HEAD)
    text(x0 + 6, y0 + 9, "▄▄█████▄▄", DIM)
    text(x0 + 1, y0 + 10, "▄" * 18, MID)
    text(x0 + 1, y0 + 11, "▀" + "▪" * 16 + "▀", DIM)
    for j in range(16):
        put(x0 + 2 + j, y0 + 11, "▪", PILL_BLUE if j < 8 and j % 4 == 1 else PILL_RED if j % 4 == 1 else DIM)


LOADED = [
    "fountain 6750", "model 13149", "24 on a sheet", "mirror spawns",
    "team_at", "CastlePalette", "blue band", "gold merlons",
    "dark stone", "blood band", "red pool", "white perim",
]


def loaded_list(x0, y0):
    clear(x0 - 1, y0, x0 + 22, y0 + len(LOADED) - 1)
    for k, name in enumerate(LOADED):
        yy = y0 + k
        hot = name == "fountain 6750"
        text(x0, yy, name, WHITE if hot else CYAN)
        text(x0 + len(name), yy, "." * (18 - len(name)), FAINT)
        text(x0 + 19, yy, "ok", HEAD if hot else MID)


clear(2, 2, W - 3, 2)
lx, lw = code_logo(3, "MOBASCAPE")
pills(4, 3)
loaded_list(125, 3)
text(3, 2, "▐ tHE pILLS ▌", DIM)
text(W - 18, 2, "▐ lOADED ▌", DIM)
top = "  bLUE pILL: wHITE sTONE, gOLD.   rED pILL: dARK sTONE, bLOOD.   tHE fOUNTAIN iS rEAL.  "
assert len(top) <= 108, len(top)
center(2, top, MID)
fade_bar(1, 2, W - 3)
clear(lx - 3, 15, lx + lw + 2, 15)
ticker = " ◄◄ ICE-BLUE FOUNTAIN ◄◄ SARADOMIN BLUE ◄◄ ZAMORAK RED ◄◄ GOLD MERLONS ◄◄ BLOOD BAND"
run = (ticker * 3)[: lw]
for i, ch in enumerate(run):
    put(lx + i, 15, ch, ORANGE if ch == "◄" else AMBER)
center(16, "  ░▒▓  n O D E   1 8 :   t H E   p I L L S   //   b L U E   o R   r E D ,   y O U R   c A S T L E   c H O O S E S  ▓▒░  ", AMBER)
center(17, "  " + elite("you take the blue pill,") + "   " + elite("the castle stays white.") + "  ", WHITE)
fade_bar(18, 2, W - 3)

# --- release info -------------------------------------------------------------
header(19, elite("release info"))
clear(4, 20, W - 5, 22)
info = [
    ("rELEASE dATE", "2026-11-25", "cOMMITS", "9c3ab09 fountain + 1338896 colours  ->  sam/main"),
    ("tYPE", "pHASE 5.3: bASES. fOUNTAINS + gOD cOLOURS", "tESTS", "470 pass // clippy clean"),
    ("wAS", "tWO wHITE cASTLES aND a gREEN gLOW rING", "nOW", "sARADOMIN bLUE + gOLD // zAMORAK bLOOD"),
]
for i, (k1, v1, k2, v2) in enumerate(info):
    assert len(v1) <= 50 and 89 + len(v2) <= W - 6, (v1, v2)
    yy = 20 + i
    text(8, yy, f"{k1:.<16}", DIM)
    text(25, yy, v1, RED if k1 == "wAS" else BRIGHT)
    text(76, yy, f"{k2:.<12}", DIM)
    text(89, yy, v2, HEAD)

# --- left: the arena from above, two courtyards ---------------------------------
box(4, 24, 74, 46, elite("pills.dat // the arena from above"))
LX = 6
text(LX + 1, 26, "THE MAP", HEAD)
text(LX + 9, 26, "// 282x282 perimeter, courtyards in the corners", DIM)
MW, MH = 44, 13
MX, MY = LX + 3, 28
for y in range(MH):
    for x in range(MW):
        edge = x in (0, MW - 1) or y in (0, MH - 1)
        put(MX + x, MY + y, "█" if edge else " ", SARA_TOP)
# Diagonal river from top-left to bottom-right, three lanes.
for y in range(1, MH - 1):
    rx = 2 + int(y * (MW - 4) / MH)
    for dx in (0, 1):
        put(MX + rx + dx, MY + y, "≈", WATER)
        shade(MX + rx + dx, MY + y, MX + rx + dx, MY + y, (6, 26, 60))
for x in range(4, MW - 4):
    put(MX + x, MY + 1, "·", PARCH)
    put(MX + x, MY + MH - 2, "·", PARCH)
for y in range(1, MH - 1):
    put(MX + 1, MY + y, "·", PARCH)
    put(MX + MW - 2, MY + y, "·", PARCH)
for k in range(1, MH - 1):
    x = int(k * (MW - 1) / (MH - 1))
    y = MH - 1 - k
    if 0 < x < MW - 1:
        put(MX + x, MY + y, "·", PARCH)


def courtyard(x0, y0, w, h, pal_top, pal_band, pal_merlon, pool, label, label_col):
    for y in range(h):
        for x in range(w):
            edge = x in (0, w - 1) or y in (0, h - 1)
            if edge:
                put(x0 + x, y0 + y, "▀" if y == 0 else "█", pal_band if y in (0, h - 1) else pal_top)
                if y == 0 and x % 2 == 0:
                    put(x0 + x, y0 + y, "▄", pal_merlon)
            else:
                put(x0 + x, y0 + y, " ", BLACK)
                shade(x0 + x, y0 + y, x0 + x, y0 + y, (14, 20, 14))
    cx, cy = x0 + w // 2, y0 + h // 2
    put(cx, cy, "◉", pool)
    put(cx - 2, cy, "Ψ", label_col)
    put(cx + 2, cy, "Ψ", label_col)
    text(cx - len(label) // 2, y0 + h, label, label_col)


courtyard(MX, MY + MH - 6, 12, 6, SARA_TOP, SARA_BAND, SARA_MERLON, SARA_POOL, "", SARA)
courtyard(MX + MW - 12, MY, 12, 6, ZAM_TOP, ZAM_BAND, ZAM_DARK, ZAM_POOL, "", ZAM)
put(MX + 3, MY + MH - 3, "♦", ICE)
put(MX + MW - 4, MY + 2, "♦", ICE)
text(MX + MW + 3, MY + 0, "ZAMORAK", ZAM)
text(MX + MW + 3, MY + 1, "dark stone", ZAM_TOP)
text(MX + MW + 3, MY + 2, "blood band", ZAM_BAND)
text(MX + MW + 3, MY + 3, "red pool", ZAM_POOL)
text(MX + MW + 3, MY + 5, "♦ fountain", ICE)
text(MX + MW + 3, MY + 6, "◉ dais", SARA_POOL)
text(MX + MW + 3, MY + 7, "≈ river", WATER)
text(MX + MW + 3, MY + 9, "SARADOMIN", SARA)
text(MX + MW + 3, MY + 10, "white stone", SARA_TOP)
text(MX + MW + 3, MY + 11, "blue band", SARA_BAND)
text(MX + MW + 3, MY + 12, "gold merlons", SARA_MERLON)
text(LX + 1, 42, "─" * 66, FAINT)
term = [
    ("> ", "you take the blue pill: white stone, gold."),
    ("> ", "you take the red pill: the castle bleeds."),
    ("> ", "i'm offering the truth. nothing more."),
]
for k, (prompt, line) in enumerate(term):
    yy = 43 + k
    text(LX + 1, yy, prompt, DIM)
    text(LX + 3, yy, line, PILL_BLUE if k == 0 else PILL_RED if k == 1 else HEAD)
put(LX + 3 + len(term[-1][1]) + 1, 45, "█", BRIGHT)

# --- right: fountain + palette -------------------------------------------------
box(76, 24, W - 5, 46, elite("fountain.dat + palette.dat"))
RX = 79
text(RX, 26, "THE FOUNTAIN", HEAD)
text(RX + 13, 26, "// 24 cache fountains on one model-sheet", DIM)
text(RX, 27, "─" * 64, FAINT)
cands = [
    ("6750", "Large fountain", "white stone, blue water", "◄ CHOSEN", WHITE),
    ("2654", "Sinclair family", "teal pool, stone rim", "runner-up", MID),
    ("31625", "Fountain of Uhld", "dark rim, blue water", "too dark", DIM),
    ("39162", "Fountain", "red stand", "zamorak?", DIM),
]
for k, (oid, name, look, verdict, col) in enumerate(cands):
    yy = 28 + k
    text(RX, yy, "o" + oid, CYAN)
    text(RX + 7, yy, name, AMBER if k == 0 else STONE)
    text(RX + 25, yy, look, col if k == 0 else MID)
    text(RX + 52, yy, verdict, HEAD if k == 0 else col)
text(RX, 32, "at", CYAN)
text(RX + 3, 32, "blue (2713,3165) ◄─ mirror ─► red (2977,2925)", BRIGHT)
text(RX, 33, "x1.7  2x2 block  plain texture  inside the 900 heal range", MID)
text(RX, 34, "─" * 64, FAINT)
text(RX, 35, "CASTLEPALETTE", HEAD)
text(RX + 14, 35, "// team_at: base rect + 1 tile owns the piece", DIM)
rows = [
    ("SARADOMIN", [("top", SARA_TOP), ("band", SARA_BAND), ("merlon", SARA_MERLON), ("pool", SARA_POOL)], SARA),
    ("ZAMORAK", [("top", ZAM_TOP), ("band", ZAM_BAND), ("merlon", ZAM_DARK), ("pool", ZAM_POOL)], ZAM),
    ("NEUTRAL", [("top", SARA_TOP), ("band", SARA_TOP), ("merlon", SARA_TOP), ("pool", SARA_POOL)], STONE),
]
for k, (name, sw, col) in enumerate(rows):
    yy = 36 + k
    text(RX, yy, name, col)
    for j, (label, rgb) in enumerate(sw):
        x = RX + 11 + j * 13
        text(x, yy, "   ", BLACK)
        shade(x, yy, x + 2, yy, rgb)
        text(x + 4, yy, label, DIM)
text(RX, 40, "perimeter", CYAN)
text(RX + 10, 40, "open walls stay white; only courtyards take sides", MID)
text(RX, 41, "test", CYAN)
text(RX + 10, 41, "gates + dais own a team, mid-wall none, 0 != 1", MID)
text(RX, 44, "TESTS", HEAD)
text(RX + 7, 44, "470 pass  castle layout test checks every team", MID)
text(RX, 45, "SHOTS", HEAD)
text(RX + 7, 45, "--shots: both spawns, both courtyards, perimeter", MID)

# --- sysop's note ------------------------------------------------------------
header(48, elite("sysop's note // the pills"))
clear(4, 49, W - 5, 50)
open_ = "sTILL oPEN: gROUND mIST oVER uNSEEN aREAS (p5.7), mUSIC tOGGLE (p4.6), iTEM sETS (p3), oBJECTIVES pANEL (p6.3)."
refs = "CastlePalette::{SARADOMIN, ZAMORAK, NEUTRAL} // CastleSpec::team_at // props.place o6750 x2 // scene.rs"
assert len(open_) <= 142 and len(refs) <= 142, (len(open_), len(refs))
center(49, open_, HEAD)
center(50, refs, DIM)

# --- roadmap -------------------------------------------------------------------
header(51, elite("roadmap // docs/ROADMAPPLAN.md"))
clear(4, 52, W - 5, 56)
phases = [
    ("P1 LIGHT", 5), ("P2 MINIMAP", 6), ("P3 SHOP", 7), ("P4 SELECT", 9),
    ("P5 ARENA", 9), ("P6 HUD", 4), ("P7 POST", 0),
]
for i, (name, done) in enumerate(phases):
    x = 8 + (i % 4) * 35
    yy = 52 + (i // 4) * 2
    text(x, yy, f"{name:<11}", CYAN)
    text(x + 11, yy, "[", DIM)
    for j in range(10):
        put(x + 12 + j, yy, "▓" if j < done else "░", BRIGHT if j < done else FAINT)
    text(x + 22, yy, f"] {done * 10:>3}%", HEAD if done else FAINT)
nxt = ">> next: ground mist from cache smoke over unseen areas (P5.7, closes P5), then the objectives panel (P6.3)"
assert 8 + len(nxt) <= W - 6, len(nxt)
text(8, 56, nxt, MID)

fade_bar(57, 2, W - 3)
clear(10, 58, W - 11, 61)
greetz = "gREETZ: mORPHEUS (tHE pILLS) * sARADOMIN * zAMORAK * tHE lARGE fOUNTAIN * tHE sENTINELS * tANK * kOLODION"
assert len(greetz) <= 128, len(greetz)
center(58, greetz, MID)
center(59, "nOT AFFILIATED WITH JAGEX. cACHE ASSETS STAY OUT OF GIT. aLWAYS.", DIM)
center(60, "  b l u e   p i l l ,   r e d   p i l l .   t h e   f o u n t a i n   i s   r e a l .  ", HEAD)
foot = "-=[ mObAsCaPe // 2026 // nODE 18: tHE pILLS // 57.6k // sYSoP: rIFT // 2 fOUNTAINS, 2 gODS, 1 wHITE wALL ]=-"
assert len(foot) <= 128, len(foot)
center(61, foot, AMBER)


# --- outputs ------------------------------------------------------------------
with open(out + ".txt", "w") as f:
    for row in grid:
        f.write("".join(c for c, _ in row).rstrip() + "\n")

with open(out + ".ans", "w") as f:
    for y, row in enumerate(grid):
        last = None
        line = []
        for x, (ch, fg) in enumerate(row):
            back = bg.get((x, y))
            if (fg, back) != last:
                line.append(f"\x1b[38;2;{fg[0]};{fg[1]};{fg[2]}m")
                line.append(f"\x1b[48;2;{back[0]};{back[1]};{back[2]}m" if back else "\x1b[49m")
                last = (fg, back)
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
        px, py = 8 + x * cw, 8 + y * ch_h
        if (x, y) in bg:
            d.rectangle((px, py, px + cw - 1, py + ch_h - 1), fill=bg[(x, y)])
        if c == " ":
            continue
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
        elif c in "●•":
            # LED dots as real discs, nearly the two-column dot pitch wide.
            r = cw * (0.78 if c == "●" else 0.3)
            cx, cy = px + cw / 2, py + ch_h / 2
            d.ellipse((cx - r, cy - r, cx + r, cy + r), fill=fg)
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
