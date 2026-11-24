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
def shaded_logo(y, word):
    """Logo shaded left-to-right with the four block densities, the
    classic ░▒▓█ ramp, so each letter fades in from the dark."""
    width = len(word) * 10 - 1
    x0 = (W - width) // 2
    clear(x0 - 3, y - 1, x0 + width + 3, y + LOGO_ROWS)
    ramp = "░▒▓█"
    for i, letter in enumerate(word):
        for r in range(LOGO_ROWS):
            for c, px in enumerate(LOGO[letter][r]):
                if px == " ":
                    continue
                ch = px if px in "▀▄" else ramp[min(3, c * 4 // 9 + 1)]
                put(x0 + i * 10 + c, y + r, ch, LOGO_GRAD[r])


shaded_logo(3, "MOBASCAPE")
fade_bar(1, 2, W - 3)
center(10, "  ░▒▓  t H E   d R A F T  ::  1 0   h E R O E S   //   0   d U P L I C A T E S  ▓▒░  ", AMBER)
center(11, "  " + elite("phase.4 // choose wisely, they are watching") + "  ", WHITE)
fade_bar(12, 2, W - 3)

# --- release info -------------------------------------------------------------
header(14, elite("release info"))
clear(4, 15, W - 5, 18)
info = [
    ("rELEASE dATE", "2026-11-24", "cOMMIT", "96cef36  ->  sam/main"),
    ("tYPE", "hERO sELECT: tEAM cOLUMNS", "tESTS", "400 PASS // 0 FAIL   (+4)"),
    ("pICKS", "9 BOTS // DONE BY 22 s", "lOBBY", "shows the real lineups now"),
    ("rULE", "NEVER STEAL YOUR HIGHLIGHT", "sTILL", "bots don't play their pick yet"),
]
for i, (k1, v1, k2, v2) in enumerate(info):
    y = 15 + i
    text(8, y, f"{k1:.<16}", DIM)
    text(25, y, v1, AMBER if k1 == "rULE" else BRIGHT)
    text(76, y, f"{k2:.<12}", DIM)
    text(89, y, v2, RED if k2 == "sTILL" else HEAD)

# --- left: the draft timeline --------------------------------------------------
box(4, 20, 74, 44, elite("draft.log // 30 seconds"))
text(8, 22, "  t(s)  0    5    10   15   20   25   30", DIM)
text(8, 23, "        ├────┼────┼────┼────┼────┼────┤", DIM)
picks = [
    ("RED  1", 2.5, "Kalphite Queen", RED),
    ("BLUE 2", 4.5, "Venenatis", CYAN),
    ("RED  2", 7.0, "Vorkath", RED),
    ("BLUE 3", 9.5, "Blowpipe Ranger", CYAN),
    ("RED  3", 12.0, "Ancient Wyvern", RED),
    ("BLUE 4", 14.5, "Big Frog", CYAN),
    ("RED  4", 17.0, "Kree'arra Jr.", RED),
    ("BLUE 5", 19.5, "JalRek-Jad", CYAN),
    ("RED  5", 22.0, "General Graardor", RED),
]
for i, (who, t, hero, col) in enumerate(picks):
    y = 24 + i * 2
    text(8, y, who, col)
    x = 16 + int(t)
    for k in range(16, x):
        put(k, y, "·", FAINT)
    put(x, y, "◆", col)
    text(x + 2, y, hero, WHITE)
text(8, 42, "YOU    ", AMBER)
text(15, 42, "─" * 31 + "► Kolodion (locked)", AMBER)

# --- right: the stage ---------------------------------------------------------
box(76, 20, W - 5, 44, elite("stage.scr // the spotlight"))
stage = [
    " YOUR TEAM              ▲ 17 ▲              ENEMY ",
    " ┌──────┐            ░░░░░░░░░░░            ┌──────┐",
    " │ YOU  │           ░░░▒▒▒▒▒▒▒░░░           │ BOT1 │",
    " │ ~*~  │          ░░▒▒▓▓▓▓▓▓▓▒▒░░          │ KQ   │",
    " ├──────┤          ░▒▒▓▓  o  ▓▓▒▒░          ├──────┤",
    " │ BOT1 │         ░▒▓▓▓  /|\\  ▓▓▓▒░         │ BOT2 │",
    " │ VEN  │         ░▒▓▓▓  / \\  ▓▓▓▒░         │ VRK  │",
    " ├──────┤        ░▒▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▒░        ├──────┤",
    " │ BOT2 │       ░▒▒▓▓█ KOLODION █▓▓▒▒░      │ BOT3 │",
    " │ ...  │        ▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀        │ ...  │",
    " └──────┘          [  LOCK IN  ]            └──────┘",
]
for i, line in enumerate(stage):
    for j, ch in enumerate(line):
        if ch == " ":
            continue
        if ch in "░▒▓█▀":
            col = (230, 200, 120) if ch in "▓█" else (150, 130, 80) if ch == "▒" else (90, 80, 50)
        elif ch in "┌┐└┘─│├┤":
            col = DIM
        elif j < 10:
            col = CYAN
        elif j > 40:
            col = RED
        else:
            col = WHITE
        put(79 + j, 22 + i, ch, col)
roster = [
    ("[Big Frog]", MID), ("[Callisto]", MID), ("[PICKED]", RED), ("[PICKED]", RED),
    ("[Venenatis]", MID), ("[PICKED]", RED), ("[Kolodion]", AMBER), ("[Wise Old M]", MID),
]
x = 80
for i, (name, col) in enumerate(roster):
    text(80 + (i % 4) * 15, 35 + i // 4, name, col)
text(80, 38, "PICKED  = dimmed, stamped, unclickable", MID)
text(80, 39, "arrows  = step right over it", MID)
text(80, 40, "bots    = take the type their team lacks", MID)
text(80, 41, "lock in = the rest pick instantly", MID)
text(80, 42, "result  = melee + ranged + magic, both sides", (120, 255, 140))

# --- design call --------------------------------------------------------------
header(46, elite("sysop's note // left on your desk"))
clear(4, 47, W - 5, 49)
center(47, "mULTI-pART lATE rECIPES nEED a dESIGN cALL: nO oSRS cRAFT tAKES tWO oF oUR iTEMS.  wE'LL wAIT fOR tHE oPERATOR.", HEAD)
center(48, "hero_select::PICK_SCHEDULE   //   HeroSelect { blue, red, seed }   //   is_picked()   //   lineups()   //   fill_picks()", DIM)

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
center(58, "gREETZ: eVERY 'mID oR fEED' * tHE 30-sECOND tIMER * tHE bOT wHO lOCKED iN kALPHITE qUEEN aT 2.5 s", MID)
center(59, "nOT AFFILIATED WITH JAGEX. cACHE ASSETS STAY OUT OF GIT. aLWAYS.", DIM)
center(60, "  w h a t   i f   i   t o l d   y o u   t h e   e n e m y   d r a f t   i s   a   h a s h   o f   t h e   c l o c k ?  ", HEAD)
center(61, "-=[ mObAsCaPe // 2026 // nODE 7: tHE dRAFT // 57.6k // sYSoP: rIFT // 10 hEROES, 0 dUPES ]=-", AMBER)

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
