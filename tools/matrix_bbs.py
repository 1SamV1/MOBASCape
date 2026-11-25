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
    lrng = random.Random(15)
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


def operator_crt(x0, y0):
    """Tank's console loading the popular build: Cerberus's Tank list,
    bars by price (torva 6500 ... guardian boots 3000)."""
    clear(x0 - 1, y0, x0 + 21, y0 + 11)
    for j in range(20):
        put(x0 + j, y0, "▄", MID)
        put(x0 + j, y0 + 8, "▄", MID)
    for k in range(1, 9):
        put(x0, y0 + k, "█", DIM)
        put(x0 + 19, y0 + k, "█", DIM)
    text(x0 + 2, y0 + 1, "> load ", DIM)
    text(x0 + 9, y0 + 1, "TANK", AMBER)
    build = [("torva", 9), ("bandos", 6), ("dragon", 5), ("guard.", 4), ("ring", 5)]
    for k, (name, n) in enumerate(build):
        text(x0 + 2, y0 + 2 + k, f"{name:<7}", CYAN)
        text(x0 + 9, y0 + 2 + k, "▓" * n, BRIGHT if k == 0 else MID)
    text(x0 + 2, y0 + 7, "> 5 items. ok.", HEAD)
    text(x0 + 6, y0 + 9, "▄▄█████▄▄", DIM)
    text(x0 + 1, y0 + 10, "▄" * 18, MID)
    text(x0 + 1, y0 + 11, "▀" + "▪" * 16 + "▀", DIM)
    for j in range(16):
        put(x0 + 2 + j, y0 + 11, "▪", GOLD if j % 5 == 2 else DIM)


GOLD = (255, 210, 90)
PASSIVES = [
    "frozen breath", "sticky tongue", "thick hide", "three heads", "bandos' war cry",
    "second form", "shapeshifter", "demonic poison", "venom fangs", "dragonfire splash",
    "old wisdom", "swift blessing",
]


def programs(x0, y0):
    """The twelve kit passives, loaded like training programs."""
    clear(x0 - 1, y0, x0 + 22, y0 + len(PASSIVES) - 1)
    for k, name in enumerate(PASSIVES):
        yy = y0 + k
        hot = name == "three heads"
        text(x0, yy, name, WHITE if hot else CYAN)
        text(x0 + len(name), yy, "." * (18 - len(name)), FAINT)
        text(x0 + 19, yy, "ok", HEAD if hot else MID)


clear(2, 2, W - 3, 2)
lx, lw = code_logo(3, "MOBASCAPE")
operator_crt(4, 3)
programs(125, 3)
text(3, 2, "▐ oPERATOR ▌", DIM)
text(W - 17, 2, "▐ pROGRAMS ▌", DIM)
center(2, "  oPERATOR, i nEED a bUILD fOR a hELLHOUND tANK.   lOADING...   dONE.  ", MID)
fade_bar(1, 2, W - 3)
clear(lx - 3, 15, lx + lw + 2, 15)
ticker = " ◄◄ POPULAR BUILD ◄◄ PASSIVE ◄◄ Q W E R ◄◄ THE PALE YELLOW BOX ◄◄ 1024x600 FITS"
run = (ticker * 3)[: lw]
for i, ch in enumerate(run):
    put(lx + i, 15, ch, ORANGE if ch == "◄" else AMBER)
center(16, "  ░▒▓  n O D E   1 5 :   t H E   o P E R A T O R   //   t A N K ,   l O A D   t H E   b U I L D  ▓▒░  ", AMBER)
center(17, "  " + elite("i know three heads.") + "   " + elite("show me.") + "  ", WHITE)
fade_bar(18, 2, W - 3)

# --- release info -------------------------------------------------------------
header(19, elite("release info"))
clear(4, 20, W - 5, 22)
info = [
    ("rELEASE dATE", "2026-11-24", "cOMMIT", "f079e93  ->  sam/main"),
    ("tYPE", "hERO sELECT: pASSIVE rOW + pOPULAR bUILD", "tESTS", "433 pass // clippy clean"),
    ("wAS", "1024x600: rOSTER, tAGS aND r oFF tHE pANEL", "nOW", "rows give way; the hover box keeps the rest"),
]
for i, (k1, v1, k2, v2) in enumerate(info):
    yy = 20 + i
    text(8, yy, f"{k1:.<16}", DIM)
    text(25, yy, v1, RED if k1 == "wAS" else BRIGHT)
    text(76, yy, f"{k2:.<12}", DIM)
    text(89, yy, v2, HEAD)

# --- left: the parchment --------------------------------------------------------
box(4, 24, 74, 46, elite("parchment.dat // 1280x720"))
LX = 6
IW = 56


def wall(left, middle, right, fill="═"):
    run = IW + 4 - len(left) - len(middle) - len(right)
    half = run // 2
    return " " + left + fill * half + middle + fill * (run - half) + right + " "


def row(inner):
    assert len(inner) <= IW, (len(inner), inner)
    return " ║ " + inner.ljust(IW) + " ║ "


P_BG = (210, 190, 150)
P_INK = (62, 46, 30)
P_RED = (150, 28, 18)
P_BLUE = (40, 60, 150)
P_RULE = (120, 96, 60)
CHIP_BG = (26, 22, 18)
CHIP_FG = (236, 226, 200)
for j, ch in enumerate(wall("╔═", "[1017 parchment]", "═╗")):
    if ch != " ":
        put(LX + j, 26, ch, STONE if ch in "╔╗═" else DIM)
for j, ch in enumerate(wall("╚═", "", "═╝")):
    if ch != " ":
        put(LX + j, 39, ch, STONE)
PX = LX + 3
for k in range(1, 13):
    put(LX + 1, 26 + k, "║", STONE)
    put(LX + IW + 4, 26 + k, "║", STONE)
shade(LX + 2, 27, LX + IW + 3, 38, P_BG)


def chip(x, y, label):
    text(x, y, label, CHIP_FG)
    shade(x, y, x + len(label) - 1, y, CHIP_BG)


def ability(y, key, name, cooldowns, line):
    chip(PX, y, f"  {key}  ")
    put(PX + 6, y, "✶", (200, 90, 20))
    text(PX + 10, y, name, P_RED)
    text(PX + 26, y, cooldowns, P_BLUE)
    if line:
        text(PX + 10, y + 1, line, P_INK)


text(PX, 27, "Popular build", P_RED)
text(PX + 15, 27, "Tank", P_BLUE)
for i, colour in enumerate([(110, 110, 125), (120, 90, 50), (170, 30, 25), (70, 60, 50), (200, 60, 40)]):
    text(PX + 22 + i * 4, 27, "[", P_RULE)
    put(PX + 23 + i * 4, 27, "■", colour)
    text(PX + 24 + i * 4, 27, "]", P_RULE)
text(PX, 28, "─" * IW, P_RULE)
chip(PX, 29, " Passive ")
text(PX + 10, 29, "Three heads", P_RED)
put(PX + 22, 29, "↖", (20, 20, 20))
text(PX + 10, 30, "Every third basic attack also hits...", P_INK)
ability(31, "Q", "Triple Bite", "6 / 5.5 / 5 / 4.5 s", "Three quick bites in front; the last...")
ability(33, "W", "Lava Pools", "10 / 9.5 / 9 / 8.5 s", "Spits three pools of lava that burn...")
ability(35, "E", "Summoned Souls", "12 / 11 / 10 / 9 s", "A ranger, mage and warrior soul fly...")
ability(37, "R", "Hellhound Howl", "65 / 58 / 50 s", "Every nearby enemy flees in fear.")

# The hover box, drawn last over whatever is under it: pale yellow, black
# edge and text, as the client draws it.
tip = ["Every third basic attack also hits", "enemies within 260 of the target", "for 70% damage."]
tx, ty, tw = PX + 14, 30, 38
shade(tx - 1, ty, tx + tw, ty + len(tip) + 1, TIP)
for j in range(tw):
    put(tx + j, ty, "─", BLACK)
    put(tx + j, ty + len(tip) + 1, "─", BLACK)
for k in range(len(tip) + 2):
    put(tx - 1, ty + k, "│" if 0 < k < len(tip) + 1 else ("┌" if k == 0 else "└"), BLACK)
    put(tx + tw, ty + k, "│" if 0 < k < len(tip) + 1 else ("┐" if k == 0 else "┘"), BLACK)
for k, line in enumerate(tip):
    text(tx, ty + 1 + k, " " + line.ljust(tw - 1), BLACK)

text(LX + 1, 40, "THE SENTENCE", HEAD)
text(LX + 14, 40, "// Passive::describe, from the numbers", DIM)
src = '{"type":"nth_attack","n":3,"radius":260,"damage":0.7}'
text(LX + 2, 41, src, CYAN)
for i, ch in enumerate(src):
    if ch.isdigit() or ch == ".":
        put(LX + 2 + i, 41, ch, AMBER)
text(LX + 2, 42, "└─describe()─►", DIM)
text(LX + 17, 42, '"Every third ... within 260 ... for 70%."', BRIGHT)

term = [
    ("> ", "tank, load the build."),
    ("> ", "i know three heads."),
    ("> ", "show me."),
]
for k, (prompt, line) in enumerate(term):
    yy = 43 + k
    text(LX + 1, yy, prompt, DIM)
    text(LX + 3, yy, line, BRIGHT if k < 2 else HEAD)
put(LX + 3 + len(term[-1][1]) + 1, 45, "█", BRIGHT)
text(LX + 30, 43, "12 kit passives named", MID)
text(LX + 30, 44, "Status::describe for on-hit", MID)
text(LX + 30, 45, '"..." when a row cuts it short', MID)

# --- right: the short windows ---------------------------------------------------
box(76, 24, W - 5, 46, elite("give_way.dat // short windows"))
RX = 79
cols = [0, 12, 30, 40, 54]
for c, h in zip(cols, ["WINDOW", "ROWS", "ICONS", "ROSTER", "STATS"]):
    text(RX + c, 26, h, HEAD)
text(RX, 27, "─" * 64, FAINT)
table = [
    ["1920x1080", "3 lines", "stacked", "2x8, named", "labelled"],
    ["1280x720", "1 line + ...", "beside", "2x8, named", "labelled"],
    ["1024x600", "name + hover", "beside", "2x8, hover", "icon + n"],
]
for k, cells in enumerate(table):
    for c, cell in zip(cols, cells):
        text(RX + c, 28 + k, cell, WHITE if c == 0 else BRIGHT if k < 2 else AMBER)
text(RX, 31, "─" * 64, FAINT)
text(RX, 32, "GIVE WAY", HEAD)
text(RX + 9, 32, "// first to go on the left", DIM)
chain = [("tagline", 7), ("weapon", 6), ("descriptions", 12), ("build", 5)]
x = RX + 6
spans = []
for i, (word, n) in enumerate(chain):
    text(x, 33, word, CYAN)
    spans.append((x, n))
    if i < len(chain) - 1:
        text(x + n + 1, 33, "──►", DIM)
    x += n + 5
alive = {"1080": [1, 1, 1, 1], "720": [0, 0, 1, 1], "600": [0, 0, 0, 1]}
for k, (res, keep) in enumerate(alive.items()):
    text(RX, 34 + k, res, WHITE)
    for (sx, n), on in zip(spans, keep):
        text(sx, 34 + k, ("▓" if on else "░") * n, BRIGHT if on else FAINT)
text(RX + 53, 35, "1 line each", MID)
text(RX + 53, 36, "on hover", MID)
text(RX, 37, "─" * 64, FAINT)
text(RX, 38, "AT 1024x600, WAS", RED)
text(RX + 33, 38, "NOW", BRIGHT)
fixes = [
    ("roster ran past its panel", "2 rows of 8, no name line"),
    ("tags fell off the parchment", "the left column widens"),
    ("R row fell off the bottom", "one-line rows, hover text"),
    ("Damage under the Armour icon", "icon + number, label on hover"),
]
for k, (was, now) in enumerate(fixes):
    assert RX + 33 + len(now) + 2 < W - 6, now
    text(RX, 39 + k, "- " + was, (200, 90, 80))
    text(RX + 33, 39 + k, "+ " + now, MID)
text(RX, 44, "TESTS", HEAD)
text(RX + 7, 44, "433 pass  +5: sentences, names, clip, build, roster", MID)
text(RX, 45, "FLAG", HEAD)
text(RX + 7, 45, "--cursor X Y   puts the cursor down for hover shots", MID)

# --- sysop's note ------------------------------------------------------------
header(48, elite("sysop's note // the operator"))
clear(4, 49, W - 5, 50)
center(49, "sTILL oPEN: bATTLE sPELLS (p4.3), mUSIC tOGGLE (p4.6), cLIFFS + bRIDGES + gOD sTATUES (p5), iTEM sETS iN tHE sHOP (p3).", HEAD)
center(50, "Passive::describe // Status::describe // ItemShop::popular_build // hero_select::clip_lines // Hud::tooltip // --cursor", DIM)

# --- roadmap -------------------------------------------------------------------
header(51, elite("roadmap // docs/ROADMAPPLAN.md"))
clear(4, 52, W - 5, 56)
phases = [
    ("P1 LIGHT", 5), ("P2 MINIMAP", 6), ("P3 SHOP", 7), ("P4 SELECT", 8),
    ("P5 ARENA", 1), ("P6 HUD", 4), ("P7 POST", 0),
]
for i, (name, done) in enumerate(phases):
    x = 8 + (i % 4) * 35
    yy = 52 + (i // 4) * 2
    text(x, yy, f"{name:<11}", CYAN)
    text(x + 11, yy, "[", DIM)
    for j in range(10):
        put(x + 12 + j, yy, "▓" if j < done else "░", BRIGHT if j < done else FAINT)
    text(x + 22, yy, f"] {done * 10:>3}%", HEAD if done else FAINT)
text(8, 56, ">> next: battle spells (P4.3: D/F slots, OSRS spell sprites, gameplay), the music toggle (P4.6), cliffs from cache objects (P5)", MID)

fade_bar(57, 2, W - 3)
clear(10, 58, W - 11, 61)
greetz = "gREETZ: tANK (tHE oPERATOR aND tHE bUILD) * pARCHMENT 1017 * tHE pALE yELLOW bOX * 1024x600, wHICH fITS nOW"
assert len(greetz) <= 128, len(greetz)
center(58, greetz, MID)
center(59, "nOT AFFILIATED WITH JAGEX. cACHE ASSETS STAY OUT OF GIT. aLWAYS.", DIM)
center(60, "  t a n k ,   l o a d   t h e   b u i l d .   i   k n o w   t h r e e   h e a d s .   s h o w   m e .  ", HEAD)
center(61, "-=[ mObAsCaPe // 2026 // nODE 15: tHE oPERATOR // 57.6k // sYSoP: rIFT // 12 pASSIVES, 5 iTEMS, 1 pALE yELLOW bOX ]=-", AMBER)

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
