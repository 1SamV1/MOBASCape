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
def italic_logo(y, word):
    """Slanted block logo: each row shifts left as it goes down, with a ▒
    shadow trailing right, like the 1990s ANSi group headers."""
    width = len(word) * 10 - 1 + LOGO_ROWS
    x0 = (W - width) // 2
    clear(x0 - 3, y - 1, x0 + width + 4, y + LOGO_ROWS + 1)
    for i, letter in enumerate(word):
        for r in range(LOGO_ROWS):
            shift = LOGO_ROWS - r
            for c, px in enumerate(LOGO[letter][r]):
                if px == " ":
                    continue
                x = x0 + shift + i * 10 + c
                if grid[y + r + 1][x + 1][0] == " ":
                    put(x + 1, y + r + 1, "▒", FAINT)
                put(x, y + r, px, LOGO_GRAD[r])
    # Speed lines off the left edge.
    for r, length in [(1, 10), (3, 16), (5, 8)]:
        for k in range(length):
            put(x0 - 2 - k + (LOGO_ROWS - r), y + r, "─" if k % 3 else "═", DIM if k > length // 2 else MID)


fade_bar(1, 2, W - 3)
italic_logo(3, "MOBASCAPE")
center(11, "  ░▒▓  t H E   e X C H A N G E   r A T E  ::  7 0  /  1 0 0  ▓▒░  ", AMBER)
center(12, "  " + elite("phase.3 // sell, undo, repeat") + "  ", WHITE)
fade_bar(13, 2, W - 3)

# --- release info -------------------------------------------------------------
header(15, elite("release info"))
clear(4, 16, W - 5, 19)
info = [
    ("rELEASE dATE", "2026-11-24", "cOMMIT", "f06c2d6  ->  sam/main"),
    ("tYPE", "sHOP eCONOMY + uI", "tESTS", "389 PASS // 0 FAIL   (+2)"),
    ("rATES", "SELL 70%  //  UNDO 100%", "cLIPPY", "-D warnings :: CLEAN"),
    ("aPPROVED bY", "THE OPERATOR", "pROOF", "--demo-shop, one frame, exact gold"),
]
for i, (k1, v1, k2, v2) in enumerate(info):
    y = 16 + i
    text(8, y, f"{k1:.<16}", DIM)
    text(25, y, v1, AMBER if k1 == "rATES" else BRIGHT)
    text(76, y, f"{k2:.<12}", DIM)
    text(89, y, v2, HEAD)

# --- left: the receipt ---------------------------------------------------------
box(4, 21, 62, 43, elite("ledger.txt // demo run"))
text(9, 23, "GRAND EXCHANGE  *  RECEIPT #0x2400", HEAD)
text(9, 24, "-" * 46, DIM)
ledger = [
    ("GRANT  dev gold", "+8000", BRIGHT),
    ("BUY    Bronze sword", "-300", RED),
    ("BUY    Iron sword", "-550", RED),
    ("BUY    Steel sword", "-850", RED),
    ("BUY    Mithril sword", "-1200", RED),
    ("BUY    Adamant sword", "-1700", RED),
    ("BUY    Rune sword", "-2400", RED),
    ("SELL   Rune sword       70%", "+1680", CYAN),
    ("UNDO   Adamant sword   100%", "+1700", AMBER),
]
for i, (what, amount, col) in enumerate(ledger):
    y = 25 + i
    text(9, y, what, WHITE if what.startswith(("SELL", "UNDO")) else MID)
    dots = 46 - len(what) - len(amount) - 1
    text(9 + len(what), y, " " + "." * (dots - 1), FAINT)
    text(55 - len(amount), y, amount, col)
text(9, 34, "-" * 46, DIM)
text(9, 35, "BAG ........................ 4 / 6", MID)
text(9, 36, "UNDO STACK ................. 4 left", MID)
text(9, 37, "FINAL GOLD (+ passive 8/s)", MID)
text(51, 37, "4415", AMBER)
text(9, 39, "  ||| || ||| | |||| || | ||| || |||| |", DIM)
text(9, 40, "  thank you for trading. no refunds", DIM)
text(9, 41, "  after the window closes.", DIM)

# --- right: coins + the rewind tape -------------------------------------------
box(64, 21, W - 5, 43, elite("vault.sys // the stack"))
coins = [
    "           _.-------._",
    "        .-'  _____    '-.",
    "       /   .'  70% '.    \\",
    "      |   /  SELL   \\    |",
    "      |   \\  BACK   /    |",
    "       \\   '._____.'    /",
    "        '-._       _.-'",
    "            '-----'",
]
for i, line in enumerate(coins):
    text(66, 23 + i, line, AMBER if i not in (2, 3, 4) else (230, 200, 90))
stack = ["    (@@@)", "   (@@@@@)", "  (@@@@@@@)", " (@@@@@@@@@)"]
for i, line in enumerate(stack):
    text(68, 32 + i, line, AMBER)
text(68, 37, " 1680 + 1700", WHITE)
# The cassette.
cx, cy = 98, 24
tape = [
    "┌──────────────────────────────────────┐",
    "│  ◄◄  UNDO.TAPE   side A: this visit  │",
    "│   ╭────╮                  ╭────╮     │",
    "│   │ ◉◉ │════════════════> │ ◉◉ │     │",
    "│   ╰────╯   newest first   ╰────╯     │",
    "│  bag item   -> stats off, 100% back  │",
    "│  chest item -> leaves delivery list  │",
    "│  sold item  -> struck from the tape  │",
    "│  close shop -> tape erased           │",
    "└──────────────────────────────────────┘",
]
for i, line in enumerate(tape):
    for j, ch in enumerate(line):
        col = DIM if ch in "┌┐└┘─│╭╮╰╯" else (AMBER if ch in "◄◉" else (BRIGHT if ch in "═>" else MID))
        put(cx - 4 + j, cy + i, ch, col)
text(94, 35, "guard handed it over mid-visit? the tape", DIM)
text(94, 36, "re-labels it: chest -> bag. still undoable.", DIM)
text(94, 38, "the old HUD's sell button: 50% -> 70% too.", MID)
text(94, 40, "SELL greys out unless the item is in the bag", MID)
text(94, 41, "UNDO greys out when the tape is empty", MID)

# --- the code -----------------------------------------------------------------
header(45, elite("the code of the exchange"))
clear(4, 46, W - 5, 48)
center(46, "I. sELL fOR sEVENTY   II. uNDO fOR aLL   III. tHE cHEST cOUNTS   IV. nO dOUBLE dIPPING   V. cLOSE iS fINAL", HEAD)
center(47, "items::SELL_REFUND_PERCENT = 70   //   ItemShop::undo: Vec<Purchase>   //   AnimatedScene::cancel_stored()", DIM)

# --- roadmap -------------------------------------------------------------------
header(50, elite("roadmap // the oracle says"))
clear(4, 51, W - 5, 54)
phases = [
    ("P1 LIGHT", 9), ("P2 MINIMAP", 5), ("P3 SHOP", 4), ("P4 SELECT", 1),
    ("P5 ARENA", 0), ("P6 HUD", 0), ("P7 POST", 0),
]
for i, (name, done) in enumerate(phases):
    x = 8 + (i % 4) * 35
    y = 51 + (i // 4) * 2
    text(x, y, f"{name:<11}", CYAN)
    text(x + 11, y, "[", DIM)
    for j in range(10):
        put(x + 12 + j, y, "▓" if j < done else "░", BRIGHT if j < done else FAINT)
    text(x + 22, y, f"] {done * 10:>3}%", HEAD if done else FAINT)
text(8, 54, ">> next in P3: recipes A+B>C, builds-into data, search box", MID)

fade_bar(56, 2, W - 3)
clear(10, 57, W - 11, 60)
center(57, "gREETZ: tHE OSRS CACHE * sERDE * wgpu * eVERY bOT tHAT eVER bOUGHT a rUNE sWORD aND rEGRETTED iT", MID)
center(58, "nOT AFFILIATED WITH JAGEX. cACHE ASSETS STAY OUT OF GIT. aLWAYS.", DIM)
center(59, "  y o u   h a v e   t o   l e t   i t   a l l   g o ,   n e o .   f o r   s e v e n t y   p e r c e n t .  ", HEAD)
center(60, "-=[ mObAsCaPe // 2026 // nODE 3: tHE vAULT // 57.6k // sYSoP: rIFT // r/s: 70 / 100 ]=-", AMBER)

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
