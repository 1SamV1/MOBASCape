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
def extruded_logo(y, word, depth=3):
    """ACiD/iCE style extrusion: dim ░▒▓ layers stepping back down-right,
    the lit face on top with a vertical gradient."""
    width = len(word) * 10 - 1
    x0 = (W - width) // 2 - depth // 2
    clear(x0 - 3, y - 1, x0 + width + depth + 3, y + LOGO_ROWS + depth)
    layers = [("░", FAINT), ("▒", DIM), ("▓", MID)]
    for d in range(depth, 0, -1):
        ch, col = layers[min(depth - d, 2)]
        for i, letter in enumerate(word):
            for r in range(LOGO_ROWS):
                for c, px in enumerate(LOGO[letter][r]):
                    if px != " ":
                        put(x0 + i * 10 + c + d, y + r + d, ch, col)
    for i, letter in enumerate(word):
        for r in range(LOGO_ROWS):
            for c, px in enumerate(LOGO[letter][r]):
                if px != " ":
                    put(x0 + i * 10 + c, y + r, px, LOGO_GRAD[r])
    return x0, width


fade_bar(1, 2, W - 3)
extruded_logo(3, "MOBASCAPE")
center(12, "  ·:·  t H E   g R A N D   e X C H A N G E   o P E N s  ·:·  ", AMBER)
center(13, "  " + elite("phase.3 initiated :: follow the gold") + "  ", WHITE)
fade_bar(14, 2, W - 3)

# --- release info -------------------------------------------------------------
header(16, elite("release info"))
clear(4, 17, W - 5, 20)
info = [
    ("rELEASE dATE", "2026-11-23", "cOMMIT", "cb46ddb  ->  sam/main"),
    ("tYPE", "sHOP uI / mINIMAP", "tESTS", "387 PASS // 0 FAIL   (+4)"),
    ("gAMEPLAY", "UNCHANGED :: layout only", "cLIPPY", "-D warnings :: CLEAN"),
    ("rULE", "ONLY OSRS CACHE ASSETS", "iCONS", "cache item models, baked"),
]
for i, (k1, v1, k2, v2) in enumerate(info):
    y = 17 + i
    text(8, y, f"{k1:.<16}", DIM)
    text(25, y, v1, AMBER if k1 in ("rULE", "gAMEPLAY") else BRIGHT)
    text(76, y, f"{k2:.<12}", DIM)
    text(89, y, v2, HEAD)

# --- left: the shop grid, drawn as an NFO ------------------------------------
box(4, 22, 86, 42, elite("ge.exe // tiered grid"))
ICONS = {"sword": " /", "bow": " D", "staff": " |*", "ring": " o", "boots": " L", "body": "[#]", "amulet": " Y", "shield": "(+)"}


def item_cell(x, y, icon, price, selected=False, affordable=True):
    frame = HEAD if selected else DIM
    text(x, y, "┌───┐", frame)
    text(x, y + 1, "│", frame)
    text(x + 1, y + 1, f"{ICONS[icon]:^3}"[:3], CYAN if affordable else FAINT)
    text(x + 4, y + 1, "│", frame)
    text(x, y + 2, "└───┘", frame)
    text(x + (5 - len(str(price))) // 2, y + 3, str(price), AMBER if affordable else DIM)


sections = [
    ("STARTER", ["body", "sword", "bow", "staff", "amulet", "body"], [200, 300, 300, 300, 300, 350]),
    ("EARLY", ["sword", "body", "ring", "staff", "bow", "sword"], [550, 550, 600, 650, 800, 850]),
    ("CORE", ["staff", "shield", "body", "bow", "boots", "sword"], [1450, 1600, 1800, 2100, 2400, 2400]),
]
gold = 1116
y = 24
for label, icons, prices in sections:
    text(7, y, label, HEAD)
    for x in range(8 + len(label), 58):
        put(x, y, "─", DIM)
    y += 1
    for k, (icon, price) in enumerate(zip(icons, prices)):
        item_cell(8 + k * 8, y, icon, price, selected=(label == "STARTER" and k == 1), affordable=price <= gold)
    y += 5
# Detail pane beside the grid.
dx = 60
text(dx, 24, "Bronze sword", AMBER)
text(dx, 25, "300 gold", DIM)
text(dx, 27, "+10 damage", BRIGHT)
text(dx, 29, "tier ..... STARTER", DIM)
text(dx, 30, "by cost .. <= 500", DIM)
text(dx, 33, "▐ BUY ▌", WHITE)
text(dx, 36, "gold:", DIM)
text(dx + 6, 36, f"{gold}", AMBER)
text(dx, 37, "white = can't", DIM)
text(dx, 38, "afford yet", DIM)
text(7, 40, "> click a cell, then buy. wheel stops at the end of the grid.", MID)

# --- right: the portrait ring --------------------------------------------------
box(88, 22, W - 5, 42, elite("radar.sys // you are here"))
cx, cy = 104, 31
for row in range(-6, 7):
    for col in range(-13, 14):
        d = (col / 2.0) ** 2 + row ** 2
        if 30 <= d <= 42:
            put(cx + col, cy + row, "█", AMBER)
        elif 24 <= d < 30:
            put(cx + col, cy + row, "▓", (160, 130, 40))
        elif d < 24:
            put(cx + col, cy + row, "░", FAINT)
face = ["  /^^^\\  ", " / ,-. \\ ", " | o o | ", " |  ^  | ", "/|_____|\\", "|=GREEN=|"]
for i, line in enumerate(face):
    text(cx - 4, cy - 3 + i, line, (200, 170, 60) if i < 4 else MID)
text(119, 25, "portrait.map", CYAN)
text(119, 26, "16px disc, cut from", DIM)
text(119, 27, "the baked image: head", DIM)
text(119, 28, "+ shoulders, corners", DIM)
text(119, 29, "alpha'd clear.", DIM)
text(119, 31, "ring r=10, gold =", DIM)
text(119, 32, "the player.", DIM)
text(119, 34, "clamped 11px in:", HEAD)
text(119, 35, "the fountain no", DIM)
text(119, 36, "longer halves you.", DIM)
text(91, 39, "bots: plain team rings until they carry a hero id", MID)

# --- terminal -----------------------------------------------------------------
header(44, elite("tty1 // incoming"))
clear(4, 45, W - 5, 48)
text(8, 45, "> wake up, neo...", BRIGHT)
text(8, 46, "> the shop has you.", BRIGHT)
text(8, 47, "> follow the gold coins: 200 . 300 . 550 . 1200 . 3000 ...", MID)
text(78, 45, "knock, knock.", DIM)
text(78, 46, "SELL / UNDO / RECIPES :: awaiting operator", AMBER)
text(78, 47, "(the refund rate is your call)", DIM)

# --- roadmap -------------------------------------------------------------------
header(50, elite("roadmap // the oracle says"))
clear(4, 51, W - 5, 54)
phases = [
    ("P1 LIGHT", 9), ("P2 MINIMAP", 5), ("P3 SHOP", 2), ("P4 SELECT", 1),
    ("P5 ARENA", 0), ("P6 HUD", 0), ("P7 POST", 0),
]
for i, (name, done) in enumerate(phases):
    col = i % 4
    row = i // 4
    x = 8 + col * 35
    y = 51 + row * 2
    text(x, y, f"{name:<11}", CYAN)
    text(x + 11, y, "[", DIM)
    for j in range(10):
        put(x + 12 + j, y, "▓" if j < done else "░", BRIGHT if j < done else FAINT)
    text(x + 22, y, f"] {done * 10:>3}%", HEAD if done else FAINT)

fade_bar(56, 2, W - 3)
clear(10, 57, W - 11, 60)
center(57, "gREETZ: tHE OSRS CACHE * sERDE * NAGA * wgpu * aLL WHO SPEND 300 GOLD ON A BRONZE SWORD", MID)
center(58, "nOT AFFILIATED WITH JAGEX. cACHE ASSETS STAY OUT OF GIT. aLWAYS.", DIM)
center(59, "  t h e r e   i s   n o   s p o o n .   t h e r e   i s   a   b r o n z e   s w o r d .  ", HEAD)
center(60, "-=[ mObAsCaPe // 2026 // nODE 3: gRAND eXCHANGE // 33.6k // sYSoP: rIFT // r/s: 70% ?? ]=-", AMBER)

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
