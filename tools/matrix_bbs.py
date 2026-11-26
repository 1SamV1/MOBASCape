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
    lrng = random.Random(21)
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


TESTS = 472

GOLD = (255, 210, 90)
BLOOD = (200, 30, 40)
STONE2 = (150, 145, 150)
CHIP_BG = (26, 22, 18)
CHIP_FG = (236, 226, 200)
FIRE = (255, 140, 40)
DIRT = (150, 110, 60)
DIRT_BG = (34, 24, 10)
CITY_BG = (16, 22, 14)
AWNING = (236, 232, 220)
STRIPE = (90, 150, 255)
LAMP = (255, 230, 120)
WOOD = (150, 100, 55)
WATER = (80, 150, 255)
WATER_BG = (6, 18, 40)
KEEP_BG = (12, 16, 26)
SARA = (90, 140, 255)
TARGET = (255, 80, 60)
ELF_BG = (6, 26, 26)
CRYSTAL = (110, 240, 255)
ELF_LAMP = (235, 245, 255)
LEAF = (90, 200, 90)
GNOME_BG = (10, 24, 8)
SPIRIT = (120, 220, 90)
TOAD = (230, 80, 50)


def oracle(x0, y0):
    """The Oracle's kitchen: an oven of cookies, almost done."""
    clear(x0 - 1, y0, x0 + 21, y0 + 11)
    for j in range(20):
        put(x0 + j, y0, "▄", MID)
        put(x0 + j, y0 + 8, "▄", MID)
    for k in range(1, 9):
        put(x0, y0 + k, "█", DIM)
        put(x0 + 19, y0 + k, "█", DIM)
    text(x0 + 2, y0 + 1, "> oracle", DIM)
    art = ["  ▄▄▄▄▄▄▄  ", " █ ◦ ◦ ◦ █ ", " █ ◦ ◦ ◦ █ ", " ▐▓▓▓▓▓▓▓▌ ", "  ▀▀   ▀▀  "]
    for k, row in enumerate(art):
        for j, ch in enumerate(row):
            col = GOLD if ch == "◦" else FIRE if ch == "▓" else STONE2
            if ch != " ":
                put(x0 + 4 + j, y0 + 2 + k, ch, col)
    text(x0 + 2, y0 + 7, "> cookies. ok.", HEAD)
    text(x0 + 6, y0 + 9, "▄▄█████▄▄", DIM)
    text(x0 + 1, y0 + 10, "▄" * 18, MID)
    for j in range(18):
        put(x0 + 1 + j, y0 + 11, "▪", [AWNING, STRIPE, CRYSTAL, SPIRIT][j % 4] if 0 < j < 17 else DIM)


LOADED = [
    "market stalls", "well square", "knight statue", "archery range",
    "dummy line", "seren fragment", "elven lamps", "crystal trees",
    "spirit tree", "gnome glider", "gnomeball", "pockets 8/8",
]


def loaded_list(x0, y0):
    clear(x0 - 1, y0, x0 + 22, y0 + len(LOADED) - 1)
    for k, name in enumerate(LOADED):
        yy = y0 + k
        last = name.startswith("pockets")
        hot = name in ("seren fragment", "spirit tree")
        text(x0, yy, name, GOLD if last else WHITE if hot else CYAN)
        text(x0 + len(name), yy, "." * (18 - len(name)), FAINT)
        text(x0 + 19, yy, "ok", GOLD if last else HEAD if hot else MID)


def plot(x0, y0, rows, cols, cells, back, base=" "):
    """Draw a little map: `cells` maps (row, col) -> (glyph, colour), and
    `back(row, col, glyph)` gives each cell's background."""
    for r in range(rows):
        for c in range(cols):
            ch, col = cells.get((r, c), (base, FAINT))
            put(x0 + c, y0 + r, ch, col)
            shade(x0 + c, y0 + r, x0 + c, y0 + r, back(r, c, ch))


clear(2, 2, W - 3, 2)
lx, lw = code_logo(3, "MOBASCAPE")
oracle(4, 3)
loaded_list(125, 3)
text(3, 2, "▐ oRACLE ▌", DIM)
text(W - 18, 2, "▐ lOADED ▌", DIM)
top = "  fOUR rADIANT pOCKETS dRESSED.   tHE mARKET oPENS.   tHE sPIRIT tREE wAKES.  "
assert len(top) <= 104, len(top)
center(2, top, MID)
fade_bar(1, 2, W - 3)
clear(lx - 3, 15, lx + lw + 2, 15)
ticker = " ◄◄ MARKET STALLS ◄◄ WELL SQUARE ◄◄ KNIGHTS' YARD ◄◄ FRAGMENT OF SEREN ◄◄ SPIRIT TREE"
run = (ticker * 3)[: lw]
for i, ch in enumerate(run):
    put(lx + i, 15, ch, ORANGE if ch == "◄" else AMBER)
center(16, "  ░▒▓  n O D E   2 2 :   t H E   o R A C L E   //   t H E   r A D I A N T   s I D E ,   d R E S S E D  ▓▒░  ", AMBER)
center(17, "  " + elite("you didn't come here to make the choice.") + "   " + elite("you've already made it.") + "  ", WHITE)
fade_bar(18, 2, W - 3)

# --- release info -------------------------------------------------------------
header(19, elite("release info"))
clear(4, 20, W - 5, 22)
info = [
    ("rELEASE dATE", "2026-11-25", "cOMMITS", "252c930 radiant x4 · 37c0dbe ogre + goblin"),
    ("tYPE", "tHEMED pOCKETS: tHE rADIANT sIDE", "tESTS", f"{TESTS} pass // clippy clean"),
    ("wAS", "tRANSPLANTED sCENERY, nO sET pIECE", "nOW", "market, yard, grove, tree: 8/8 pockets"),
]
for i, (k1, v1, k2, v2) in enumerate(info):
    assert len(v1) <= 50 and 89 + len(v2) <= W - 6, (v1, v2)
    yy = 20 + i
    text(8, yy, f"{k1:.<16}", DIM)
    text(25, yy, v1, RED if k1 == "wAS" else BRIGHT)
    text(76, yy, f"{k2:.<12}", DIM)
    text(89, yy, v2, HEAD)

# --- left: the human city ---------------------------------------------------------
box(4, 24, 74, 46, elite("city.dat // radiant_human_city, by the top lane"))
LX = 6
text(LX + 1, 26, "THE MARKET STREET", HEAD)
text(LX + 19, 26, "// stalls face the road, a well by the river", DIM)
PX, PY, PR, PC = LX + 2, 28, 8, 36
city = {}
for r in range(PR):
    for c in range(7 + 2 * r, 11 + 2 * r):
        city[(r, c)] = ("░", DIRT)  # the top lane
    for c in range(max(0, 29 - 2 * max(0, r - 3)), PC):
        city[(r, c)] = ("≈", WATER)  # the river, bending south under the lane
for r in range(PR):
    for c in range(7 + 2 * r, 11 + 2 * r):
        if city.get((r, c), ("",))[0] == "≈" or (r >= 6 and c >= 19):
            city[(r, c)] = ("╪", WOOD)  # the bridge
for r in range(7):
    city[(r, 3 + 2 * r)] = ("Π", STRIPE if r in (0, 3) else AWNING)
    if r % 2 == 0:
        city[(r, 6 + 2 * r)] = ("¡", LAMP)
for rc, g in {
    (2, 0): ("o", WOOD), (3, 1): ("o", WOOD), (4, 0): ("▪", WOOD), (5, 2): ("▪", WOOD),
    (7, 4): ("╤", WOOD), (1, 20): ("¡", LAMP), (3, 24): ("¡", LAMP),
    (2, 22): ("◎", CRYSTAL), (1, 24): ("═", WOOD), (3, 20): ("═", WOOD),
    (0, 21): ("¶", AWNING), (0, 26): ("♔", WHITE),
}.items():
    city[rc] = g
plot(PX, PY, PR, PC, city,
     lambda r, c, ch: WATER_BG if ch == "≈" else DIRT_BG if ch in "░╪" else CITY_BG)
leg = [("Π", AWNING, "stalls 1421 + five goods"), (" ", AWNING, "  striped 24905 · 24903"),
       ("¡", LAMP, "hanging lamps 1324"), ("◎", CRYSTAL, "roofed well 1410, plaza 3"),
       ("♔", WHITE, "king's statue 1527"), ("¶", AWNING, "noticeboard 2040"),
       ("═", WOOD, "benches 1496 · ╤ 1453"), ("o", WOOD, "barrels 1460 · ▪ 7699")]
for k, (ch, col, label) in enumerate(leg):
    yy = PY + k
    assert PX + 40 + len(label) <= 73, label
    put(PX + 38, yy, ch, col)
    text(PX + 40, yy, label, MID)
text(LX + 1, 37, "─" * 66, FAINT)
notes = [
    ("cut", "the top lane and the river split the pocket in three"),
    ("row", "seven stalls 7.6 tiles off the lane, yaw down the road"),
    ("fix", "five spots failed the lane/river check; moved, not forced"),
]
for k, (key, line) in enumerate(notes):
    text(LX + 1, 38 + k, key, AMBER)
    text(LX + 8, 38 + k, line, MID)
term = [
    ("> ", "bakery stall 1445. the cookies are almost done."),
    ("> ", "and by the time you've had one, you'll feel right as rain."),
]
for k, (prompt, line) in enumerate(term):
    yy = 43 + k
    text(LX + 1, yy, prompt, DIM)
    text(LX + 3, yy, line, BRIGHT if k == 0 else HEAD)
put(LX + 3 + len(term[-1][1]) + 1, 44, "█", BRIGHT)

# --- right: keep, elves, gnomes ---------------------------------------------------
box(76, 24, W - 5, 46, elite("radiant.dat // keep, grove, stronghold"))
RX = 79
IX = RX + 19


def section(y, title, sub, art, back, items):
    text(RX, y, title, HEAD)
    text(RX + len(title) + 1, y, sub, DIM)
    plot(RX, y + 1, 5, 17, art, back)
    for k, (oid, what) in enumerate(items):
        assert IX + 8 + len(what) <= W - 7, what
        text(IX, y + 1 + k, oid, CYAN)
        text(IX + 8, y + 1 + k, what, MID)


keep = {(r, 1): ("◉", TARGET) for r in range(5)}
keep.update({(r, 15): ("¥", WOOD) for r in range(5)})
keep.update({(1, 6): ("¶", SARA), (1, 10): ("¶", SARA), (3, 6): ("¶", SARA), (3, 10): ("¶", SARA),
             (2, 8): ("♞", WHITE), (4, 5): ("╫", WOOD), (4, 11): ("╫", WOOD),
             (4, 7): ("Ж", STONE2), (4, 9): ("Ж", STONE2), (0, 8): ("♞", STONE2)})
section(26, "THE KNIGHTS' YARD", "// radiant_human_keep", keep, lambda r, c, ch: KEEP_BG, [
    ("40158", "knight statue, four blue banners"),
    ("1451", "archery range x4 + 12896, west"),
    ("1286", "dummy line 25177 32307, east"),
    ("5946", "weapon racks · armour stands"),
    ("budget", "falador transplant 100 -> 35"),
])

elf = {(2, 8): ("✶", CRYSTAL)}
for rc in [(0, 6), (0, 10), (2, 4), (2, 12), (4, 6), (4, 10)]:
    elf[rc] = ("o", ELF_LAMP)
elf.update({(2, 1): ("Ψ", STONE2), (2, 15): ("Ψ", LEAF), (0, 2): ("♣", CRYSTAL), (0, 14): ("♣", CRYSTAL),
            (4, 2): ("♣", CRYSTAL), (4, 14): ("♣", CRYSTAL), (1, 8): ("◆", CRYSTAL), (3, 0): ("◆", CRYSTAL),
            (3, 16): ("◆", CRYSTAL), (4, 8): ("⌂", ELF_LAMP)})
section(33, "THE CRYSTAL GROVE", "// radiant_elven_lands", elf, lambda r, c, ch: ELF_BG, [
    ("38604", "fragment of seren, six lamps 37359"),
    ("38323", "glarial · 38322 baxtorian"),
    ("37353", "crystal trees x5, saplings"),
    ("30686", "arcane crystals at x0.6, not x1.3"),
    ("budget", "jungle transplant 110 -> 60"),
])

gno = {(0, 7): ("▄", SPIRIT), (0, 8): ("█", SPIRIT), (0, 9): ("▄", SPIRIT),
       (1, 6): ("♣", SPIRIT), (1, 7): ("♣", SPIRIT), (1, 8): ("♣", SPIRIT), (1, 9): ("♣", SPIRIT), (1, 10): ("♣", SPIRIT),
       (2, 8): ("┃", WOOD), (4, 2): ("∩", AWNING), (4, 14): ("∩", AWNING), (4, 8): ("⚑", GOLD),
       (0, 14): ("✈", AWNING), (0, 12): ("°", LAMP), (0, 16): ("°", LAMP),
       (1, 1): ("π", WOOD), (2, 2): ("π", WOOD), (3, 15): ("Ħ", WOOD),
       (2, 5): ("Ψ", STONE2), (2, 11): ("Ψ", STONE2), (3, 4): ("♠", TOAD), (0, 1): ("♠", TOAD)}
section(40, "THE STRONGHOLD", "// radiant_gnome_stronghold", gno, lambda r, c, ch: GNOME_BG, [
    ("8110", "spirit tree x2.3, cache leaf canopy"),
    ("1323", "glider between landing lights 5647"),
    ("1317", "gnomeball goals · flag 10348"),
    ("17144", "cafe: tables, chairs, cooker"),
    ("24742", "swings · net 1439 · toadstools"),
])

# --- sysop's note ------------------------------------------------------------
header(48, elite("sysop's note // the oracle"))
clear(4, 49, W - 5, 50)
open_ = "aLL eIGHT pOCKETS dRESSED fROM cACHE oBJECTS (8/8), nO fOG, nO mIST. sTILL oPEN: tHE oBJECTIVES pANEL (p6.3)."
refs = "radiant_human_city + radiant_human_keep + radiant_elven_lands + radiant_gnome_stronghold // shipped_pockets_have_their_set_pieces"
assert len(open_) <= 142 and len(refs) <= 142, (len(open_), len(refs))
center(49, open_, HEAD)
center(50, refs, DIM)

# --- roadmap -------------------------------------------------------------------
header(51, elite("roadmap // docs/ROADMAPPLAN.md"))
clear(4, 52, W - 5, 56)
phases = [
    ("P1 LIGHT", 5), ("P2 MINIMAP", 6), ("P3 SHOP", 7), ("P4 SELECT", 9),
    ("P5 ARENA", 9), ("POCKETS", 10), ("P6 HUD", 4), ("P7 POST", 0),
]
for i, (name, done) in enumerate(phases):
    x = 8 + (i % 4) * 35
    yy = 52 + (i // 4) * 2
    text(x, yy, f"{name:<11}", CYAN)
    text(x + 11, yy, "[", DIM)
    for j in range(10):
        put(x + 12 + j, yy, "▓" if j < done else "░", (GOLD if done == 10 else BRIGHT) if j < done else FAINT)
    text(x + 22, yy, f"] {done * 10:>3}%", GOLD if done == 10 else HEAD if done else FAINT)
nxt = ">> next: P6.3 objectives panel: named objectives with respawn timers and per-lane tower states"
assert 8 + len(nxt) <= W - 6, len(nxt)
text(8, 56, nxt, MID)

fade_bar(57, 2, W - 3)
clear(10, 58, W - 11, 61)
greetz = "gREETZ: tHE oRACLE * sERAPH * tHE kEYMAKER * kING rOALD * sIR aMIK vARZE * gLARIAL * bAXTORIAN * kING nARNODE"
assert len(greetz) <= 128, len(greetz)
center(58, greetz, MID)
center(59, "nOT AFFILIATED WITH JAGEX. cACHE ASSETS STAY OUT OF GIT. aLWAYS.", DIM)
center(60, "  y o u ' r e   n o t   t h e   o n e .   b u t   t h e   m a p   i s   d o n e .  ", HEAD)
foot = "-=[ mObAsCaPe // 2026 // nODE 22: tHE oRACLE // 57.6k // sYSoP: rIFT // 8/8 pOCKETS, 0 mIST, 100% cACHE ]=-"
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
