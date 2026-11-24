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
import math

VIOLET = (190, 140, 255)
GOLD = (255, 210, 90)
EMBER = (255, 120, 60)


def extruded_logo(y, word):
    """ACiD-style 3D block logo: the face in the green gradient, extruded
    two cells down-right in ▓▒ shade, with a ░ cast shadow beneath."""
    width = len(word) * 10 - 1
    x0 = (W - width) // 2 - 1
    clear(x0 - 2, y - 1, x0 + width + 5, y + LOGO_ROWS + 2)
    cells = {}
    for i, letter in enumerate(word):
        for r in range(LOGO_ROWS):
            for c, px in enumerate(LOGO[letter][r]):
                if px != " ":
                    cells[(x0 + i * 10 + c, y + r)] = px
    def enclosed(x, yy):
        # Inside a letter's counter (face cells both sides on the row):
        # the inner wall is drawn dark so the letter still reads.
        return any((x - k, yy) in cells for k in range(1, 4)) and any(
            (x + k, yy) in cells for k in range(1, 8))

    for (dx, dy, ch, col) in ((3, 2, "░", (0, 40, 10)), (2, 1, "▒", (0, 90, 25)), (1, 1, "▓", (0, 140, 35))):
        for (x, yy) in cells:
            tx, ty = x + dx, yy + dy
            if (tx, ty) not in cells:
                inner = enclosed(tx, ty)
                put(tx, ty, "░" if inner else ch, (0, 45, 12) if inner else col)
    for (x, yy), px in cells.items():
        put(x, yy, px if px in "▀▄" else "█", LOGO_GRAD[yy - y])


def rune_circle(cx, cy, rx, ry, runes, col_ring, col_rune):
    """A summoning circle: double ring at 2:1 aspect, runes on the outer
    ring, a hexagram of ∙ chords inside, a ◆ at the heart."""
    clear(cx - rx - 1, cy - ry, cx + rx + 1, cy + ry)
    for k in range(240):
        t = k / 240 * math.tau
        for (sx, sy, ch) in ((1.0, 1.0, "·"), (0.78, 0.74, "∙")):
            x = cx + round(math.cos(t) * rx * sx)
            yy = cy + round(math.sin(t) * ry * sy)
            put(x, yy, ch, col_ring if sx == 1.0 else DIM)
    pts = [(cx + math.cos(math.tau * i / 6 - math.pi / 2) * rx * 0.7,
            cy + math.sin(math.tau * i / 6 - math.pi / 2) * ry * 0.7) for i in range(6)]
    for tri in ((0, 2, 4), (1, 3, 5)):
        for j in range(3):
            (ax, ay), (bx, by) = pts[tri[j]], pts[tri[(j + 1) % 3]]
            for k in range(30):
                f = k / 29
                put(round(ax + (bx - ax) * f), round(ay + (by - ay) * f), "·", FAINT if tri[0] else (60, 40, 100))
    for i, ch in enumerate(runes):
        t = math.tau * i / len(runes) - math.pi / 2
        put(cx + round(math.cos(t) * rx), cy + round(math.sin(t) * ry), ch, col_rune)
    put(cx, cy, "◆", WHITE)


extruded_logo(3, "MOBASCAPE")
rune_circle(12, 6, 9, 4, "Q✶W†E‡R§", VIOLET, GOLD)
rune_circle(W - 13, 6, 9, 4, "★Q☩W✦E☼R", VIOLET, GOLD)
fade_bar(1, 2, W - 3)
center(12, "  ░▒▓  n O D E   1 0 :   t H E   g R I M O I R E   //   9   b O T S   l E A R N   t O   c A S T  ▓▒░  ", AMBER)
center(13, "  " + elite("q. w. e. r. read from the rank tables, spoken in the lane.") + "  ", WHITE)
fade_bar(14, 2, W - 3)

# --- release info ---------------------------------------------------------------
header(16, elite("release info"))
clear(4, 17, W - 5, 20)
info = [
    ("rELEASE dATE", "2026-11-24", "cOMMIT", "f7d6170  ->  sam/main"),
    ("tYPE", "bOT aBILITIES", "tESTS", "410 pass // clippy clean"),
    ("bUG", "BOTS SPAWNED WITH 0 MANA", "fIX", "champion pool, +20/level"),
    ("aDR", "055: cast from the data", "cASTS", "110 in a 6:00 sim"),
]
for i, (k1, v1, k2, v2) in enumerate(info):
    yy = 17 + i
    text(8, yy, f"{k1:.<16}", DIM)
    text(25, yy, v1, RED if k1 == "bUG" else BRIGHT)
    text(76, yy, f"{k2:.<12}", DIM)
    text(89, yy, v2, HEAD)

# --- left: the spellbook ----------------------------------------------------------
box(4, 22, 74, 46, elite("spellbook.dat // four shapes"))
cards = [
    ("CARE", "no damage in the ranks", [
        "     +  ✚  +     ",
        "   ✚    @    ✚   ",
        "      /█\\        ",
        "      / \\        ",
        " heal 25% missing",
        " or shield 15%   ",
        " when hp < 60%   ",
    ], (110, 230, 120)),
    ("NOVA", "range 0", [
        "   · ∙ ∙ ∙ ·     ",
        " ∙  ░▒▒▒▒░  ∙    ",
        "∙  ▒  @   ▒  ∙   ",
        " ∙  ░▒▒▒▒░  ∙    ",
        "   · ∙ ∙ ∙ ·     ",
        " r 320 round the ",
        " caster          ",
    ], (255, 190, 80)),
    ("STRIKE", "range <= 450", [
        "                 ",
        " @ ════► ✸  m    ",
        "       ░▒█▒░     ",
        "     m  ░▒░  m   ",
        "                 ",
        " blast + splash  ",
        " 170 at target   ",
    ], (120, 200, 255)),
    ("BOLT", "range > 450", [
        "                 ",
        " @ ·∙•○◎●═══>  X ",
        "                 ",
        "  homing: never  ",
        "  misses         ",
        " stun/root/slow  ",
        " land up front   ",
    ], (200, 140, 255)),
]
for i, (name, rule, art, col) in enumerate(cards):
    cx = 7 + (i % 2) * 34
    cy = 24 + (i // 2) * 11
    text(cx, cy, f"[{name}]", col)
    text(cx + len(name) + 3, cy, rule, DIM)
    for j, line in enumerate(art):
        for k, ch in enumerate(line):
            if ch == " ":
                continue
            c = col if ch in "✚+░▒█✸●◎○•∙·═►>" else WHITE if ch == "@" else RED if ch in "Xm" else MID
            put(cx + k, cy + 1 + j, ch, c)
text(8, 45, "ranks: Q@1 W@2 E@3 R@6, +1 every 3 lv  //  mana 2%/s", HEAD)

# --- right: the cast log ------------------------------------------------------------
box(76, 22, W - 5, 46, elite("cast.log // 6:00 headless sim"))
casts = [
    ("Kree'arra Jr.", 27, "ARCANE LANCE"), ("JalRek-Jad", 14, "strike"),
    ("Cerberus", 12, "TRIPLE BITE + LAVA"), ("Big Frog", 12, "TONGUE LASH"),
    ("Kolodion", 11, "3 GOD SPELLS"), ("General Graardor", 11, "SMASH, BODYGUARDS"),
    ("Ancient Wyvern", 9, "ICY BREATH"), ("Callisto", 7, "MAUL"),
    ("Kalphite Queen", 7, "MANDIBLE CRUSH"),
]
for i, (who, n, what) in enumerate(casts):
    yy = 24 + i * 2
    text(79, yy, f"{who:<17}", CYAN if i % 2 == 0 else (90, 210, 180))
    for k in range(n):
        shade = "█" if k < n - 2 else "▓" if k < n - 1 else "▒"
        put(97 + k, yy, shade, BRIGHT if k < 14 else AMBER)
    text(97 + n + 1, yy, f"{n:>2}", WHITE)
    text(97 + n + 4, yy, what.lower() if what.isupper() else what, DIM)
text(79, 42, "SHAPES  bolt 65  ███████████████████████████▒", (200, 140, 255))
text(79, 43, "        strike 43 ██████████████████▒", (120, 200, 255))
text(79, 44, "        nova 2 ▒   care 0 (nobody fell under 60% mid-fight)", (255, 190, 80))

# --- sysop's note ------------------------------------------------------------------
header(48, elite("sysop's note // read the margins"))
clear(4, 49, W - 5, 50)
center(49, "aN aPPROXIMATION, sIGNED aND dATED: dASHES, sUMMONS aND zONES rEAD aS bOLTS oR sTRIKES fOR nOW.", HEAD)
center(50, "the player's kit runtime is single-caster; making it caster-generic lets bots cast the real kits (ADR-055)", DIM)

# --- roadmap -------------------------------------------------------------------
header(52, elite("roadmap // the oracle says"))
clear(4, 53, W - 5, 56)
phases = [
    ("P1 LIGHT", 9), ("P2 MINIMAP", 5), ("P3 SHOP", 8), ("P4 SELECT", 5),
    ("P5 ARENA", 2), ("P6 HUD", 0), ("P7 POST", 0),
]
for i, (name, done) in enumerate(phases):
    x = 8 + (i % 4) * 35
    yy = 53 + (i // 4) * 2
    text(x, yy, f"{name:<11}", CYAN)
    text(x + 11, yy, "[", DIM)
    for j in range(10):
        put(x + 12 + j, yy, "▓" if j < done else "░", BRIGHT if j < done else FAINT)
    text(x + 22, yy, f"] {done * 10:>3}%", HEAD if done else FAINT)
text(8, 56, ">> next: caster-generic kits, bot dashes, lane picker (Top/Jungle/Mid/Carry/Support)", MID)

fade_bar(57, 2, W - 3)
clear(10, 58, W - 11, 61)
center(58, "gREETZ: kREE'ARRA jR. (27 lANCES) * jALREK-jAD (tHE sTRIKE kING) * tHE mANA bAR tHAT wAS zERO aLL aLONG", MID)
center(59, "nOT AFFILIATED WITH JAGEX. cACHE ASSETS STAY OUT OF GIT. aLWAYS.", DIM)
center(60, "  f r e e   y o u r   m i n d .   t h e n   s p e n d   y o u r   m a n a .  ", HEAD)
center(61, "-=[ mObAsCaPe // 2026 // nODE 10: tHE gRIMOIRE // 57.6k // sYSoP: rIFT // 110 cASTS, 0 mISSES ]=-", AMBER)

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
