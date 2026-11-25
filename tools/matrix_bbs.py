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
    lrng = random.Random(19)
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
ICE = (150, 220, 255)
WATER = (70, 150, 255)
SARA = (235, 240, 255)
ZAM = (255, 70, 50)
MIST1 = (120, 135, 130)
MIST2 = (170, 185, 180)
MIST3 = (215, 225, 220)
MIST_BG = (22, 30, 28)
CLEAR_BG = (10, 38, 14)
CHIP_BG = (26, 22, 18)
CHIP_FG = (236, 226, 200)


def radar(x0, y0):
    """Team vision as a clear disc in the mist."""
    clear(x0 - 1, y0, x0 + 21, y0 + 11)
    for j in range(20):
        put(x0 + j, y0, "▄", MID)
        put(x0 + j, y0 + 8, "▄", MID)
    for k in range(1, 9):
        put(x0, y0 + k, "█", DIM)
        put(x0 + 19, y0 + k, "█", DIM)
    text(x0 + 2, y0 + 1, "> vision", DIM)
    cx, cy = x0 + 9.5, y0 + 4.5
    for yy in range(y0 + 2, y0 + 8):
        for xx in range(x0 + 1, x0 + 19):
            d = (((xx - cx) / 2.0) ** 2 + (yy - cy) ** 2) ** 0.5
            if d < 1.6:
                ch, col = " ", BRIGHT
                shade(xx, yy, xx, yy, CLEAR_BG)
            elif d < 2.4:
                ch, col = "░", MIST1
            elif d < 3.2:
                ch, col = "▒", MIST2
            else:
                ch, col = "▓", MIST3
            put(xx, yy, ch, col)
    put(x0 + 9, y0 + 4, "@", CYAN)
    put(x0 + 11, y0 + 5, "·", BRIGHT)
    text(x0 + 2, y0 + 7, " 48 puffs ", HEAD)
    text(x0 + 6, y0 + 9, "▄▄█████▄▄", DIM)
    text(x0 + 1, y0 + 10, "▄" * 18, MID)
    text(x0 + 1, y0 + 11, "▀" + "▪" * 16 + "▀", DIM)
    for j in range(16):
        put(x0 + 2 + j, y0 + 11, "▪", MIST3 if j % 4 == 1 else DIM)


LOADED = [
    "5.1 cliffs", "5.2 bridges", "5.3 fountains", "5.3 god colours",
    "5.4 surge splash", "5.5 flora", "5.6 tree fix", "5.7 mist",
    "spot 354", "used_fraction", "smoothstep fade", "PHASE 5",
]


def loaded_list(x0, y0):
    clear(x0 - 1, y0, x0 + 22, y0 + len(LOADED) - 1)
    for k, name in enumerate(LOADED):
        yy = y0 + k
        hot = name in ("5.7 mist", "PHASE 5")
        text(x0, yy, name, WHITE if hot else CYAN)
        text(x0 + len(name), yy, "." * (18 - len(name)), FAINT)
        text(x0 + 19, yy, "ok", HEAD if hot else MID)


clear(2, 2, W - 3, 2)
lx, lw = code_logo(3, "MOBASCAPE")
radar(4, 3)
loaded_list(125, 3)
text(3, 2, "▐ vISION ▌", DIM)
text(W - 18, 2, "▐ pHASE 5 ▌", DIM)
top = "  wHAT yOU cAN'T sEE iS mIST.   wHAT yOUR tEAM sEES iS rEAL.   pHASE 5: dONE.  "
assert len(top) <= 108, len(top)
center(2, top, MID)
fade_bar(1, 2, W - 3)
clear(lx - 3, 15, lx + lw + 2, 15)
ticker = " ◄◄ GROUND MIST ◄◄ SPOT 354 ◄◄ SOFT EDGE OF VISION ◄◄ PHASE 5 COMPLETE ◄◄ 7 OF 7"
run = (ticker * 3)[: lw]
for i, ch in enumerate(run):
    put(lx + i, 15, ch, ORANGE if ch == "◄" else AMBER)
center(16, "  ░▒▓  n O D E   1 9 :   t H E   d E S E R T   o F   t H E   r E A L   //   p H A S E   5   c O M P L E T E  ▓▒░  ", AMBER)
center(17, "  " + elite("welcome") + "   " + elite("to the desert of the real.") + "  ", WHITE)
fade_bar(18, 2, W - 3)

# --- release info -------------------------------------------------------------
header(19, elite("release info"))
clear(4, 20, W - 5, 22)
info = [
    ("rELEASE dATE", "2026-11-25", "cOMMITS", "48df811 ground mist  ->  sam/main"),
    ("tYPE", "pHASE 5.7: gROUND mIST // aRENA pHASE cLOSED", "tESTS", "473 pass (+3 mist) // clippy clean"),
    ("wAS", "uNSEEN gROUND wAS oNLY a dARKER fLOOR", "nOW", "low smoke banks fade with team vision"),
]
for i, (k1, v1, k2, v2) in enumerate(info):
    assert len(v1) <= 50 and 89 + len(v2) <= W - 6, (v1, v2)
    yy = 20 + i
    text(8, yy, f"{k1:.<16}", DIM)
    text(25, yy, v1, RED if k1 == "wAS" else BRIGHT)
    text(76, yy, f"{k2:.<12}", DIM)
    text(89, yy, v2, HEAD)

# --- left: the mist field ------------------------------------------------------
box(4, 24, 74, 46, elite("mist.dat // the desert of the real"))
LX = 6
text(LX + 1, 26, "THE FOG OF WAR", HEAD)
text(LX + 16, 26, "// mist where no ally sees, clear where one does", DIM)
FW, FH = 64, 12
FX, FY = LX + 1, 28
mrng = random.Random(19)
eyes = [(16, 6, 9.0), (40, 4, 7.0), (52, 9, 5.5)]  # hero, ally tower, ally minion
for y in range(FH):
    for x in range(FW):
        best = min((((x - ex) / 2.0) ** 2 + (y - ey) ** 2) ** 0.5 / (r / 2.0) for ex, ey, r in eyes)
        n = mrng.random() * 0.35
        v = best + n
        xx, yy = FX + x, FY + y
        if v < 0.85:
            ch = mrng.choice("  ,.'  ") if mrng.random() < 0.5 else " "
            put(xx, yy, ch, MID)
            shade(xx, yy, xx, yy, CLEAR_BG)
        elif v < 1.15:
            put(xx, yy, "░", MIST1)
            shade(xx, yy, xx, yy, MIST_BG)
        elif v < 1.5:
            put(xx, yy, "▒", MIST2)
            shade(xx, yy, xx, yy, MIST_BG)
        else:
            put(xx, yy, "▓" if mrng.random() < 0.7 else "▒", MIST3 if mrng.random() < 0.6 else MIST2)
            shade(xx, yy, xx, yy, MIST_BG)
put(FX + 16, FY + 6, "@", CYAN)
text(FX + 18, FY + 6, "you", CYAN)
put(FX + 40, FY + 4, "▲", SARA)
text(FX + 42, FY + 4, "tower", SARA)
put(FX + 52, FY + 9, "·", WATER)
text(FX + 54, FY + 9, "imp", WATER)
put(FX + 5, FY + 2, "?", ZAM)
put(FX + 60, FY + 1, "?", ZAM)
text(LX + 1, 41, "─" * 66, FAINT)
legend = [("▓", MIST3, "full mist"), ("▒", MIST2, "fading"), ("░", MIST1, "edge"), (" ", BRIGHT, "seen")]
for k, (ch, col, label) in enumerate(legend):
    x = LX + 1 + k * 17
    put(x, 42, ch, col)
    if ch == " ":
        shade(x, 42, x, 42, CLEAR_BG)
    text(x + 2, 42, label, col if ch != " " else BRIGHT)
term = [
    ("> ", "what is real? how do you define real?"),
    ("> ", "what your team can see. the rest is smoke."),
]
for k, (prompt, line) in enumerate(term):
    yy = 44 + k
    text(LX + 1, yy, prompt, DIM)
    text(LX + 3, yy, line, BRIGHT if k == 0 else HEAD)
put(LX + 3 + len(term[-1][1]) + 1, 45, "█", BRIGHT)

# --- right: mist internals + phase 5 ledger -------------------------------------
box(76, 24, W - 5, 46, elite("mist.rs + phase5.dat"))
RX = 79
text(RX, 26, "SPOT 354", HEAD)
text(RX + 9, 26, "// model 6288 · 27 frames · 1.88 s · all translucent", DIM)
text(RX, 27, "─" * 64, FAINT)
params = [
    ("grid", "seeded 5-tile jitter inside the walls, ~3,000 puffs"),
    ("shape", "x5-8 wide, flattened 0.28: a bank, not a ball"),
    ("fade", "1.4/s toward mist while unseen, smoothstep edge"),
    ("draw", "nearest 48 within 22 tiles, additive glow 0.06"),
    ("budget", "stops at half the fx vertices: spells always fit"),
]
for k, (key, line) in enumerate(params):
    yy = 28 + k
    assert len(line) <= 54, line
    text(RX, yy, key, AMBER)
    text(RX + 8, yy, line, MID)
text(RX, 33, "first pass", RED)
text(RX + 11, 33, "glow 0.32: round white balls ──► 0.06", (200, 90, 80))
text(RX, 34, "─" * 64, FAINT)
text(RX, 35, "PHASE 5 // ARENA SET DRESSING", HEAD)
text(RX + 30, 35, "// 7 of 7, all cache objects", DIM)
ledger = [
    ("5.1", "cliffs", "~410 rocks 19997/818/819"),
    ("5.2", "bridges", "plank piece 699 x3"),
    ("5.3", "bases", "o6750 fountains, god colours"),
    ("5.4", "towers", "orbs 1526-8, surge 1460/63/66"),
    ("5.5", "flora", "~1,950 ferns + stumps"),
    ("5.6", "trees", "dire canopy keeps its leaves"),
    ("5.7", "mist", "spot 354 over unseen ground"),
]
for k, (num, name, what) in enumerate(ledger):
    yy = 36 + k
    hot = num == "5.7"
    text(RX, yy, f" {num} ", CHIP_FG)
    shade(RX, yy, RX + 4, yy, CHIP_BG)
    text(RX + 6, yy, name, WHITE if hot else AMBER)
    text(RX + 15, yy, what, BRIGHT if hot else MID)
    text(RX + 47, yy, "[▓▓▓▓]", BRIGHT)
    text(RX + 54, yy, "done", HEAD if hot else MID)
text(RX, 44, "TESTS", HEAD)
text(RX + 7, 44, "473 pass  +3: seeded, fades w/o pop, nearest only", MID)
text(RX, 45, "BENCH", HEAD)
text(RX + 7, 45, "77.8 fps at spawn (p95 13.8 ms) // 680M target 60", MID)

# --- sysop's note ------------------------------------------------------------
header(48, elite("sysop's note // the desert of the real"))
clear(4, 49, W - 5, 50)
open_ = "sTILL oPEN: oBJECTIVES pANEL (p6.3), mUSIC tOGGLE (p4.6), iTEM sETS (p3), tERRAIN pALETTE (p1.3), mINIMAP cAMP iCONS (p2.5)."
refs = "animated::mist::{Mist, MistPuff, MIST_SPOT} // FxMesh::used_fraction // kit::spot::VERIFIED += 354"
assert len(open_) <= 142 and len(refs) <= 142, (len(open_), len(refs))
center(49, open_, HEAD)
center(50, refs, DIM)

# --- roadmap -------------------------------------------------------------------
header(51, elite("roadmap // docs/ROADMAPPLAN.md"))
clear(4, 52, W - 5, 56)
phases = [
    ("P1 LIGHT", 5), ("P2 MINIMAP", 6), ("P3 SHOP", 7), ("P4 SELECT", 9),
    ("P5 ARENA", 10), ("P6 HUD", 4), ("P7 POST", 0),
]
for i, (name, done) in enumerate(phases):
    x = 8 + (i % 4) * 35
    yy = 52 + (i // 4) * 2
    text(x, yy, f"{name:<11}", CYAN)
    text(x + 11, yy, "[", DIM)
    for j in range(10):
        put(x + 12 + j, yy, "▓" if j < done else "░", (WHITE if done == 10 else BRIGHT) if j < done else FAINT)
    text(x + 22, yy, f"] {done * 10:>3}%", HEAD if done else FAINT)
text(8 + 3 * 35, 54, "◄◄ P5 SEALED", AMBER)
nxt = ">> next: the objectives panel with named camp timers (P6.3), then the death screen and scoreboard (P7)"
assert 8 + len(nxt) <= W - 6, len(nxt)
text(8, 56, nxt, MID)

fade_bar(57, 2, W - 3)
clear(10, 58, W - 11, 61)
greetz = "gREETZ: mORPHEUS (tHE dESERT oF tHE rEAL) * tHE sMOKE cLOUD (354) * tHE sENTINELS * sARADOMIN * zAMORAK * tANK"
assert len(greetz) <= 128, len(greetz)
center(58, greetz, MID)
center(59, "nOT AFFILIATED WITH JAGEX. cACHE ASSETS STAY OUT OF GIT. aLWAYS.", DIM)
center(60, "  w e l c o m e   t o   t h e   d e s e r t   o f   t h e   r e a l .   p h a s e   5 :   d o n e .  ", HEAD)
foot = "-=[ mObAsCaPe // 2026 // nODE 19: tHE dESERT oF tHE rEAL // 57.6k // sYSoP: rIFT // 7 oF 7, 1 cLOUD ]=-"
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
