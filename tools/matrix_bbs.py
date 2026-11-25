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
    lrng = random.Random(17)
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
BARK = (150, 110, 70)
PLANK = (200, 160, 90)
WATER = (70, 150, 255)
SARA = (235, 240, 255)
ZAM = (255, 70, 50)
EARTH = (120, 220, 80)
LANE_BG = (40, 32, 22)
RIVER_BG = (6, 26, 60)
S_BG = (46, 39, 31)
CHIP_BG = (26, 22, 18)
CHIP_FG = (236, 226, 200)
S_RULE = (110, 92, 66)


def sentinel(x0, y0):
    """The Sentinel: a tower locks on and fires. Orb out, Surge splash in."""
    clear(x0 - 1, y0, x0 + 21, y0 + 11)
    for j in range(20):
        put(x0 + j, y0, "▄", MID)
        put(x0 + j, y0 + 8, "▄", MID)
    for k in range(1, 9):
        put(x0, y0 + k, "█", DIM)
        put(x0 + 19, y0 + k, "█", DIM)
    text(x0 + 2, y0 + 1, "> sentinel ", DIM)
    text(x0 + 13, y0 + 1, "LOCK", RED)
    text(x0 + 2, y0 + 2, "target: kolodion", CYAN)
    # The obelisk on the right, the orb in flight, the splash on the hero.
    text(x0 + 15, y0 + 3, "▄█▄", STONE)
    text(x0 + 15, y0 + 4, "███", STONE)
    text(x0 + 15, y0 + 5, "███", STONE)
    text(x0 + 2, y0 + 4, "¤@¤", ORANGE)
    put(x0 + 3, y0 + 4, "@", CYAN)
    text(x0 + 6, y0 + 4, "◄~~~", ORANGE)
    put(x0 + 10, y0 + 4, "●", ZAM)
    text(x0 + 11, y0 + 4, "~~", RED)
    text(x0 + 2, y0 + 5, "1466", ORANGE)
    text(x0 + 9, y0 + 5, "1528", ZAM)
    text(x0 + 2, y0 + 6, "▓" * 9 + "▒░", RED)
    text(x0 + 13, y0 + 6, "-15", RED)
    text(x0 + 2, y0 + 7, "> splash. ok.", HEAD)
    text(x0 + 6, y0 + 9, "▄▄█████▄▄", DIM)
    text(x0 + 1, y0 + 10, "▄" * 18, MID)
    text(x0 + 1, y0 + 11, "▀" + "▪" * 16 + "▀", DIM)
    for j in range(16):
        put(x0 + 2 + j, y0 + 11, "▪", ZAM if j % 5 == 2 else DIM)


LOADED = [
    "cliff rocks", "plank bridges", "god statues", "god banners", "ferns+stumps",
    "plain texture", "surge splash", "hit_scale", "flesh hook", "devour",
    "thrall scrolls", "opcode 99",
]


def construct_list(x0, y0):
    """Everything this edition loads, like the Construct's racks."""
    clear(x0 - 1, y0, x0 + 22, y0 + len(LOADED) - 1)
    for k, name in enumerate(LOADED):
        yy = y0 + k
        hot = name == "surge splash"
        text(x0, yy, name, WHITE if hot else CYAN)
        text(x0 + len(name), yy, "." * (18 - len(name)), FAINT)
        text(x0 + 19, yy, "ok", HEAD if hot else MID)


clear(2, 2, W - 3, 2)
lx, lw = code_logo(3, "MOBASCAPE")
sentinel(4, 3)
construct_list(125, 3)
text(3, 2, "▐ sENTINEL ▌", DIM)
text(W - 18, 2, "▐ cONSTRUCT ▌", DIM)
top = "  tANK, lOAD tHE cONSTRUCT.   cLIFFS, bRIDGES, gODS.   sENTINELS oNLINE.  "
assert len(top) <= 110, len(top)
center(2, top, MID)
fade_bar(1, 2, W - 3)
clear(lx - 3, 15, lx + lw + 2, 15)
ticker = " ◄◄ PLANK BRIDGES ◄◄ GOD STATUES ◄◄ SURGE SPLASH ◄◄ FLESH HOOK ◄◄ THRALLS RISE"
run = (ticker * 3)[: lw]
for i, ch in enumerate(run):
    put(lx + i, 15, ch, ORANGE if ch == "◄" else AMBER)
center(16, "  ░▒▓  n O D E   1 7 :   t H E   c O N S T R U C T   //   r O C K S ,   b R I D G E S ,   s E N T I N E L S  ▓▒░  ", AMBER)
center(17, "  " + elite("we need cliffs.") + "   " + elite("lots of cliffs.") + "  ", WHITE)
fade_bar(18, 2, W - 3)

# --- release info -------------------------------------------------------------
header(19, elite("release info"))
clear(4, 20, W - 5, 22)
info = [
    ("rELEASE dATE", "2026-11-25", "cOMMITS", "0435638 .. fc65463  ->  sam/main"),
    ("tYPE", "aRENA, sENTINELS, bLOAT, tHRALLS, oPCODE 99", "tESTS", "470 pass // clippy clean"),
    ("wAS", "wHITE vOID: cUBE rOCKS, nO cLIFFS, nO bRIDGES", "nOW", "410 rocks, 1950 ferns, 3 bridges, 2 gods"),
]
for i, (k1, v1, k2, v2) in enumerate(info):
    assert len(v1) <= 50 and 89 + len(v2) <= W - 6, (v1, v2)
    yy = 20 + i
    text(8, yy, f"{k1:.<16}", DIM)
    text(25, yy, v1, RED if k1 == "wAS" else BRIGHT)
    text(76, yy, f"{k2:.<12}", DIM)
    text(89, yy, v2, HEAD)

# --- left: the construct (a slice of the arena) --------------------------------
box(4, 24, 74, 46, elite("construct.dat // phase 5"))
LX = 6
text(LX + 1, 26, "THE CONSTRUCT", HEAD)
text(LX + 15, 26, "// map.json props: place · lines · scatter", DIM)
# One lane crossing the river on a plank bridge, Saradomin at the left
# gate, Zamorak at the right, a sentinel firing at a hero mid-lane.
ARENA = [
    "¶Ψ ♣ , ♣  ▲▲ ▲▲▲ ▲▲  ▲ ▲▲    ≈≈≈≈    ▲▲▲ ▲▲  ▲ ▲▲▲  ♣ , ♣   Ψ¶",
    "▐█▌  ♣    ,   ♣    ,   ♣    ≈≈≈≈   ♣    ,    ♣   ▄█▄   ,    ▐█▌",
    "▐█▌░░░░░░░░░░░░░░░░░░░░░░░##########░░░░░░░░░░░░░███░░░░░░░▐█▌",
    "▐█▌░░░░░░░░░░░░░░░░░¤@¤◄~~##########~~~●~~~░░░░░░███░░░░░░░▐█▌",
    "▐█▌░░░░░░░░░░░░░░░░░░░░░░░##########░░░░░░░░░░░░░░░░░░░░░░░▐█▌",
    "▐█▌  ,   ♣    ,  ♣    ,   ≈≈≈≈≈   ,    ♣     ,    ♣    ,    ▐█▌",
    "¶Ψ ♣  ▲▲▲ ▲▲  ▲ ▲▲▲  ▲▲ ≈≈≈≈≈   ▲▲ ▲▲▲  ▲▲  ▲ ▲▲▲  ▲▲ ♣ ,   Ψ¶",
    "  ,  ♣   ,   ♣  ,   ♣ ≈≈≈≈≈   ♣  ,    ♣   ,   ♣    ,   ♣    ,  ",
]
AX = LX + 2
for r, row in enumerate(ARENA):
    assert AX + len(row) <= 73, (r, len(row))
    yy = 28 + r
    for c, ch in enumerate(row):
        x = AX + c
        side = SARA if c < len(row) // 2 else ZAM
        colour = {
            "▲": STONE, "♣": FERN, ",": BARK, "≈": WATER, "#": PLANK,
            "░": PARCH, "Ψ": side, "¶": side, "@": CYAN, "¤": ORANGE,
            "◄": ORANGE, "~": RED, "●": ZAM,
        }.get(ch, STONE if ch in "▄█▐▌" else DIM)
        put(x, yy, ch, colour)
        if ch == "░" or ch == "#":
            shade(x, yy, x, yy, LANE_BG)
        elif ch == "≈":
            shade(x, yy, x, yy, RIVER_BG)
        if ch in "░":
            put(x, yy, " ", PARCH)
text(AX, 36, "SARADOMIN 9146 + 2065", SARA)
text(AX + 26, 36, "bridge 699", PLANK)
text(AX + 42, 36, "1400 + 8655 ZAMORAK", ZAM)
text(LX + 1, 38, "─" * 66, FAINT)
loads = [
    ("cliffs", "~410 rocks (19997/818/819) 8.5 tiles off lanes"),
    ("flora", "~1,950 ferns + stumps, seeded, off camps"),
    ("plain", "cache face colours: white gods, green ferns"),
]
for k, (key, line) in enumerate(loads):
    yy = 39 + k
    assert LX + 10 + len(line) <= 73, line
    text(LX + 1, yy, key, AMBER)
    text(LX + 10, yy, line, MID)
term = [
    ("> ", "tank, load the construct."),
    ("> ", "we need cliffs. lots of cliffs."),
    ("> ", "and bridges."),
]
for k, (prompt, line) in enumerate(term):
    yy = 43 + k
    text(LX + 1, yy, prompt, DIM)
    text(LX + 3, yy, line, BRIGHT if k < 2 else HEAD)
put(LX + 3 + len(term[-1][1]) + 1, 45, "█", BRIGHT)
notes = ["rift-tools model-sheet", "picks props by eye", "MAP_FORMAT v1.8"]
for k, line in enumerate(notes):
    assert LX + 42 + len(line) <= 73, line
    text(LX + 42, 43 + k, line, MID)

# --- right: sentinels, the bloat, the thralls ----------------------------------
box(76, 24, W - 5, 46, elite("sentinel.dat + bloat.dat"))
RX = 79
text(RX, 26, "THE SENTINELS", HEAD)
text(RX + 14, 26, "// tower shots: cache orb out, Surge splash in", DIM)
text(RX, 27, "─" * 64, FAINT)
shots = [
    ("blue ", "1526", WATER, "1460 water surge", "saradomin"),
    ("red  ", "1528", ZAM, "1466 fire surge", "zamorak"),
    ("watch", "1527", EARTH, "1463 earth surge", "neutral"),
]
for k, (who, orb, colour, splash, god) in enumerate(shots):
    yy = 28 + k
    text(RX, yy, who, CYAN)
    text(RX + 7, yy, "orb " + orb, STONE)
    put(RX + 17, yy, "●", colour)
    text(RX + 18, yy, "~~~►", colour)
    put(RX + 23, yy, "¤", colour)
    text(RX + 25, yy, splash, AMBER)
    text(RX + 44, yy, god, DIM)
text(RX, 31, "hit_scale", CYAN)
text(RX + 10, 31, "flies at 3.2x, splashes at 1.3 (hero size)", MID)
text(RX, 32, "─" * 64, FAINT)
text(RX, 33, "PESTILENT BLOAT", HEAD)
text(RX + 16, 33, "// npc 8359 · MELEE · the flesh hooker", DIM)
kit = [
    ("P", "Flesh heap", "+2 max hp per death near it, +25 a hero", "---"),
    ("Q", "Flesh Hook", "thrown limb drags the first enemy home", "1574"),
    ("W", "Swarm", "flies ride it 5 s: bite, slow, 4 hp/tick", "1568"),
    ("E", "Bloated Hide", "140 shield + 20% less damage for 4 s", "1579"),
    ("R", "Devour", "2.4 s channel, stun, chew, heal 15 a bite", "1576"),
]
for k, (key, name, what, gfx) in enumerate(kit):
    yy = 34 + k
    assert len(what) <= 41, what
    text(RX, yy, f" {key} ", CHIP_FG)
    shade(RX, yy, RX + 2, yy, CHIP_BG)
    text(RX + 4, yy, name, AMBER)
    text(RX + 17, yy, what, MID)
    text(RX + 60, yy, gfx, CYAN)
text(RX, 39, "─" * 64, FAINT)
text(RX, 40, "THE OPERATOR LOADS", HEAD)
text(RX + 19, 40, "// thrall scrolls, 300 gp, keys 1-6", DIM)
text(RX, 41, "ghostly", PURPLE)
text(RX + 8, 41, "10880 mage", MID)
text(RX + 21, 41, "skeletal", STONE)
text(RX + 30, 41, "10883 range", MID)
text(RX + 44, 41, "zombified", GREEN)
text(RX + 54, 41, "10886", MID)
text(RX, 42, "opcode 99", CYAN)
text(RX + 10, 42, "a flag, not a byte ──► bloat, hydra, nightmare", BRIGHT)
text(RX, 44, "TESTS", HEAD)
text(RX + 7, 44, "470 pass  +1: tower looks stay on verified ids", MID)
text(RX, 45, "FLAGS", HEAD)
text(RX + 7, 45, "--demo-tower  --demo-thrall  --items a,b", MID)

# --- sysop's note ------------------------------------------------------------
header(48, elite("sysop's note // the construct"))
clear(4, 49, W - 5, 50)
open_ = "sTILL oPEN: gOD-cOLOURED cASTLES + tHE fOUNTAIN (p5.3), gROUND mIST (p5.7), mUSIC tOGGLE (p4.6), iTEM sETS (p3)."
refs = "scene::props // ProjectileLook::hit_scale // Passive::FleshHeap // items::ThrallSpec // npc opcode 99"
assert len(open_) <= 142 and len(refs) <= 142, (len(open_), len(refs))
center(49, open_, HEAD)
center(50, refs, DIM)

# --- roadmap -------------------------------------------------------------------
header(51, elite("roadmap // docs/ROADMAPPLAN.md"))
clear(4, 52, W - 5, 56)
phases = [
    ("P1 LIGHT", 5), ("P2 MINIMAP", 6), ("P3 SHOP", 7), ("P4 SELECT", 9),
    ("P5 ARENA", 8), ("P6 HUD", 4), ("P7 POST", 0),
]
for i, (name, done) in enumerate(phases):
    x = 8 + (i % 4) * 35
    yy = 52 + (i // 4) * 2
    text(x, yy, f"{name:<11}", CYAN)
    text(x + 11, yy, "[", DIM)
    for j in range(10):
        put(x + 12 + j, yy, "▓" if j < done else "░", BRIGHT if j < done else FAINT)
    text(x + 22, yy, f"] {done * 10:>3}%", HEAD if done else FAINT)
nxt = ">> next: the fountain + god-coloured castle pieces (P5.3), ground mist from cache smoke (P5.7), the music toggle (P4.6)"
assert 8 + len(nxt) <= W - 6, len(nxt)
text(8, 56, nxt, MID)

fade_bar(57, 2, W - 3)
clear(10, 58, W - 11, 61)
greetz = "gREETZ: tANK (tHE cONSTRUCT) * tHE sENTINELS * pESTILENT bLOAT * tHE tHRALLS * sARADOMIN * zAMORAK * kOLODION"
assert len(greetz) <= 128, len(greetz)
center(58, greetz, MID)
center(59, "nOT AFFILIATED WITH JAGEX. cACHE ASSETS STAY OUT OF GIT. aLWAYS.", DIM)
center(60, "  w e   n e e d   c l i f f s .   l o t s   o f   c l i f f s .   s e n t i n e l s   o n l i n e .  ", HEAD)
foot = "-=[ mObAsCaPe // 2026 // nODE 17: tHE cONSTRUCT // 57.6k // sYSoP: rIFT // 410 rOCKS, 3 bRIDGES, 3 sPLASHES ]=-"
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
