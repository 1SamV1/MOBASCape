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
def stencil_logo(y, word):
    """Stencil logo: letters cut by a horizontal gap at mid-height and
    framed top and bottom with ▄▀ rails, like a BBS sysop's door banner."""
    width = len(word) * 10 - 1
    x0 = (W - width) // 2
    clear(x0 - 4, y - 2, x0 + width + 4, y + LOGO_ROWS + 2)
    for x in range(x0 - 3, x0 + width + 3):
        put(x, y - 1, "▄", DIM if x % 2 else MID)
        put(x, y + LOGO_ROWS + 1, "▀", DIM if x % 2 else MID)
    for i, letter in enumerate(word):
        for r in range(LOGO_ROWS):
            for c, px in enumerate(LOGO[letter][r]):
                if px == " ":
                    continue
                row = r if r < 3 else r + 1
                put(x0 + i * 10 + c, y + row, px, LOGO_GRAD[min(r, len(LOGO_GRAD) - 1)])
    for x in range(x0 - 3, x0 + width + 3, 4):
        put(x, y + 3, "·", FAINT)


stencil_logo(3, "MOBASCAPE")
fade_bar(1, 2, W - 3)
center(12, "  ░▒▓  t H E   o R A C L E   r E C O M M E N D S  ::  1 3   i T E M S   f O R   y O U  ▓▒░  ", AMBER)
center(13, "  " + elite("phase.3 // know thy build") + "  ", WHITE)
fade_bar(14, 2, W - 3)

# --- release info -------------------------------------------------------------
header(16, elite("release info"))
clear(4, 17, W - 5, 19)
info = [
    ("rELEASE dATE", "2026-11-24", "cOMMIT", "815f2f6  ->  sam/main"),
    ("tYPE", "rECOMMENDED tAB + fIX", "tESTS", "396 PASS // 0 FAIL   (+2)"),
    ("lISTS", "MELEE RANGED MAGIC TANK", "fIXED", "detail != buy target. now one."),
]
for i, (k1, v1, k2, v2) in enumerate(info):
    y = 17 + i
    text(8, y, f"{k1:.<16}", DIM)
    text(25, y, v1, AMBER if k1 == "lISTS" else BRIGHT)
    text(76, y, f"{k2:.<12}", DIM)
    text(89, y, v2, RED if k2 == "fIXED" else HEAD)

# --- left: BBS door menu -------------------------------------------------------
box(4, 21, 66, 44, elite("door.exe // the oracle"))
text(8, 23, "╔══════════════════════════════════════════════════╗", DIM)
text(8, 24, "║", DIM); text(10, 24, "THE ORACLE's ITEM DOOR  v1.0       node 6 of 6", HEAD); text(59, 24, "║", DIM)
text(8, 25, "╠══════════════════════════════════════════════════╣", DIM)
menu = [
    ("[1]", "consult by HERO ID", "jad, kril, kree ..."),
    ("[2]", "consult by ROLE WORD", "TANK"),
    ("[3]", "consult by CLASS", "MELEE / RANGED / MAGIC"),
    ("[4]", "no list? open on", "ALL"),
]
for i, (k, what, eg) in enumerate(menu):
    y = 26 + i * 2
    text(8, y, "║", DIM); text(59, y, "║", DIM)
    text(10, y, k, AMBER)
    text(14, y, what, WHITE)
    text(36, y, eg, CYAN)
    text(8, y + 1, "║", DIM); text(59, y + 1, "║", DIM)
text(8, 34, "╚══════════════════════════════════════════════════╝", DIM)
text(8, 36, "Your choice? _", (120, 255, 140))
put(21, 36, "█", BRIGHT)
text(8, 38, "> kolodion :: BATTLE MAGE :: MAGIC", MID)
text(8, 39, "  id? no.  BATTLE? no.  MAGE? no.  MAGIC? YES.", MID)
text(8, 40, "  13 items. starter -> late. shop opens here.", (120, 255, 140))
text(8, 42, "first match wins. _comments ignored. typos fail.", DIM)

# --- right: the list as a file listing -----------------------------------------
box(68, 21, W - 5, 44, elite("recommended.json // magic"))
text(71, 23, "FILENAME            TIER     SIZE(gp)  DESC", HEAD)
text(71, 24, "─" * 70, DIM)
listing = [
    ("STAFF_AIR.ITM", "starter", "300", "where every mage begins"),
    ("BOOTS_LTH.ITM", "starter", "200", "leather. humble. fast."),
    ("BOOTS_MYS.ITM", "early", "900", "mystic boots"),
    ("STAFF_FIR.ITM", "early", "850", "builds into mystic fire"),
    ("ROBE_MYS.ITM ", "core", "1400", "mystic robe top"),
    ("STAFF_MFS.ITM", "core", "1400", "the upgrade lands"),
    ("RING_SEER.ITM", "core", "2000", "seers ring"),
    ("NECK_OCLT.ITM", "core", "2800", "occult necklace"),
    ("ROBE_AHR.ITM ", "late", "3200", "ahrim's robetop"),
    ("TRIDENT.ITM  ", "late", "3800", "trident of the seas"),
    ("BOOTS_ETR.ITM", "late", "4200", "eternal boots"),
    ("ROBE_ANC.ITM ", "late", "4800", "ancestral robe top"),
    ("RING_MAGS.ITM", "late", "5000", "magus ring. endgame."),
]
tier_col = {"starter": (120, 255, 140), "early": CYAN, "core": AMBER, "late": RED}
for i, (name, tier, size, desc) in enumerate(listing):
    y = 25 + i
    text(71, y, name, WHITE)
    text(91, y, tier, tier_col[tier])
    text(100, y, f"{size:>6}", HEAD)
    text(111, y, desc, DIM)
text(71, 39, "─" * 70, DIM)
text(71, 40, "13 file(s)   30,850 gp total   bytes free: your gold", MID)
text(71, 42, "[D]ownload = Buy    [S]ell = 70%    [U]ndo = 100%", AMBER)

# --- patch notes ---------------------------------------------------------------
header(46, elite("patch notes // two minds, one item"))
clear(4, 47, W - 5, 49)
center(47, "tHE dETAIL pANE sHOWED sTAFF oF aIR wHILE bUY pOINTED aT a bRONZE sWORD tHE tAB hID.  ItemShop::shown() nOW rULES tHEM aLL.", HEAD)
center(48, "data/items/recommended.json   //   recommend_for(id, role, class)   //   CATEGORY_LABELS[0] = RECOMMENDED", DIM)

# --- roadmap -------------------------------------------------------------------
header(51, elite("roadmap // the oracle says"))
clear(4, 52, W - 5, 56)
phases = [
    ("P1 LIGHT", 9), ("P2 MINIMAP", 5), ("P3 SHOP", 8), ("P4 SELECT", 1),
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
text(8, 55, ">> next: multi-part late recipes, then P4 hero select (lanes, team columns, battle spells)", MID)

fade_bar(57, 2, W - 3)
clear(10, 58, W - 11, 61)
center(58, "gREETZ: tHE oRACLE * tHE dOOR gAME sYSOPS * lORD * tRADEWARS * eVERY 'wHAT dO i bUY' iN aLL cHAT", MID)
center(59, "nOT AFFILIATED WITH JAGEX. cACHE ASSETS STAY OUT OF GIT. aLWAYS.", DIM)
center(60, "  y o u ' r e   n o t   t h e   o n e .   b u t   y o u r   b u i l d   c o u l d   b e .  ", HEAD)
center(61, "-=[ mObAsCaPe // 2026 // nODE 6: tHE oRACLE // 57.6k // sYSoP: rIFT // cOOKIE iNCLUDED ]=-", AMBER)

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
