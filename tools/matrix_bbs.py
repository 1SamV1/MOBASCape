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
    """Signal-loss logo: the block font on interlaced scanlines (odd rows
    dimmed to ▓, even rows solid), with two rows torn sideways like a
    VHS tracking error and RGB-split ghosts trailing the tear."""
    width = len(word) * 10 - 1
    x0 = (W - width) // 2
    clear(x0 - 5, y - 1, x0 + width + 5, y + LOGO_ROWS)
    tear = {1: 2, 4: -3}
    for ghosts in (True, False):
        for r in range(LOGO_ROWS):
            shift = tear.get(r, 0)
            if ghosts and not shift:
                continue
            for i, letter in enumerate(word):
                for c, px in enumerate(LOGO[letter][r]):
                    if px == " ":
                        continue
                    x = x0 + i * 10 + c
                    if ghosts:
                        # RGB-split ghost left where the row used to be.
                        put(x, y + r, "░", (150, 40, 50) if shift > 0 else (40, 120, 150))
                    else:
                        ch = px if px in "▀▄" else ("█" if r % 2 == 0 else "▓")
                        put(x + shift, y + r, ch, LOGO_GRAD[r])


glitch_logo(3, "MOBASCAPE")
fade_bar(1, 2, W - 3)
center(10, "  ░▒▓  t H E   b O T N E T  ::  9   h E R O E S   j A C K E D   i N   //   1   n E X U S   o P E N  ▓▒░  ", AMBER)
center(11, "  " + elite("they lane. they jungle. they gank in pairs. they buy boots.") + "  ", WHITE)
fade_bar(12, 2, W - 3)

# --- release info -------------------------------------------------------------
header(14, elite("release info"))
clear(4, 15, W - 5, 19)
info = [
    ("rELEASE dATE", "2026-11-24", "cOMMIT", "52da251  ->  sam/main"),
    ("tYPE", "bOT hEROES + oPEN nEXUS", "tESTS", "407 pass // clippy clean"),
    ("hITSPLATS", "48x46 -> 24x23 (cache 1x)", "bANANA", "40% pure chips towers"),
    ("lANE bUGS", "WAVES DIED OUT AT 2:00", "fIX", "cap counts the living"),
    ("", "WAVES WALKED PAST TOWERS", "fIX", "edge + 400 reach"),
]
for i, (k1, v1, k2, v2) in enumerate(info):
    y = 15 + i
    text(8, y, f"{k1:.<16}", DIM)
    text(25, y, v1, RED if y >= 18 else BRIGHT)
    text(76, y, f"{k2:.<12}", DIM)
    text(89, y, v2, HEAD)

# --- left: the bot brain --------------------------------------------------------
box(4, 21, 74, 44, elite("brain.c // one think per second"))
brain = [
    "            ┌────────────┐   90 s    ┌────────────┐",
    "   spawn ──►│    LANE    │──────────►│   JUNGLE   │",
    "            │ last-hit   │◄──────────│ own side   │",
    "            │ no tower   │  1 camp   │ of river   │",
    "            │ dives solo │           └─────┬──────┘",
    "            └──┬───┬─────┘                 │ lv 5+",
    "     every 80 s│   │ gold >= item+400      ▼",
    "        ┌──────▼┐  │            ┌─────────────────┐",
    "        │ GANK  │  │            │  REGION  BOSS   │",
    "        │ x  2  │  │            │ jungler+laner   │",
    "        │weakest│  │            │ -> TEAM BUFF    │",
    "        └───────┘  ▼            └─────────────────┘",
    "            ┌──────────────┐",
    "  hp < 30% ►│   RETREAT    │► heal 6%/s, buy next",
    "            └──────────────┘  recommended item",
]
for i, line in enumerate(brain):
    for j, ch in enumerate(line):
        if ch == " ":
            continue
        if ch in "┌┐└┘─│┬┴├┤▼►◄":
            col = DIM
        elif ch.isupper():
            col = AMBER if line[j:j + 4] in ("GANK", "RETR") else CYAN
        else:
            col = MID
        put(7 + j, 23 + i, ch, col)
text(8, 38, "bLUE  you(mid)  top  bot  bot  jungle", CYAN)
text(8, 39, "rED   top  mid  bot  bot  jungle", RED)
text(8, 41, "BOSSES  inner_bot_west (blue, 190 hp)  //  top_east (red, 150 hp)", HEAD)
text(8, 42, "ganks never enter a base. camps now fight back at bots.", DIM)

# --- right: the lane exploit ----------------------------------------------------
box(76, 21, W - 5, 44, elite("exploit.txt // the tower nobody saw"))
lane = [
    "   ░░ TOWER ░░          minion aggro 520 from CENTRE",
    "  ░▒▒▓▓██▓▓▒▒░                                       ",
    "  ░▒▓  ◆◆  ▓▒░ ◄── 230 radius                        ",
    "  ░▒▒▓▓██▓▓▒▒░                                       ",
    "   ░░░░░░░░░░                                        ",
    "        │ 5 tiles off the lane + formation spread    ",
    "        ▼                                            ",
    "  ─ ─ ─ m m m ─ ─ ─ ─ ─ ─ ─ ─►  INHIBITOR  ► NEXUS   ",
    "        the wave walks by, the tower shoots it,      ",
    "        nobody answers. ~900 > 520.              ",
    "                                                     ",
    "  PATCH:  distance - hit_radius - 400               ",
    "          900 - 230 - 400 = 270 < 520  ► ENGAGE      ",
]
for i, line in enumerate(lane):
    for j, ch in enumerate(line):
        if ch == " ":
            continue
        if ch in "░▒▓█":
            col = (200, 70, 60) if ch in "▓█" else (130, 50, 40)
        elif ch == "◆":
            col = WHITE
        elif ch == "m":
            col = CYAN
        elif ch.isdigit() or ch in "<>=-":
            col = AMBER
        elif ch.isupper():
            col = HEAD
        else:
            col = MID
        put(79 + j, 23 + i, ch, col)
text(80, 36, "sim, before: red nexus 1500 -> 1230,", RED)
text(80, 37, "             inhibitor dead, towers ~full", RED)
text(80, 38, "7:30 sim, after: a blue tower down, red", (120, 255, 140))
text(80, 39, "             at 204 / 368, inhibs 3/3", (120, 255, 140))
text(80, 41, "the nexus shield is gone (your call).", HEAD)
text(80, 42, "inhibitors still spawn super minions.", DIM)

# --- sysop's note --------------------------------------------------------------
header(46, elite("sysop's note // left on your desk"))
clear(4, 47, W - 5, 49)
center(47, "bOTS fIGHT wITH bASIC aTTACKS oNLY fOR nOW.  aBILITIES fOR bOTS aRE tHE nEXT bIG jOB.", HEAD)
center(48, "animated::bots  //  BotBrain  //  BotMode::{Lane,Jungle,Boss,Gank,Retreat}  //  --bots", DIM)

# --- roadmap -------------------------------------------------------------------
header(51, elite("roadmap // the oracle says"))
clear(4, 52, W - 5, 56)
phases = [
    ("P1 LIGHT", 9), ("P2 MINIMAP", 5), ("P3 SHOP", 8), ("P4 SELECT", 5),
    ("P5 ARENA", 1), ("P6 HUD", 0), ("P7 POST", 0),
]
for i, (name, done) in enumerate(phases):
    x = 8 + (i % 4) * 35
    y = 52 + (i // 4) * 2
    text(x, y, f"{name:<11}", CYAN)
    text(x + 11, y, "[", DIM)
    for j in range(10):
        put(x + 12 + j, y, "▓" if j < done else "░", BRIGHT if j < done else FAINT)
    text(x + 22, y, f"] {done * 10:>3}%", HEAD if done else FAINT)
text(8, 55, ">> next: bot abilities, lane picker (Top/Jungle/Mid/Carry/Support), battle spells", MID)

fade_bar(57, 2, W - 3)
clear(10, 58, W - 11, 61)
center(58, "gREETZ: tHE 37tH mINION (fIRST iN 2 mINUTES) * kREE'ARRA jR. (lV 6, bOSS sLAYER) * eVERY bOT tHAT bOUGHT bOOTS", MID)
center(59, "nOT AFFILIATED WITH JAGEX. cACHE ASSETS STAY OUT OF GIT. aLWAYS.", DIM)
center(60, "  t h e r e   i s   n o   s p o o n .   t h e r e   i s   o n l y   t h e   w a v e .  ", HEAD)
center(61, "-=[ mObAsCaPe // 2026 // nODE 9: tHE bOTNET // 57.6k // sYSoP: rIFT // 9 bOTS, 1 oPEN nEXUS ]=-", AMBER)

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
