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
ICE = (170, 230, 255)


def jump_logo(y, word, streak_x0):
    """Motion-blur logo mid-dash: solid letters whose strokes smear to ▓
    on their trailing edge, ▒ and ░ afterimages three and six cells
    behind, and speed streaks running back from every row in ░▒═━."""
    width = len(word) * 10 - 1
    x0 = (W - width) // 2 + 4
    clear(streak_x0, y - 1, x0 + width + 2, y + LOGO_ROWS)
    cells = set()
    for i, letter in enumerate(word):
        for r in range(LOGO_ROWS):
            for c, px in enumerate(LOGO[letter][r]):
                if px != " ":
                    cells.add((x0 + i * 10 + c, y + r))
    # Speed streaks: from each row's leftmost cell back to streak_x0.
    for r in range(LOGO_ROWS):
        row = [x for (x, yy) in cells if yy == y + r]
        if not row:
            continue
        left = min(row)
        for x in range(streak_x0, left - 1):
            f = (x - streak_x0) / max(1, left - 1 - streak_x0)
            if (x + r * 3) % 7 == 0 and f < 0.5:
                continue
            ch = "░" if f < 0.3 else "▒" if f < 0.55 else "═" if f < 0.85 else "━"
            col = (0, int(50 + 150 * f), int(15 + 50 * f))
            put(x, y + r, ch, col)
    # Afterimages three and six cells behind, only on empty ground.
    for (back, ch, col) in ((6, "░", (0, 60, 18)), (3, "▒", (0, 105, 30))):
        for (x, yy) in cells:
            tx = x - back
            inside = x0 <= tx < x0 + width and (tx - x0) % 10 != 9
            if (tx, yy) not in cells and not inside:
                put(tx, yy, ch, col)
    # Face: solid, with each stroke's trailing (left) edge smeared to ▓.
    for (x, yy) in cells:
        px = LOGO[word[(x - x0) // 10]][yy - y][(x - x0) % 10]
        trailing = (x - 1, yy) not in cells and px not in "▀▄"
        put(x, yy, "▓" if trailing else (px if px in "▀▄" else "█"), LOGO_GRAD[yy - y])


jump_logo(3, "MOBASCAPE", 3)
# A comet over the logo's right shoulder.
for i, (ch, col) in enumerate([("✈", WHITE), ("═", ICE), ("─", (90, 160, 200)), ("·", (50, 90, 120))]):
    text(W - 12 + i * 2, 2, ch, col)
fade_bar(1, 2, W - 3)
center(10, "  ░▒▓  n O D E   1 1 :   t H E   j U M P   p R O G R A M   //   b O T S   d A S H   ,   b O T S   f L E E  ▓▒░  ", AMBER)
center(11, "  " + elite("everybody falls the first time. callisto didn't. two kills.") + "  ", WHITE)
fade_bar(12, 2, W - 3)

# --- release info -------------------------------------------------------------
header(14, elite("release info"))
clear(4, 15, W - 5, 18)
info = [
    ("rELEASE dATE", "2026-11-24", "cOMMIT", "4cbdf07  ->  sam/main"),
    ("tYPE", "bOT dASHES + lEAPS", "tESTS", "411 pass // clippy clean"),
    ("wAS", "CHARGES FIRED AS BOLTS", "nOW", "hurl, land, splash 170"),
    ("wAS", "LEAPS CAST AS HEALS (!)", "nOW", "chase, or jump for home"),
]
for i, (k1, v1, k2, v2) in enumerate(info):
    yy = 15 + i
    text(8, yy, f"{k1:.<16}", DIM)
    text(25, yy, v1, RED if k1 == "wAS" else BRIGHT)
    text(76, yy, f"{k2:.<12}", DIM)
    text(89, yy, v2, HEAD)

# --- left: the trajectory ------------------------------------------------------
box(4, 20, 74, 45, elite("jump.prg // trajectory"))
traj = [
    "                                                                 ",
    "   DASH  (damage)                 ✸ landing: splash 170          ",
    "                   ╭──────────╮   ▼                              ",
    "            ╭──────╯  1700/s  ╰────►  m  X  m                    ",
    "   @ ░▒▓════╯                       ░▒▓█▓▒░                      ",
    "   ▲ bot                         stops 110 short                 ",
    "                                                                 ",
    "   LEAP  (no damage)                                             ",
    "                                        ▓▓                       ",
    "   home ◄═════════════════════════╗     ▓▓ WALL                  ",
    "    ▲         ╭───────────────╮   ║     ▓▓                       ",
    "    │    ◄────╯   escape      ╰───@ ──► ▓▓ ✗ clamped at the      ",
    "    │      hp < 30%, home > 1500  ▲     ▓▓   first unwalkable    ",
    "    └─ fountain heal        retreating  ▓▓   tile (32u steps)    ",
    "                                                                 ",
    "   matched by verb in the NAME or first 3 words:                 ",
    "   charge leap dash jump lunge pounce swing burrow               ",
    "   rush dive teleport blink hop                                  ",
    "                                                                 ",
    "   'kalphite workers burst out and charge' -> stays a bolt       ",
    "    (the verb is the workers', not the queen's)                  ",
]
for i, line in enumerate(traj):
    for j, ch in enumerate(line):
        if ch == " ":
            continue
        if ch in "╭╮╯╰─═╗║►◄":
            col = ICE
        elif ch in "░▒▓█":
            col = (110, 80, 60) if "WALL" in line or (i in (8, 10, 11, 12, 13) and 38 <= j <= 42) else (120, 200, 255)
        elif ch in "✸✗X":
            col = RED
        elif ch == "@":
            col = WHITE
        elif ch == "m":
            col = (255, 140, 120)
        elif line.strip().startswith(("DASH", "LEAP")) and j < 10:
            col = GOLD
        elif i >= 15:
            col = MID if i < 19 else DIM
        else:
            col = HEAD
        put(6 + j, 22 + i, ch, col)

# --- right: who moves ---------------------------------------------------------
box(76, 20, W - 5, 45, elite("movers.dat // the roster in flight"))
movers = [
    ("DASH", "Callisto", "W", "CHARGE", "800"),
    ("DASH", "General Graardor", "R", "BANDOS' WRATH", "900"),
    ("DASH", "K'ril Tsutsaroth", "W", "DEMONIC CHARGE", "650"),
    ("DASH", "Venenatis", "W", "POUNCE", "700"),
    ("DASH", "Commander Zilyana", "W", "BLESSED DASH", "480"),
    ("LEAP", "JalRek-Jad", "W", "TELEPORT", "640"),
    ("LEAP", "Kree'arra Jr.", "W", "KREE'S LEAP", "896"),
    ("LEAP", "King Awowogei", "W", "VINE SWING", "900"),
    ("LEAP", "Blowpipe Ranger", "E", "RANGER'S FOOTWORK", "512"),
]
for x, label in ((79, "TYPE"), (87, "HERO"), (106, "KEY"), (111, "ABILITY"), (130, "RANGE")):
    text(x, 22, label, DIM)
for x, n in ((79, 6), (87, 17), (106, 3), (111, 17), (130, 14)):
    text(x, 23, "─" * n, FAINT)
for i, (kind, who, key, name, rng) in enumerate(movers):
    yy = 24 + i * 2
    col = (255, 150, 90) if kind == "DASH" else ICE
    text(79, yy, ("► " if kind == "DASH" else "↷ ") + kind, col)
    text(87, yy, f"{who:<17}", CYAN if i % 2 == 0 else (90, 210, 180))
    text(106, yy, f"[{key}]", WHITE)
    text(111, yy, f"{name:<17}", HEAD)
    bar = int(int(rng) / 900 * 8)
    text(130, yy, "▮" * bar, col)
    text(139, yy, rng, DIM)
text(79, 42, "6:00 SIM  bolt 47  strike 43  nova 1  dash 1  escape 19", AMBER)
text(79, 43, "          escapes: jad 10, kree 9 // callisto's charge: 2 kills", DIM)

# --- sysop's note ------------------------------------------------------------
header(47, elite("sysop's note // free your legs"))
clear(4, 48, W - 5, 49)
center(48, "sTILL aN aPPROXIMATION: sUMMONS aND zONES rEAD aS bOLTS oR sTRIKES.  tHE rEAL kITS nEED a cASTER-gENERIC rUNTIME.", HEAD)
center(49, "animated::bot_cast  //  SpellShape::{Care,Nova,Strike,Bolt,Dash,Leap}  //  bot_hurl  //  bot_try_escape", DIM)

# --- roadmap -------------------------------------------------------------------
header(51, elite("roadmap // the oracle says"))
clear(4, 52, W - 5, 56)
phases = [
    ("P1 LIGHT", 9), ("P2 MINIMAP", 5), ("P3 SHOP", 8), ("P4 SELECT", 5),
    ("P5 ARENA", 2), ("P6 HUD", 0), ("P7 POST", 0),
]
for i, (name, done) in enumerate(phases):
    x = 8 + (i % 4) * 35
    yy = 52 + (i // 4) * 2
    text(x, yy, f"{name:<11}", CYAN)
    text(x + 11, yy, "[", DIM)
    for j in range(10):
        put(x + 12 + j, yy, "▓" if j < done else "░", BRIGHT if j < done else FAINT)
    text(x + 22, yy, f"] {done * 10:>3}%", HEAD if done else FAINT)
text(8, 56, ">> next: caster-generic kit runtime (bots cast real kits), summons/zones, lane picker", MID)

fade_bar(57, 2, W - 3)
clear(10, 58, W - 11, 61)
center(58, "gREETZ: cALLISTO (oNE cHARGE, tWO kILLS) * jALREK-jAD (tEN tELEPORTS hOME) * tHE wALL tHAT sAID nO", MID)
center(59, "nOT AFFILIATED WITH JAGEX. cACHE ASSETS STAY OUT OF GIT. aLWAYS.", DIM)
center(60, "  y o u   h a v e   t o   l e t   i t   a l l   g o .   f e a r ,   d o u b t ,   c o o l d o w n s .  ", HEAD)
center(61, "-=[ mObAsCaPe // 2026 // nODE 11: tHE jUMP pROGRAM // 57.6k // sYSoP: rIFT // 1 cHARGE, 19 jUMPS ]=-", AMBER)

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
