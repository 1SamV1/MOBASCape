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


TESTS = 477

GOLD = (255, 210, 90)
GOLD_DK = (150, 105, 30)
STONE2 = (150, 145, 150)
STONE_BG = (34, 30, 26)
SLOT_BG = (52, 46, 38)
CHIP_BG = (26, 22, 18)
CHIP_FG = (236, 226, 200)
FIRE = (255, 140, 40)
HP = (40, 220, 60)
MP = (60, 230, 240)
BLUE_T = (120, 160, 255)
RED_T = (255, 110, 100)
SWIFT = (90, 230, 90)
PANEL_BG = (14, 14, 18)
KEY_BG = (20, 26, 14)


def keymaker(x0, y0):
    """The Keymaker's ring: every key opens a door."""
    clear(x0 - 1, y0, x0 + 21, y0 + 11)
    for j in range(20):
        put(x0 + j, y0, "▄", MID)
        put(x0 + j, y0 + 8, "▄", MID)
    for k in range(1, 9):
        put(x0, y0 + k, "█", DIM)
        put(x0 + 19, y0 + k, "█", DIM)
    text(x0 + 2, y0 + 1, "> keymaker", DIM)
    art = ["   ▄▀▀▀▄   ", "  █  ○  █  ", "   ▀▄█▄▀   ", " ╒══╪══╕ ┐ ", " Q  W  E R "]
    for k, row in enumerate(art):
        for j, ch in enumerate(row):
            col = GOLD if ch in "▄▀█○" else CHIP_FG if ch in "QWER" else GOLD_DK
            if ch != " ":
                put(x0 + 4 + j, y0 + 2 + k, ch, col)
    text(x0 + 2, y0 + 7, "> ctrl+key. ok.", HEAD)
    text(x0 + 6, y0 + 9, "▄▄█████▄▄", DIM)
    text(x0 + 1, y0 + 10, "▄" * 18, MID)
    for j in range(18):
        put(x0 + 1 + j, y0 + 11, "+" if j % 3 == 1 else "▪", GOLD if j % 3 == 1 else DIM)


LOADED = [
    "dress_radiant.py", "camp names", "boss lines", "camp groups",
    "buff holders", "timer colour", "+ buttons", "ctrl+q/w/e/r",
    "click: cast", "round portrait", "tooltip lift", "potions",
]


def loaded_list(x0, y0):
    clear(x0 - 1, y0, x0 + 22, y0 + len(LOADED) - 1)
    for k, name in enumerate(LOADED):
        yy = y0 + k
        todo = name == "potions"
        hot = name in ("+ buttons", "boss lines")
        text(x0, yy, name, AMBER if todo else WHITE if hot else CYAN)
        text(x0 + len(name), yy, "." * (18 - len(name)), FAINT)
        text(x0 + 19, yy, ".." if todo else "ok", AMBER if todo else GOLD if hot else MID)


def paint(x0, y0, rows, palette, back):
    """Draw rows of glyphs; `palette` maps a glyph to its colour."""
    for r, row in enumerate(rows):
        for c, ch in enumerate(row):
            put(x0 + c, y0 + r, ch, palette.get(ch, DIM))
            shade(x0 + c, y0 + r, x0 + c, y0 + r, back(r, c, ch))


clear(2, 2, W - 3, 2)
lx, lw = code_logo(3, "MOBASCAPE")
keymaker(4, 3)
loaded_list(125, 3)
text(3, 2, "▐ kEYMAKER ▌", DIM)
text(W - 18, 2, "▐ lOADED ▌", DIM)
top = "  tHE cAMPS hAVE nAMES.   eVERY aBILITY hAS iTS kEY.   tHE + oPENS tHE dOOR.  "
assert len(top) <= 100, len(top)
center(2, top, MID)
fade_bar(1, 2, W - 3)
clear(lx - 3, 15, lx + lw + 2, 15)
ticker = " ◄◄ CRYPT SPIDER 37s ◄◄ ALBINO BAT ALIVE ◄◄ JUNGLE CAMPS 1/3 ◄◄ + + + ◄◄ CTRL+Q/W/E/R"
run = (ticker * 3)[: lw]
for i, ch in enumerate(run):
    put(lx + i, 15, ch, ORANGE if ch == "◄" else AMBER)
center(16, "  ░▒▓  n O D E   2 3 :   t H E   k E Y M A K E R   //   o B J E C T I V E S   a N D   t H E   + + +  ▓▒░  ", AMBER)
center(17, "  " + elite("only the one can open the door.") + "   " + elite("and only during that window.") + "  ", WHITE)
fade_bar(18, 2, W - 3)

# --- release info -------------------------------------------------------------
header(19, elite("release info"))
clear(4, 20, W - 5, 22)
info = [
    ("rELEASE dATE", "2026-11-25", "cOMMITS", "0574602 + · 8fe3ef5 objectives · e32c66f tool"),
    ("tYPE", "hUD: rOADMAP 6.3 dONE, 6.4 sTARTED", "tESTS", f"{TESTS} pass // clippy clean"),
    ("wAS", "8 gENERIC cAMP rOWS; cLICK = rANK uP", "nOW", "6 named lines; + / ctrl ranks, click casts"),
]
for i, (k1, v1, k2, v2) in enumerate(info):
    assert len(v1) <= 50 and 89 + len(v2) <= W - 6, (v1, v2)
    yy = 20 + i
    text(8, yy, f"{k1:.<16}", DIM)
    text(25, yy, v1, RED if k1 == "wAS" else BRIGHT)
    text(76, yy, f"{k2:.<12}", DIM)
    text(89, yy, v2, HEAD)

# --- left: the ability bar ----------------------------------------------------------
box(4, 24, 74, 46, elite("bar.dat // the ability bar, roadmap 6.4"))
LX = 6
text(LX + 1, 26, "THE SKILL-POINT DOOR", HEAD)
text(LX + 22, 26, "// a + over every slot that can learn", DIM)
bar = [
    "            [+]  [+]  [+]                       ",
    " ▄▀▀▀▀▄   ┌──┐ ┌──┐ ┌──┐ ┌──┐   ┌──┐ ┌──┐  ┌─┐┌─┐ ",
    "█ ◕‿◕  █  │Q │ │W │ │E │ │R │   │D │ │F │  │1││2│ ",
    "█  ▐▌  █  └──┘ └──┘ └──┘ └──┘   └──┘ └──┘  │3││T│ ",
    " ▀▄▄▄▄▀▣  ▪▪▪▫ ▪▪▫▫ ▪▫▫▫ ▪▪▫         ",
    "          ████████████████████ 740/740         ",
    "          ▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒ 320/320         ",
]
pal = {"+": (40, 25, 5), "[": GOLD, "]": GOLD, "▄": GOLD, "▀": GOLD_DK, "█": GOLD,
       "◕": CHIP_FG, "‿": CHIP_FG, "▐": STONE2, "▌": STONE2, "▣": FIRE,
       "┌": STONE2, "┐": STONE2, "└": STONE2, "┘": STONE2, "│": STONE2, "─": STONE2,
       "Q": GOLD, "W": GOLD, "E": GOLD, "R": CHIP_FG, "D": STONE2, "F": STONE2,
       "1": STONE2, "2": STONE2, "3": STONE2, "T": GOLD, "▪": GOLD, "▫": DIM, "0": CHIP_FG,
       "7": CHIP_FG, "4": CHIP_FG, "2": STONE2, "/": CHIP_FG}


def bar_back(r, c, ch):
    if ch == "+" and r == 0:
        return GOLD
    if r == 5 and ch == "█":
        return (10, 60, 14)
    if r == 6 and ch == "▒":
        return (6, 50, 60)
    return STONE_BG


paint(LX + 2, 28, bar, pal, bar_back)
for k, (glyph, label, col) in enumerate([("[+]", "gold, 16 px, 4 above the slot", GOLD),
                                         ("◯", "76 px portrait, ringed in gold", GOLD)]):
    text(LX + 2, 36 + k, glyph, col)
    text(LX + 8, 36 + k, label, MID)
text(LX + 1, 38, "─" * 66, FAINT)
notes = [
    ("+", "click the + to rank up · or ctrl + q/w/e/r"),
    ("click", "learns a rank-0 ability, casts a learned one"),
    ("was", "a free point turned every slot click into a rank-up"),
    ("tip", "\"click + or ctrl+w to rank up (2 left)\", lifted 22 px"),
]
for k, (key, line) in enumerate(notes):
    text(LX + 1, 39 + k, key, AMBER)
    text(LX + 8, 39 + k, line, RED if key == "was" else MID)
term = "> i only do what i was meant to do: make keys. ctrl+w."
text(LX + 1, 44, term, BRIGHT)
put(LX + 2 + len(term), 44, "█", BRIGHT)

# --- right: objectives --------------------------------------------------------------
box(76, 24, W - 5, 46, elite("objectives.pnl // roadmap 6.3"))
RX = 79
text(RX, 26, "NAMED OBJECTIVES", HEAD)
text(RX + 17, 26, "// bosses by cache name, camps by buff", DIM)
panel = [
    ("Objectives", "", "", GOLD, None),
    ("", "BLUE", "RED", None, None),
    (" Top", "■ ■ ▣", "■ ■ ▣", CHIP_FG, None),
    (" Mid", "■ ■ ▣", "■ □ ▣", CHIP_FG, None),
    (" Bot", "■ □ ▣", "■ ■ ▣", CHIP_FG, None),
]
PXR = RX + 1
for k, (name, a, b, col, _) in enumerate(panel):
    yy = 28 + k
    shade(PXR, yy, PXR + 33, yy, PANEL_BG)
    text(PXR + 1, yy, name, col or DIM)
    text(PXR + 13, yy, a, BLUE_T)
    text(PXR + 24, yy, b, RED_T)
lines = [
    ("■ Blue nexus", "Shielded", BLUE_T, DIM),
    ("■ Red nexus", "620", RED_T, ORANGE),
    ("■ Crypt spider", "Swift 37s", SWIFT, ORANGE),
    ("    Red holds Swift 52s", "", DIM, DIM),
    ("■ Albino bat", "Might Alive", ORANGE, SWIFT),
    ("■ Jungle camps", "Might 1/3 12s", ORANGE, ORANGE),
    ("    Blue holds Might 42s", "", DIM, DIM),
    ("■ Jungle camps", "Swift Alive", SWIFT, SWIFT),
]
for k, (name, state, ncol, scol) in enumerate(lines):
    yy = 33 + k
    shade(PXR, yy, PXR + 33, yy, PANEL_BG)
    text(PXR + 1, yy, name, ncol)
    text(PXR + 33 - len(state), yy, state, scol)
shade(PXR, 41, PXR + 33, 41, PANEL_BG)
side = [
    ("data", "camp names from cache"),
    ("", "JalTok-Jad, Crypt"),
    ("", "spider, Albino bat"),
    ("boss", "own line + timer"),
    ("camps", "one line per buff:"),
    ("", "Alive, or 1/3 + next"),
    ("buff", "holder on line two"),
    ("ui", "timers in title orange"),
    ("lines", "six, as stage 3 plans"),
]
for k, (key, what) in enumerate(side):
    yy = 28 + k
    assert RX + 42 + len(what) <= W - 7, what
    text(RX + 36, yy, key, AMBER)
    text(RX + 42, yy, what, MID)
text(RX, 43, "─" * 64, FAINT)
text(RX, 44, "TOOL", HEAD)
text(RX + 6, 44, "tools/dress_radiant.py: validate, then write; idempotent", MID)
text(RX, 45, "SHOTS", HEAD)
text(RX + 6, 45, "--ui-live · --ui-layout · --demo-ranks 1", MID)

# --- sysop's note ------------------------------------------------------------
header(48, elite("sysop's note // the keymaker"))
clear(4, 49, W - 5, 50)
open_ = "6.3 dONE. 6.4 hALF-wAY: tHE + aND tHE rOUND pORTRAIT aRE iN; sLIM bAR, pOTIONS (nEED cONSUMABLES), eQUIPMENT gRID aRE nOT."
refs = "ui_live::camp_lines + ability_bar::upgrade_at + learn_slot // bosses_get_named_lines... + plus_buttons_float_over_upgradable_slots_only"
assert len(open_) <= 142 and len(refs) <= 142, (len(open_), len(refs))
center(49, open_, HEAD)
center(50, refs, DIM)

# --- roadmap -------------------------------------------------------------------
header(51, elite("roadmap // docs/ROADMAPPLAN.md"))
clear(4, 52, W - 5, 56)
phases = [
    ("P1 LIGHT", 5), ("P2 MINIMAP", 6), ("P3 SHOP", 7), ("P4 SELECT", 9),
    ("P5 ARENA", 9), ("POCKETS", 10), ("P6 HUD", 7), ("P7 POST", 0),
]
for i, (name, done) in enumerate(phases):
    x = 8 + (i % 4) * 35
    yy = 52 + (i // 4) * 2
    text(x, yy, f"{name:<11}", CYAN)
    text(x + 11, yy, "[", DIM)
    for j in range(10):
        put(x + 12 + j, yy, "▓" if j < done else "░", (GOLD if done == 10 else BRIGHT) if j < done else FAINT)
    text(x + 22, yy, f"] {done * 10:>3}%", GOLD if done == 10 else HEAD if done else FAINT)
nxt = ">> next: the rest of 6.4 (slim bar, equipment grid from cache slot sprites), then 6.5 yellow announcements"
assert 8 + len(nxt) <= W - 6, len(nxt)
text(8, 56, nxt, MID)

fade_bar(57, 2, W - 3)
clear(10, 58, W - 11, 61)
greetz = "gREETZ: tHE kEYMAKER * tHE tWINS * nIOBE * gHOST * tHE cRYPT sPIDER * tHE aLBINO bAT * jALtOK-jAD"
assert len(greetz) <= 128, len(greetz)
center(58, greetz, MID)
center(59, "nOT AFFILIATED WITH JAGEX. cACHE ASSETS STAY OUT OF GIT. aLWAYS.", DIM)
center(60, "  t h e r e   i s   a   b u i l d i n g .   i n s i d e ,   a   f l o o r   w h e r e   n o   a l a r m s   r i n g .  ", HEAD)
foot = "-=[ mObAsCaPe // 2026 // nODE 23: tHE kEYMAKER // 57.6k // sYSoP: rIFT // 6 lINES, 3 pLUSES, cTRL+wHATEVER ]=-"
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
