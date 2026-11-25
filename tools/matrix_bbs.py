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
    lrng = random.Random(20)
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
BLOOD = (200, 30, 40)
BLOOD_BG = (50, 6, 10)
VIOLET = (170, 90, 255)
VIOLET_BG = (26, 8, 44)
SWAMP = (90, 140, 70)
SWAMP_BG = (10, 22, 12)
STONE2 = (150, 145, 150)
IRON = (120, 110, 115)
LAMP = (120, 255, 140)
CANDLE = (255, 220, 120)
CHIP_BG = (26, 22, 18)
CHIP_FG = (236, 226, 200)


def chateau(x0, y0):
    """The Merovingian's door: a gothic arch with red glass."""
    clear(x0 - 1, y0, x0 + 21, y0 + 11)
    for j in range(20):
        put(x0 + j, y0, "▄", MID)
        put(x0 + j, y0 + 8, "▄", MID)
    for k in range(1, 9):
        put(x0, y0 + k, "█", DIM)
        put(x0 + 19, y0 + k, "█", DIM)
    text(x0 + 2, y0 + 1, "> chateau", DIM)
    art = ["  ▲ ▲ ▲ ▲  ", " ▐█▀▀█▀▀█▌ ", " ▐▌▓▌ ▐▓▐▌ ", " ▐▌▓▌ ▐▓▐▌ ", " ▀▀▀▀▀▀▀▀▀ "]
    for k, row in enumerate(art):
        for j, ch in enumerate(row):
            col = BLOOD if ch == "▓" else IRON if ch in "▲" else STONE2
            if ch != " ":
                put(x0 + 4 + j, y0 + 2 + k, ch, col)
    text(x0 + 2, y0 + 7, "> enter. ok.", HEAD)
    text(x0 + 6, y0 + 9, "▄▄█████▄▄", DIM)
    text(x0 + 1, y0 + 10, "▄" * 18, MID)
    text(x0 + 1, y0 + 11, "▀" + "▪" * 16 + "▀", DIM)
    for j in range(16):
        put(x0 + 2 + j, y0 + 11, "▪", BLOOD if j % 4 == 1 else VIOLET if j % 4 == 3 else DIM)


LOADED = [
    "vampyre statue", "darkmeyer rails", "swampy lamps", "pool of blood",
    "vampyre tombs", "mort myre reeds", "dark altar", "candle ring",
    "chaos altar", "crystal ball", "biome scatter", "mist: removed",
]


def loaded_list(x0, y0):
    clear(x0 - 1, y0, x0 + 22, y0 + len(LOADED) - 1)
    for k, name in enumerate(LOADED):
        yy = y0 + k
        gone = name.startswith("mist")
        hot = name in ("vampyre statue", "dark altar")
        text(x0, yy, name, RED if gone else WHITE if hot else CYAN)
        text(x0 + len(name), yy, "." * (18 - len(name)), FAINT)
        text(x0 + 19, yy, "--" if gone else "ok", RED if gone else HEAD if hot else MID)


clear(2, 2, W - 3, 2)
lx, lw = code_logo(3, "MOBASCAPE")
chateau(4, 3)
loaded_list(125, 3)
text(3, 2, "▐ cHATEAU ▌", DIM)
text(W - 18, 2, "▐ lOADED ▌", DIM)
top = "  tHE mIST iS gONE.   tHE vAMPYRES sTAYED.   oNLY cACHE oBJECTS, eVERY oNE oF tHEM.  "
assert len(top) <= 108, len(top)
center(2, top, MID)
fade_bar(1, 2, W - 3)
clear(lx - 3, 15, lx + lw + 2, 15)
ticker = " ◄◄ VAMPYRE STATUE ◄◄ DARKMEYER RAILINGS ◄◄ POOL OF BLOOD ◄◄ DARK ALTAR ◄◄ MORT MYRE"
run = (ticker * 3)[: lw]
for i, ch in enumerate(run):
    put(lx + i, 15, ch, ORANGE if ch == "◄" else AMBER)
center(16, "  ░▒▓  n O D E   2 0 :   t H E   c H A T E A U   //   v A M P Y R E S   a N D   d A R K   m A G I C  ▓▒░  ", AMBER)
center(17, "  " + elite("you see, there is only one constant:") + "   " + elite("causality.") + "  ", WHITE)
fade_bar(18, 2, W - 3)

# --- release info -------------------------------------------------------------
header(19, elite("release info"))
clear(4, 20, W - 5, 22)
info = [
    ("rELEASE dATE", "2026-11-25", "cOMMITS", "b261a6d revert · 9ea525d · 5f4b4a0 · dfa27f9"),
    ("tYPE", "tHEMED pOCKETS: vAMPIRE sWAMP + dARK mAGIC", "tESTS", "472 pass (+2) // clippy clean"),
    ("wAS", "gENERIC jUNGLE wITH dEAD tREES; a mIST lAYER", "nOW", "vampyre plaza + ritual ground, mist gone"),
]
for i, (k1, v1, k2, v2) in enumerate(info):
    assert len(v1) <= 50 and 89 + len(v2) <= W - 6, (v1, v2)
    yy = 20 + i
    text(8, yy, f"{k1:.<16}", DIM)
    text(25, yy, v1, RED if k1 == "wAS" else BRIGHT)
    text(76, yy, f"{k2:.<12}", DIM)
    text(89, yy, v2, HEAD)

# --- left: vampire swamp ---------------------------------------------------------
box(4, 24, 74, 46, elite("vampire.dat // mort myre, region 54,52"))
LX = 6
text(LX + 1, 26, "THE VAMPYRE PLAZA", HEAD)
text(LX + 19, 26, "// darkmeyer railings round the statue", DIM)
PX, PY = LX + 2, 28
plaza = [
    "  ,  ¥   ,    ¥   ,   ¥  ,    ¥  ,  ",
    " ¥   ┼ ╥ ╥ ╥ ╥ ┼   ,     ≈≈≈≈≈   ¥ ",
    "  ,  ╫  ¤   ¤  ╫    ¥   ≈≈≈≈≈≈≈    ",
    " ¥   ╫    Ψ    ╫  ,      ≈≈≈≈≈   , ",
    "  ,  ╫  ¤   ¤        †    ,   ♠    ",
    " ¥   ┼ ╨ ╨ ╨     ¥     ,    ¥    , ",
    "   ▄▄▄▄▄  ▄▄▄▄▄   ,   †   ,    ¥   ",
    "   █▓▓▓█  █▓▓▓█     ,    ¥   ,     ",
]
for r, row in enumerate(plaza):
    for c, ch in enumerate(row):
        xx, yy = PX + c, PY + r
        colour = {
            "Ψ": WHITE, "╥": IRON, "╨": IRON, "╫": IRON, "┼": BLOOD,
            "¤": LAMP, "≈": BLOOD, "¥": SWAMP, ",": (110, 90, 60),
            "†": STONE2, "♠": (170, 150, 110), "▄": STONE2, "█": STONE2, "▓": (70, 65, 70),
        }.get(ch, DIM)
        put(xx, yy, ch, colour)
        if ch == "≈":
            shade(xx, yy, xx, yy, BLOOD_BG)
        elif ch != " " or True:
            shade(xx, yy, xx, yy, SWAMP_BG)
leg = [("Ψ", WHITE, "vampyre statue 17643 x2.6"), ("╫", IRON, "railings 40294/5 x2.0"),
       ("¤", LAMP, "swampy lamps 32611"), ("≈", BLOOD, "pool of blood 35712"),
       ("█", STONE2, "vampyre tombs 6350/7"), ("†", STONE2, "graves 46700, 1363..")]
for k, (ch, col, label) in enumerate(leg):
    yy = PY + k
    put(PX + 38, yy, ch, col)
    text(PX + 40, yy, label, MID)
text(PX + 38, PY + 6, "¥ reeds 3549-53", SWAMP)
text(PX + 38, PY + 7, "♠ mushrooms 4092-4", (170, 150, 110))
text(LX + 1, 37, "─" * 66, FAINT)
notes = [
    ("plaza", "open on the lane side; four lamps; statue faces the road"),
    ("swamp", "28 bubble pools, 150 reeds, rotting trees, logs, roots"),
    ("floor", "dry grass layer, greyer tint"),
]
for k, (key, line) in enumerate(notes):
    text(LX + 1, 38 + k, key, AMBER)
    text(LX + 8, 38 + k, line, MID)
term = [
    ("> ", "i have survived your predecessors."),
    ("> ", "and i will survive you. (the statue, probably)"),
]
for k, (prompt, line) in enumerate(term):
    yy = 43 + k
    text(LX + 1, yy, prompt, DIM)
    text(LX + 3, yy, line, BRIGHT if k == 0 else HEAD)
put(LX + 3 + len(term[-1][1]) + 1, 44, "█", BRIGHT)

# --- right: dark magic + schema --------------------------------------------------
box(76, 24, W - 5, 46, elite("darkmagic.dat // draynor manor, 48,52"))
RX = 79
text(RX, 26, "THE RITUAL GROUND", HEAD)
text(RX + 18, 26, "// the arceuus dark altar in candles", DIM)
ring = [
    "      i     i      ",
    "   i             i ",
    "        ▄█▄        ",
    "  i    ▐▓█▓▌    i  ",
    "        ▀▀▀        ",
    "   i             i ",
    "      i     i      ",
]
for r, row in enumerate(ring):
    for c, ch in enumerate(row):
        xx, yy = RX + c, 28 + r
        col = CANDLE if ch == "i" else VIOLET if ch == "▓" else (60, 40, 90)
        put(xx, yy, ch, col)
        shade(xx, yy, xx, yy, VIOLET_BG)
items = [
    ("30837", "dark altar x0.95 (x1.8 dwarfed all)"),
    ("22936", "eight candle stands, r 4.5"),
    ("1216", "chaos altar · 27778 + 35298 zamorak"),
    ("12580", "symbol of zamorak banners x2"),
    ("1190", "crystal ball · 46675 great cauldron"),
    ("16677", "ceremonial brazier ring"),
    ("scatter", "skulls, bones, candles, braziers"),
]
for k, (oid, what) in enumerate(items):
    text(RX + 21, 28 + k, oid, CYAN)
    text(RX + 29, 28 + k, what, MID)
text(RX, 35, "budget", AMBER)
text(RX + 7, 35, "draynor transplant 110 ──► 55: trees frame, not bury", MID)
text(RX, 36, "─" * 64, FAINT)
text(RX, 37, "MAP FORMAT", HEAD)
text(RX + 11, 37, "// two new fields, nothing old changes", DIM)
text(RX, 38, " scatter.biome ", CHIP_FG)
shade(RX, 38, RX + 14, 38, CHIP_BG)
text(RX + 16, 38, "drop only inside that pocket, rect optional", MID)
text(RX, 39, " place.plaza   ", CHIP_FG)
shade(RX, 39, RX + 14, 39, CHIP_BG)
text(RX + 16, 39, "clear radius, default 6; railings use 0", MID)
text(RX, 40, "mist", RED)
text(RX + 7, 40, "reverted (48df811): fog of war + height fog unchanged", (200, 90, 80))
text(RX, 44, "TESTS", HEAD)
text(RX + 7, 44, "472 pass  every prop model decodes from the cache", MID)
text(RX, 45, "SHOTS", HEAD)
text(RX + 7, 45, "--shots: plaza + ritual, top / low / wide", MID)

# --- sysop's note ------------------------------------------------------------
header(48, elite("sysop's note // the chateau"))
clear(4, 49, W - 5, 50)
open_ = "sTILL oPEN: oGRE rUINS, gOBLIN cAMP, hUMAN cITY + kEEP, eLVEN lANDS, gNOME sTRONGHOLD; oBJECTIVES pANEL (p6.3)."
refs = "props::ScatterJson.biome // PlaceJson.plaza // MapLayout::biome_rect // dire_vampire + dire_dark_magic"
assert len(open_) <= 142 and len(refs) <= 142, (len(open_), len(refs))
center(49, open_, HEAD)
center(50, refs, DIM)

# --- roadmap -------------------------------------------------------------------
header(51, elite("roadmap // docs/ROADMAPPLAN.md"))
clear(4, 52, W - 5, 56)
phases = [
    ("P1 LIGHT", 5), ("P2 MINIMAP", 6), ("P3 SHOP", 7), ("P4 SELECT", 9),
    ("P5 ARENA", 9), ("POCKETS", 2), ("P6 HUD", 4), ("P7 POST", 0),
]
for i, (name, done) in enumerate(phases):
    x = 8 + (i % 4) * 35
    yy = 52 + (i // 4) * 2
    text(x, yy, f"{name:<11}", CYAN)
    text(x + 11, yy, "[", DIM)
    for j in range(10):
        put(x + 12 + j, yy, "▓" if j < done else "░", BRIGHT if j < done else FAINT)
    text(x + 22, yy, f"] {done * 10:>3}%", HEAD if done else FAINT)
nxt = ">> next: the remaining six pockets from cache objects (ogre, goblin, human x2, elven, gnome), then the objectives panel (P6.3)"
assert 8 + len(nxt) <= W - 6, len(nxt)
text(8, 56, nxt, MID)

fade_bar(57, 2, W - 3)
clear(10, 58, W - 11, 61)
greetz = "gREETZ: tHE mEROVINGIAN (tHE cHATEAU) * pERSEPHONE * cOUNT dRAYNOR * tHE vAMPYRE sTATUE * mORT mYRE * dARKMEYER"
assert len(greetz) <= 128, len(greetz)
center(58, greetz, MID)
center(59, "nOT AFFILIATED WITH JAGEX. cACHE ASSETS STAY OUT OF GIT. aLWAYS.", DIM)
center(60, "  c a u s a l i t y .   a c t i o n ,   r e a c t i o n .   c a u s e   a n d   e f f e c t .  ", HEAD)
foot = "-=[ mObAsCaPe // 2026 // nODE 20: tHE cHATEAU // 57.6k // sYSoP: rIFT // 2 pOCKETS, 0 mIST, 100% cACHE ]=-"
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
