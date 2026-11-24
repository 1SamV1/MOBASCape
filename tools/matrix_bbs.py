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
def bone_logo(y, word):
    """X-ray logo: every filled cell of the block font becomes a bone joined
    to its neighbours (box-drawing lines), with ● at the knuckles where three
    or more bones meet. The letters' skeleton, nothing else."""
    width = len(word) * 10 - 1
    x0 = (W - width) // 2
    clear(x0 - 3, y - 1, x0 + width + 3, y + LOGO_ROWS)
    cells = set()
    for i, letter in enumerate(word):
        for r in range(LOGO_ROWS):
            for c, px in enumerate(LOGO[letter][r]):
                if px != " ":
                    cells.add((x0 + i * 10 + c, y + r))
    lines = {
        (0, 0, 0, 0): "∙", (1, 0, 0, 0): "╵", (0, 1, 0, 0): "╷", (0, 0, 1, 0): "╴", (0, 0, 0, 1): "╶",
        (1, 1, 0, 0): "│", (0, 0, 1, 1): "─", (1, 0, 1, 0): "┘", (1, 0, 0, 1): "└",
        (0, 1, 1, 0): "┐", (0, 1, 0, 1): "┌", (1, 1, 1, 0): "┤", (1, 1, 0, 1): "├",
        (1, 0, 1, 1): "┴", (0, 1, 1, 1): "┬", (1, 1, 1, 1): "┼",
    }
    for (x, yy) in sorted(cells):
        key = ((x, yy - 1) in cells, (x, yy + 1) in cells, (x - 1, yy) in cells, (x + 1, yy) in cells)
        n = sum(key)
        ch = "●" if n >= 3 and (x + yy) % 3 == 0 else lines[key]
        col = WHITE if ch == "●" else LOGO_GRAD[yy - y]
        put(x, yy, ch, col)


bone_logo(3, "MOBASCAPE")
fade_bar(1, 2, W - 3)
center(10, "  ░▒▓  t H E   x - R A Y  ::  3 6   p A R T S   b O U N D   //   0   l E F T   b E H I N D  ▓▒░  ", AMBER)
center(11, "  " + elite("there is no spoon. there was a frozen arm.") + "  ", WHITE)
fade_bar(12, 2, W - 3)

# --- release info -------------------------------------------------------------
header(14, elite("release info"))
clear(4, 15, W - 5, 18)
info = [
    ("rELEASE dATE", "2026-11-24", "cOMMIT", "14f17b4 + 51807e7 -> sam/main"),
    ("tYPE", "sKELETON bINDING + lIMB iD", "tESTS", "405 PASS // 0 FAIL   (+5)"),
    ("bEFORE", "PARTS FROZEN IN REST POSE", "dRIFT", "0.015 - 0.758 of hero height"),
    ("aFTER", "EVERY PART <= 0.03 (A TEST)", "bONUS", "evergreens lost the bark ring"),
]
for i, (k1, v1, k2, v2) in enumerate(info):
    y = 15 + i
    text(8, y, f"{k1:.<16}", DIM)
    text(25, y, v1, RED if k1 == "bEFORE" else AMBER if k1 == "aFTER" else BRIGHT)
    text(76, y, f"{k2:.<12}", DIM)
    text(89, y, v2, HEAD)

# --- left: drift table ----------------------------------------------------------
box(4, 20, 74, 44, elite("drift.log // frozen -> bound"))
text(8, 22, "hERO              pART       fROZEN    bOUND   hOW", DIM)
text(8, 23, "─" * 62, FAINT)
rows = [
    ("Kree'arra Jr.", "28021", 0.758, 0.000, "NATIVE"),
    ("Venenatis", "28295", 0.464, 0.002, "RIGID"),
    ("K'ril Tsutsaroth", "27681", 0.148, 0.009, "NATIVE"),
    ("Vorkath", "41468", 0.129, 0.003, "NATIVE"),
    ("King Awowogei", "4823", 0.129, 0.027, "NATIVE"),
    ("Kolodion", "hands 176", 0.119, 0.002, "NATIVE"),
    ("Kalphite Queen", "24598", 0.096, 0.004, "NATIVE"),
    ("Cerberus", "41449", 0.085, 0.001, "NATIVE"),
    ("Zilyana", "27968", 0.082, 0.004, "NATIVE"),
    ("General Graardor", "27665", 0.078, 0.003, "NATIVE"),
    ("Wise Old Man", "staff 3711", 0.024, 0.001, "RIGID"),
]
for i, (hero, part, before, after, how) in enumerate(rows):
    y = 24 + i
    text(8, y, f"{hero:<18}", WHITE)
    text(26, y, f"{part:<11}", MID)
    bar = min(10, max(1, round(before * 13)))
    text(37, y, f"{before:.3f}", RED)
    text(47, y, f"{after:.3f}", BRIGHT)
    text(55, y, how, AMBER if how == "RIGID" else CYAN)
    for k in range(bar):
        put(63 + k, y, "▓", (120 + 13 * k, 40, 30))
text(8, 36, "─" * 62, FAINT)
text(8, 37, "36 attachment parts:  28 NATIVE  //  6 FOLLOW  //  2 RIGID", HEAD)
text(8, 39, "nATIVE  keep OSRS labels (what the real client does)", CYAN)
text(8, 40, "fOLLOW  each vertex rides the nearest bound limb in its region", MID)
text(8, 41, "rIGID   one label, one solid object: staffs, shells. never pairs", AMBER)
text(8, 42, "sCORE   drift + mesh stretch. followers never move a pivot", DIM)

# --- right: the x-ray ---------------------------------------------------------
box(76, 20, W - 5, 44, elite("xray.scr // limb identification"))
figure = [
    "                 .-\"\"\"-.                     ",
    "                ( ●   ● )       ◄── HEAD     ",
    "                 \\  ═  /        down < 0.22  ",
    "         ┌────────┴───┴────────┐             ",
    " ARM L ─►│  ●═══╗  TORSO  ╔═══●  │◄─ ARM R   ",
    "  side   │      ║  ┼┼┼┼┼  ║      │   > 0.38  ",
    " < -0.38 ●      ║  ┼┼┼┼┼  ║      ●           ",
    "         ║      ╚═══╦═╦═══╝      ║           ",
    "      [HAND]        ║ ║        [STAFF]═══════",
    "                 ┌──╨─╨──┐     RIGID: one bone",
    "                 │ ROBE  │◄── FOLLOW hips,   ",
    "                 │ ░▒▓▒░ │    never the arm  ",
    "                 └─┬───┬─┘                   ",
    "         LEG L ───►●   ●◄─── LEG R           ",
    "        down>0.62  ║   ║                     ",
    "                  ═╩═ ═╩═   BOOTS: a pair,   ",
    "                            can't go rigid   ",
]
for i, line in enumerate(figure):
    for j, ch in enumerate(line):
        if ch == " ":
            continue
        if ch == "●":
            col = WHITE
        elif ch in "═║╔╗╚╝╦╩╨┼":
            col = (225, 225, 200)
        elif ch in "░▒▓":
            col = (40, 160, 70)
        elif ch.isupper() or ch in "◄►":
            col = AMBER
        elif ch in "┌┐└┘─│┬┴":
            col = DIM
        else:
            col = MID
        put(79 + j, 22 + i, ch, col)
text(80, 40, "label centroid vs rest-pose box (cache Y grows DOWN)", MID)
text(80, 41, "span > 0.70 = ROOT  //  |back| > 0.55 = TAIL", MID)
text(80, 42, "a vertex follows only its own limb, or the core", (120, 255, 140))

# --- sysop's note --------------------------------------------------------------
header(46, elite("sysop's note // left on your desk"))
clear(4, 47, W - 5, 49)
center(47, "kOLODION'S bASE mODEL iS jUST hIS sLEEVES.  sO tHE sKELETON gROWS aS pARTS bIND:  hANDS, fEET, cHEST, tHEN tHE rOBE.", HEAD)
center(48, "skeleton::assemble()   //   identify_limbs()   //   Model::follow_groups   //   rift-tools skeleton all --sheet", DIM)

# --- roadmap -------------------------------------------------------------------
header(51, elite("roadmap // the oracle says"))
clear(4, 52, W - 5, 56)
phases = [
    ("P1 LIGHT", 9), ("P2 MINIMAP", 5), ("P3 SHOP", 8), ("P4 SELECT", 3),
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
text(8, 55, ">> next in P4: lane picker (Top/Jungle/Mid/Carry/Support), battle spells, bots that play their pick", MID)

fade_bar(57, 2, W - 3)
clear(10, 58, W - 11, 61)
center(58, "gREETZ: gRAARDOR'S fISTS (rEUNITED) * kREE'ARRA'S bIRD (nO lONGER 0.758 aWAY) * tHE rOBE tHAT wANTED tO bE aN aRM", MID)
center(59, "nOT AFFILIATED WITH JAGEX. cACHE ASSETS STAY OUT OF GIT. aLWAYS.", DIM)
center(60, "  f r e e   y o u r   m i n d .   t h e n   f r e e   y o u r   h a n d s   f r o m   t h e   r e s t   p o s e .  ", HEAD)
center(61, "-=[ mObAsCaPe // 2026 // nODE 8: tHE x-rAY // 57.6k // sYSoP: rIFT // 36 pARTS, 1 sKELETON ]=-", AMBER)

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
