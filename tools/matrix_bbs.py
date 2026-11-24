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
def glitch_logo(y, word):
    """Logo with horizontal tearing: some rows slip sideways, with RGB
    ghosts, like a scrambled signal on a 2400-baud line."""
    width = len(word) * 10 - 1
    x0 = (W - width) // 2
    clear(x0 - 6, y - 1, x0 + width + 6, y + LOGO_ROWS)
    slip = [0, 2, 0, -3, 1, 0]
    for r in range(LOGO_ROWS):
        for i, letter in enumerate(word):
            for c, px in enumerate(LOGO[letter][r]):
                if px == " ":
                    continue
                x = x0 + i * 10 + c + slip[r]
                if slip[r]:
                    put(x - slip[r], y + r, "░", (255, 60, 60) if slip[r] > 0 else (60, 160, 255))
                put(x, y + r, px, LOGO_GRAD[r])
    for k, (r, x) in enumerate([(1, x0 - 5), (3, x0 + width + 3), (4, x0 - 4)]):
        text(x, y + r, "▌▐"[k % 2] * 2, RED if k % 2 else CYAN)


fade_bar(1, 2, W - 3)
glitch_logo(3, "MOBASCAPE")
center(10, '  ░▒▓  g R E P   t H E   e X C H A N G E   ::   s E A R C H   "sword"  ▓▒░  ', AMBER)
center(11, "  " + elite("phase.3 // seek and ye shall find") + "  ", WHITE)
fade_bar(12, 2, W - 3)

# --- release info -------------------------------------------------------------
header(14, elite("release info"))
clear(4, 15, W - 5, 18)
info = [
    ("rELEASE dATE", "2026-11-24", "cOMMIT", "d7bdcf1  ->  sam/main"),
    ("tYPE", "sHOP sEARCH + bUGFIX", "tESTS", "394 PASS // 0 FAIL   (+2)"),
    ("mATCH", "ALL WORDS, ANY FIELD", "cLIPPY", "-D warnings :: CLEAN"),
    ("fIXED", "LOBBY CARDS LINGERING", "kEYS", "box eats them: no stray Q casts"),
]
for i, (k1, v1, k2, v2) in enumerate(info):
    y = 15 + i
    text(8, y, f"{k1:.<16}", DIM)
    text(25, y, v1, AMBER if k1 == "mATCH" else (RED if k1 == "fIXED" else BRIGHT))
    text(76, y, f"{k2:.<12}", DIM)
    text(89, y, v2, HEAD)

# --- left: the terminal session ------------------------------------------------
box(4, 20, 72, 44, elite("tty1 // the exchange"))
session = [
    ("rift@exchange:~$ ", "search staff fire", WHITE),
    ("", "  > Staff of fire ............ 850   [magic]", CYAN),
    ("", "  > Mystic fire staff ....... 1400   [magic]", CYAN),
    ("", "", MID),
    ("rift@exchange:~$ ", "search mana", WHITE),
    ("", "  > every item that GIVES mana. stats are words too.", MID),
    ("", "", MID),
    ("rift@exchange:~$ ", "search fire sword", WHITE),
    ("", "  No items match the search.", (200, 90, 90)),
    ("", "  (every word must hit. no sword is on fire. yet.)", DIM),
    ("", "", MID),
    ("rift@exchange:~$ ", "search sword", WHITE),
    ("", "  > 8 items // starter 1 // early 3 // core 3 // late 1", (120, 255, 140)),
    ("", "  > selection kept: Steel sword (still matches)", MID),
    ("", "", MID),
    ("rift@exchange:~$ ", "q", WHITE),
    ("", "  > typed. not cast. the box holds the keyboard.", AMBER),
    ("", "", MID),
    ("rift@exchange:~$ ", "<ESC>", WHITE),
    ("", "  > cleared. back to the fight.", MID),
]
for i, (prompt, cmd, col) in enumerate(session):
    y = 22 + i
    if prompt:
        text(8, y, prompt, (120, 255, 140))
        text(8 + len(prompt), y, cmd, col)
    else:
        text(8, y, cmd, col)
put(8 + len("rift@exchange:~$ "), 42, "█", BRIGHT)
text(8, 42, "rift@exchange:~$ ", (120, 255, 140))

# --- right: the matcher diagram -------------------------------------------------
box(74, 20, W - 5, 44, elite("matcher.c // the sieve"))
diagram = [
    '      query: "staff fire"',
    "            │",
    "      ┌─────┴─────┐  split on whitespace, lowercase",
    "      ▼           ▼",
    '   "staff"      "fire"',
    "      │           │",
    "      ▼           ▼",
    " ╔═══════════════════════════════════════╗",
    " ║ name      Staff of fire        ✓  ✓   ║",
    " ║ id        staff of fire        ✓  ✓   ║",
    " ║ category  magic                       ║",
    " ║ stats     ability power               ║",
    " ╚═══════════════════════════════════════╝",
    "              │  ALL words hit?",
    "              ▼",
    "        [ SHOWN IN THE GRID ]",
]
for i, line in enumerate(diagram):
    for j, ch in enumerate(line):
        col = BRIGHT if ch in "│┌┐┴▼─" else (DIM if ch in "╔╗╚╝═║" else ((120, 255, 140) if ch == "✓" else MID))
        if ch != " ":
            put(78 + j, 22 + i, ch, col)
text(120, 30, "", MID)
text(78, 39, "click box   -> gold frame + caret", HEAD)
text(78, 40, "any click   -> stop typing", MID)
text(78, 41, "close shop  -> search forgotten", MID)
text(78, 42, "held keys   -> released, no runaway camera", DIM)

# --- the bugfix ---------------------------------------------------------------
header(46, elite("patch notes // the ghost in the lobby"))
clear(4, 47, W - 5, 49)
center(47, "tHE 5v5 tEAM cARDS hAUNTED tHE aRENA: tHE oLD hUD mESH wAS uPLOADED eVERY fRAME bUT nEVER cLEARED.  eXORCISED.", HEAD)
center(48, "items::matches_query()   //   ShopHit::Search   //   ItemShop { query, typing }   //   hud.vertices.clear()", DIM)

# --- roadmap -------------------------------------------------------------------
header(51, elite("roadmap // the oracle says"))
clear(4, 52, W - 5, 56)
phases = [
    ("P1 LIGHT", 9), ("P2 MINIMAP", 5), ("P3 SHOP", 7), ("P4 SELECT", 1),
    ("P5 ARENA", 0), ("P6 HUD", 0), ("P7 POST", 0),
]
for i, (name, done) in enumerate(phases):
    x = 8 + (i % 4) * 35
    y = 52 + (i // 4) * 2
    text(x, y, f"{name:<11}", CYAN)
    text(x + 11, y, "[", DIM)
    for j in range(10):
        put(x + 12 + j, y, "▓" if j < done else "░", BRIGHT if j < done else FAINT)
    text(x + 22, y, f"] {done * 10:>3}%", HEAD if done else FAINT)
text(8, 55, ">> next in P3: recommended page, multi-part late recipes", MID)

fade_bar(57, 2, W - 3)
clear(10, 58, W - 11, 61)
center(58, "gREETZ: gREP * tHE sEARCH bOX oN eVERY bBS fILE aREA * aNYONE wHO eVER tYPED /msg iNTO tHE wRONG wINDOW", MID)
center(59, "nOT AFFILIATED WITH JAGEX. cACHE ASSETS STAY OUT OF GIT. aLWAYS.", DIM)
center(60, "  t h e   m a t r i x   h a s   y o u .   t y p e   ' s w o r d '   t o   c o n t i n u e .  ", HEAD)
center(61, "-=[ mObAsCaPe // 2026 // nODE 5: tHE iNDEX // 57.6k // sYSoP: rIFT // 8 mATCHES fOUND ]=-", AMBER)

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
