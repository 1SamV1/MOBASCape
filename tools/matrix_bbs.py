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
def outline_logo(y, word):
    """Hollow logo: only the edge of each letter is drawn, in box-drawing
    strokes, with a dotted ░ fill inside, like the ACiD-era 'neon' fonts."""
    width = len(word) * 10 - 1
    x0 = (W - width) // 2
    clear(x0 - 3, y - 1, x0 + width + 3, y + LOGO_ROWS + 1)
    cells = set()
    for i, letter in enumerate(word):
        for r in range(LOGO_ROWS):
            for c, px in enumerate(LOGO[letter][r]):
                if px != " ":
                    cells.add((x0 + i * 10 + c, y + r))
    for (x, yy) in cells:
        edge = any((x + dx, yy + dy) not in cells for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)))
        row = yy - y
        if edge:
            up = (x, yy - 1) not in cells
            down = (x, yy + 1) not in cells
            ch = "▀" if up and not down else "▄" if down and not up else "█"
            put(x, yy, ch, LOGO_GRAD[row])
        else:
            put(x, yy, "░", FAINT)
    # A scan line through the middle.
    for x in range(x0 - 2, x0 + width + 2):
        if (x, y + 3) not in cells:
            put(x, y + 3, "─", FAINT)


fade_bar(1, 2, W - 3)
outline_logo(3, "MOBASCAPE")
center(10, "  ░▒▓  b U I L D S   f R O M   //   b U I L D S   i N T O  ▓▒░  ", AMBER)
center(11, "  " + elite("phase.3 // the upgrade ladder") + "  ", WHITE)
fade_bar(12, 2, W - 3)

# --- release info -------------------------------------------------------------
header(14, elite("release info"))
clear(4, 15, W - 5, 18)
info = [
    ("rELEASE dATE", "2026-11-24", "cOMMIT", "1a78702  ->  sam/main"),
    ("tYPE", "sHOP rECIPES + dATA", "tESTS", "392 PASS // 0 FAIL   (+3)"),
    ("rULE", "PAY THE DIFFERENCE", "cLIPPY", "-D warnings :: CLEAN"),
    ("lADDERS", "7 FAMILIES // 19 RECIPES", "gUARD", "validate_recipes() at load"),
]
for i, (k1, v1, k2, v2) in enumerate(info):
    y = 15 + i
    text(8, y, f"{k1:.<16}", DIM)
    text(25, y, v1, AMBER if k1 == "rULE" else BRIGHT)
    text(76, y, f"{k2:.<12}", DIM)
    text(89, y, v2, HEAD)

# --- left: the sword ladder ----------------------------------------------------
box(4, 20, 70, 44, elite("ladder.exe // forge run"))
rungs = [
    ("RUNE", 2400, 700),
    ("ADAMANT", 1700, 500),
    ("MITHRIL", 1200, 350),
    ("STEEL", 850, 300),
    ("IRON", 550, 250),
    ("BRONZE", 300, 300),
]
for i, (name, cost, step) in enumerate(rungs):
    y = 22 + i * 3
    text(8, y, "╔═══════════════╗", DIM)
    text(8, y + 1, "║", DIM)
    text(10, y + 1, f"{name:<13}", BRIGHT if i else AMBER)
    text(24, y + 1, "║", DIM)
    text(8, y + 2, "╚═══════╤═══════╝" if i < 5 else "╚═══════════════╝", DIM)
    # The blade.
    blade = "o=[]" + "=" * (6 + (5 - i) * 0) + ">" * 1
    text(27, y + 1, "o==[]" + "=" * (12 - i * 0) + "-" * (5 - i) + ">", MID if i else BRIGHT)
    text(52, y + 1, f"{cost:>5}g", HEAD)
    if i < 5:
        text(60, y + 1, f"+{step}g", (120, 255, 140))
        text(16, y + 3, "│", FAINT) if False else None
    else:
        text(60, y + 1, "buy", DIM)
for i in range(5):
    put(16, 24 + i * 3, "▲", BRIGHT)
text(8, 41, "climb: 300 + 250 + 300 + 350 + 500 + 700 = 2400", MID)
text(8, 42, "the parts melt down. the slot comes back.", DIM)

# --- right: the recipe tree + proof ------------------------------------------
box(72, 20, W - 5, 44, elite("recipes.dat // tree"))
tree = [
    ("shortbow ─► oak ─► willow ─► maple ─► yew ─► magic", CYAN),
    ("bronze plate ─► iron ─► steel ─► mithril ─► adamant ─► rune", CYAN),
    ("staff of fire ─────────────► mystic fire staff", CYAN),
    ("bandos boots ──────────────► guardian boots", CYAN),
    ("ranger boots ──────────────► pegasian boots", CYAN),
    ("dragon boots ──────────────► primordial boots", CYAN),
]
for i, (line, col) in enumerate(tree):
    y = 22 + i * 2
    put(75, y, "■", AMBER)
    for j, ch in enumerate(line):
        put(77 + j, y, ch, BRIGHT if ch in "─►" else col)
text(75, 34, "┌─[ demo-shop.log ]───────────────────────────────────┐", DIM)
log = [
    ("upgraded Iron sword   250g of 550g", (120, 255, 140)),
    ("upgraded Steel sword  300g of 850g", (120, 255, 140)),
    ("sold Staff of air     210g (70%)", CYAN),
    ("undid Steel sword    +300g, Iron sword returns", AMBER),
]
for i, (line, col) in enumerate(log):
    text(75, 35 + i, "│", DIM)
    text(77, 35 + i, "> " + line, col)
    text(129, 35 + i, "│", DIM)
text(75, 39, "└─────────────────────────────────────────────────────┘", DIM)
text(75, 41, "grid: GREEN price = upgrade   GOLD = buy   GREY = broke", MID)
text(75, 42, "a full bag can still upgrade: the part frees its slot", DIM)

# --- the code -----------------------------------------------------------------
header(46, elite("the law of the forge"))
clear(4, 47, W - 5, 49)
center(47, "I. oWN tHE pARTS   II. pAY tHE dIFFERENCE   III. tHE pARTS aRE cONSUMED   IV. uNDO rETURNS tHEM   V. nO pROFIT lOOPS", HEAD)
center(48, '"builds_from": ["iron_sword"]   //   ItemShop::upgrade() -> Upgrade { consumed, price }   //   part.cost < item.cost', DIM)

# --- roadmap -------------------------------------------------------------------
header(51, elite("roadmap // the oracle says"))
clear(4, 52, W - 5, 56)
phases = [
    ("P1 LIGHT", 9), ("P2 MINIMAP", 5), ("P3 SHOP", 6), ("P4 SELECT", 1),
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
text(8, 55, ">> next in P3: search box, recommended page, multi-part late recipes", MID)

fade_bar(57, 2, W - 3)
clear(10, 58, W - 11, 61)
center(58, "gREETZ: dORIC's aNVIL * tHE dWARVEN mINE * eVERY nOOB wHO sMITHED 300 bRONZE dAGGERS tO gET hERE", MID)
center(59, "nOT AFFILIATED WITH JAGEX. cACHE ASSETS STAY OUT OF GIT. aLWAYS.", DIM)
center(60, "  t h e r e   i s   n o   s p o o n .   o n l y   a   b r o n z e   s w o r d   y o u   w i l l   s o o n   o u t g r o w .  ", HEAD)
center(61, "-=[ mObAsCaPe // 2026 // nODE 4: tHE fORGE // 57.6k // sYSoP: rIFT // sTEEL - iRON = 300g ]=-", AMBER)

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
