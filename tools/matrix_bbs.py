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
# Optional per-cell background colours (.ans and .png; .txt has none).
bg = {}


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
            bg.pop((x, y), None)


def shade(x0, y0, x1, y1, colour):
    """Give cells a background colour."""
    for y in range(y0, y1 + 1):
        for x in range(x0, x1 + 1):
            if 0 <= x < W and 0 <= y < H:
                bg[(x, y)] = colour


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
STONE = (206, 192, 156)
ORANGE = (255, 152, 31)
YELLOW = (255, 255, 0)
TIP = (255, 255, 160)
INK_RED = (190, 40, 30)
PARCH = (150, 128, 96)
CODE_GRAD = [WHITE, WHITE, HEAD, HEAD, BRIGHT, BRIGHT, BRIGHT, BRIGHT, MID, MID, (0, 160, 40), (0, 160, 40)]
CODE_BG = [(0, 84 - 5 * r, 22 - r) for r in range(12)]
DENSE = "#@$%&8BWM0QXZ"
SPARSE = ".:'`,-"


def code_logo(y, word):
    """The word written in the rain itself (Cypher: "I don't even see the
    code"): dense glyphs inside the letters, sparse ones outside, so it
    reads in plain text too. Every column is a falling stream whose head
    whitens what it passes. Each block-font row is two mask rows, ▀ the top
    half and ▄ the bottom."""
    width = len(word) * 10 - 1
    x0 = (W - width) // 2
    rows = LOGO_ROWS * 2
    mask = set()
    for i, letter in enumerate(word):
        for r in range(LOGO_ROWS):
            for c, px in enumerate(LOGO[letter][r]):
                x = x0 + i * 10 + c
                if px in "█▀":
                    mask.add((x, 2 * r))
                if px in "█▄":
                    mask.add((x, 2 * r + 1))
    clear(x0 - 3, y, x0 + width + 2, y + rows - 1)
    lrng = random.Random(15)
    for x in range(x0 - 3, x0 + width + 3):
        head = lrng.randrange(-4, rows + 8)
        length = lrng.randrange(4, 12)
        for r in range(rows):
            k = head - r
            streak = 0 <= k < length
            if (x, r) in mask:
                col = WHITE if k == 0 else HEAD if streak and k < 3 else CODE_GRAD[r]
                put(x, y + r, lrng.choice(DENSE), col)
                shade(x, y + r, x, y + r, (0, 120, 30) if k == 0 else CODE_BG[r])
            elif 0 <= k < 3:
                # Outside the letters only the stream's head shows.
                put(x, y + r, ":" if k == 0 else lrng.choice(SPARSE), MID if k == 0 else DIM)
            elif lrng.random() < 0.03:
                put(x, y + r, ".", FAINT)
    return x0, width


def hardline(x0, y0):
    """The exit: Tank tracing a hardline. The teleport scroll's 3 s
    channel to an allied tower, inhibitor or the nexus."""
    clear(x0 - 1, y0, x0 + 21, y0 + 11)
    for j in range(20):
        put(x0 + j, y0, "▄", MID)
        put(x0 + j, y0 + 8, "▄", MID)
    for k in range(1, 9):
        put(x0, y0 + k, "█", DIM)
        put(x0 + 19, y0 + k, "█", DIM)
    text(x0 + 2, y0 + 1, "> dial ", DIM)
    text(x0 + 9, y0 + 1, "EXIT", AMBER)
    text(x0 + 14, y0 + 1, "[T]", DIM)
    text(x0 + 2, y0 + 2, "((( ring ring )))", AMBER)
    dests = [("tower", "◄ near"), ("inhibitor", ""), ("nexus", "")]
    for k, (name, mark) in enumerate(dests):
        text(x0 + 2, y0 + 3 + k, name, CYAN)
        text(x0 + 12, y0 + 3 + k, mark, WHITE)
    text(x0 + 2, y0 + 6, "▓" * 9 + "▒░", BRIGHT)
    text(x0 + 13, y0 + 6, "3.0s", HEAD)
    text(x0 + 2, y0 + 7, "> arrived. ok.", HEAD)
    text(x0 + 6, y0 + 9, "▄▄█████▄▄", DIM)
    text(x0 + 1, y0 + 10, "▄" * 18, MID)
    text(x0 + 1, y0 + 11, "▀" + "▪" * 16 + "▀", DIM)
    for j in range(16):
        put(x0 + 2 + j, y0 + 11, "▪", GOLD if j % 5 == 2 else DIM)


GOLD = (255, 210, 90)
LOADED = [
    "ancient teleport", "heal group", "vengeance", "ice barrage", "entangle",
    "tele block", "teleport scroll", "no_mobility", "disarm", "scramble",
    "one_of", "pure chaos",
]


def spellbook(x0, y0):
    """Everything this edition loads, like Tank's training programs."""
    clear(x0 - 1, y0, x0 + 22, y0 + len(LOADED) - 1)
    for k, name in enumerate(LOADED):
        yy = y0 + k
        hot = name == "ice barrage"
        text(x0, yy, name, WHITE if hot else CYAN)
        text(x0 + len(name), yy, "." * (18 - len(name)), FAINT)
        text(x0 + 19, yy, "ok", HEAD if hot else MID)


clear(2, 2, W - 3, 2)
lx, lw = code_logo(3, "MOBASCAPE")
hardline(4, 3)
spellbook(125, 3)
text(3, 2, "▐ hARDLINE ▌", DIM)
text(W - 18, 2, "▐ sPELLBOOK ▌", DIM)
top = "  oPERATOR, i nEED aN eXIT.   hARDLINE tRACED: aLLIED tOWER, tHREE sECONDS.   rING.  "
assert len(top) <= 110, len(top)
center(2, top, MID)
fade_bar(1, 2, W - 3)
clear(lx - 3, 15, lx + lw + 2, 15)
ticker = " ◄◄ D ◄◄ F ◄◄ T ◄◄ TASTE VENGEANCE! ◄◄ THERE IS NO SPOON ◄◄ MAGIC HEROES ONLY"
run = (ticker * 3)[: lw]
for i, ch in enumerate(run):
    put(lx + i, 15, ch, ORANGE if ch == "◄" else AMBER)
center(16, "  ░▒▓  n O D E   1 6 :   t H E   h A R D L I N E   //   d ,  f   f O R   m A G E S ,   t   f O R   a L L  ▓▒░  ", AMBER)
center(17, "  " + elite("i know ice barrage.") + "   " + elite("show me.") + "  ", WHITE)
fade_bar(18, 2, W - 3)

# --- release info -------------------------------------------------------------
header(19, elite("release info"))
clear(4, 20, W - 5, 22)
info = [
    ("rELEASE dATE", "2026-11-25", "cOMMITS", "573ca67 .. d989e86 + docs  ->  sam/main"),
    ("tYPE", "mAGIC: gFX, sCROLLS, sPELLS, cHAOS eLEMENTAL", "tESTS", "462 pass // clippy clean"),
    ("wAS", "tINTED rINGS; nO wAY hOME bUT tHE lONG wALK", "nOW", "46 cache graphics, T for all, D/F for mages"),
]
for i, (k1, v1, k2, v2) in enumerate(info):
    assert len(v1) <= 50 and 89 + len(v2) <= W - 6, (v1, v2)
    yy = 20 + i
    text(8, yy, f"{k1:.<16}", DIM)
    text(25, yy, v1, RED if k1 == "wAS" else BRIGHT)
    text(76, yy, f"{k2:.<12}", DIM)
    text(89, yy, v2, HEAD)

# --- left: the spellbook picker -------------------------------------------------
box(4, 24, 74, 46, elite("spellbook.dat // hero select"))
LX = 6
IW = 56


def wall(left, middle, right, fill="═"):
    run = IW + 4 - len(left) - len(middle) - len(right)
    half = run // 2
    return " " + left + fill * half + middle + fill * (run - half) + right + " "


S_BG = (46, 39, 31)
CHIP_BG = (26, 22, 18)
CHIP_FG = (236, 226, 200)
S_RULE = (110, 92, 66)
for j, ch in enumerate(wall("╔═", "[stone picker]", "═╗")):
    if ch != " ":
        put(LX + j, 26, ch, STONE if ch in "╔╗═" else DIM)
for j, ch in enumerate(wall("╚═", "", "═╝")):
    if ch != " ":
        put(LX + j, 39, ch, STONE)
PX = LX + 3
for k in range(1, 13):
    put(LX + 1, 26 + k, "║", STONE)
    put(LX + IW + 4, 26 + k, "║", STONE)
shade(LX + 2, 27, LX + IW + 3, 38, S_BG)

PURPLE = (190, 110, 255)
ICE = (150, 220, 255)
GREEN = (90, 230, 90)
VINE = (150, 200, 80)
books = [
    ("ANCIENT", [("Ancient Teleport", "blink 450", "110 s", PURPLE), ("Ice Barrage", "freeze 1.5", "90 s", ICE)]),
    ("LUNAR", [("Heal Group", "15% + 10%", "100 s", GREEN), ("Vengeance", "75% back", "60 s", RED)]),
    ("STANDARD", [("Entangle", "root 320", "90 s", VINE), ("Tele Block", "no blinks", "90 s", PURPLE)]),
]
for c, (book, spells) in enumerate(books):
    x = PX + c * 19
    text(x, 27, book, ORANGE)
    for k, (name, what, cd, colour) in enumerate(spells):
        yy = 29 + k * 2
        put(x, yy, "◆", colour)
        text(x + 2, yy, name, YELLOW if name == "Ice Barrage" else STONE)
        text(x + 2, yy + 1, what, S_RULE)
        text(x + 18 - len(cd), yy + 1, cd, (200, 160, 90))
text(PX, 28, "─" * IW, S_RULE)
text(PX, 33, "─" * IW, S_RULE)
text(PX, 34, "LANE", ORANGE)
text(PX + 9, 34, "D", ORANGE)
text(PX + 28, 34, "F", ORANGE)
lanes = [
    ("Mid", "Ancient Teleport", "Ice Barrage"),
    ("Support", "Heal Group", "Vengeance"),
    ("Other", "Ancient Teleport", "Heal Group"),
]
for k, (lane, d, f) in enumerate(lanes):
    yy = 35 + k
    text(PX, yy, lane, STONE)
    text(PX + 9, yy, d, YELLOW)
    text(PX + 28, yy, f, YELLOW)
text(PX + 41, 35, "a default,", S_RULE)
text(PX + 41, 36, "never a rule", S_RULE)
text(PX, 38, "Melee / ranged: slots greyed, the scroll icon only", S_RULE)

text(LX + 1, 40, "THE SENTENCE", HEAD)
text(LX + 14, 40, "// Status::describe, from the numbers", DIM)
src = '{"type":"disarm","duration":2} {"type":"scramble","distance":350}'
assert LX + 2 + len(src) <= 73, len(src)
text(LX + 2, 41, src, CYAN)
for i, ch in enumerate(src):
    if ch.isdigit() or ch == ".":
        put(LX + 2 + i, 41, ch, AMBER)
said = '"disarm for 2 s" "throw 350 in a random..."'
assert LX + 17 + len(said) <= 73, len(said)
text(LX + 2, 42, "└─describe()─►", DIM)
text(LX + 17, 42, said, BRIGHT)

term = [
    ("> ", "operator, i need an exit."),
    ("> ", "i know ice barrage."),
    ("> ", "show me."),
]
for k, (prompt, line) in enumerate(term):
    yy = 43 + k
    text(LX + 1, yy, prompt, DIM)
    text(LX + 3, yy, line, BRIGHT if k < 2 else HEAD)
put(LX + 3 + len(term[-1][1]) + 1, 45, "█", BRIGHT)
notes = ["6 spells, 3 books, 2 slots", "bots pick by lane", "and cast by rule"]
for k, line in enumerate(notes):
    assert LX + 33 + len(line) <= 73, line
    text(LX + 33, 43 + k, line, MID)

# --- right: the exit and the elemental -----------------------------------------
box(76, 24, W - 5, 46, elite("hardline.dat + chaos.dat"))
RX = 79
text(RX, 26, "THE EXIT [T]", HEAD)
text(RX + 13, 26, "// every hero, like Dota's town portal", DIM)
text(RX, 27, "─" * 64, FAINT)
text(RX, 28, "read", CYAN)
text(RX + 6, 28, "▓" * 28 + "▒░", BRIGHT)
text(RX + 38, 28, "3.0 s ► tower/inhib/nexus", HEAD)
text(RX, 29, "BREAKS", RED)
text(RX + 8, 29, "stun · knock-up · fear · taunt · knockback · pull", MID)
text(RX, 30, "CANCELS", AMBER)
text(RX + 8, 30, "move · cast", MID)
text(RX + 24, 30, "REFUSED", RED)
text(RX + 32, 30, "while Tele Blocked (no_mobility)", MID)
text(RX, 31, "SEEN", CYAN)
text(RX + 8, 31, "by both teams: 678 out, 111 in, 750 marker, ping", MID)
text(RX, 32, "─" * 64, FAINT)
text(RX, 33, "CHAOS ELEMENTAL", HEAD)
text(RX + 16, 33, "// npc 2054 · MAGIC · 520 hp · 600 range", DIM)
kit = [
    ("P", "Pure chaos", "every 3rd ability hit: slow|venom|drain", "558"),
    ("Q", "Discord", "skillshot, 80 dmg + 30 mana drained", "557"),
    ("W", "Confusion", "burst; each enemy thrown 350, anywhere", "552"),
    ("E", "Madness", "skillshot, 45 dmg, disarmed for 2 s", "559"),
    ("R", "Chaos Storm", "1.2 s: 3 fans x 3 bolts, each a curse", "557"),
]
for k, (key, name, what, gfx) in enumerate(kit):
    yy = 34 + k
    assert len(what) <= 40, what
    text(RX, yy, f" {key} ", CHIP_FG)
    shade(RX, yy, RX + 2, yy, CHIP_BG)
    text(RX + 4, yy, name, AMBER)
    text(RX + 17, yy, what, MID)
    text(RX + 60, yy, gfx, CYAN)
text(RX, 39, "─" * 64, FAINT)
text(RX, 40, "THERE IS NO SPOON", HEAD)
text(RX + 18, 40, "// one_of: each bolt picks its curse", DIM)
text(RX, 41, "one_of", CYAN)
text(RX + 7, 41, "[drain 40 | throw 250 | disarm 1.5]", AMBER)
text(RX + 44, 41, "──► rng ──► gfx", BRIGHT)
text(RX, 42, "- 111 sparkle: purple blob at camp size", (200, 90, 80))
text(RX + 41, 42, "+ 552 green splash", MID)
text(RX, 44, "TESTS", HEAD)
text(RX + 7, 44, "462 pass  +1: disarm, scramble, one_of, chaos text", MID)
text(RX, 45, "FLAGS", HEAD)
text(RX + 7, 45, "--demo-gfx IDS  --spells a,b  --spell-picker N", MID)

# --- sysop's note ------------------------------------------------------------
header(48, elite("sysop's note // the hardline"))
clear(4, 49, W - 5, 50)
open_ = "sTILL oPEN: mUSIC tOGGLE (p4.6), cLIFFS + bRIDGES + gOD sTATUES (p5), iTEM sETS (p3), sECOND-wAVE sPELLS (sMITE, tHRALL)."
refs = "spells::SpellBook // animated::battle // scroll::blocked // Status::{Disarm, Scramble, OneOf} // Passive::Chaos"
assert len(open_) <= 142 and len(refs) <= 142, (len(open_), len(refs))
center(49, open_, HEAD)
center(50, refs, DIM)

# --- roadmap -------------------------------------------------------------------
header(51, elite("roadmap // docs/ROADMAPPLAN.md"))
clear(4, 52, W - 5, 56)
phases = [
    ("P1 LIGHT", 5), ("P2 MINIMAP", 6), ("P3 SHOP", 7), ("P4 SELECT", 9),
    ("P5 ARENA", 1), ("P6 HUD", 4), ("P7 POST", 0),
]
for i, (name, done) in enumerate(phases):
    x = 8 + (i % 4) * 35
    yy = 52 + (i // 4) * 2
    text(x, yy, f"{name:<11}", CYAN)
    text(x + 11, yy, "[", DIM)
    for j in range(10):
        put(x + 12 + j, yy, "▓" if j < done else "░", BRIGHT if j < done else FAINT)
    text(x + 22, yy, f"] {done * 10:>3}%", HEAD if done else FAINT)
nxt = ">> next: the music toggle (P4.6), cliffs and bridges from cache objects (P5), second-wave spells (Blood Barrage, Smite, Thrall)"
assert 8 + len(nxt) <= W - 6, len(nxt)
text(8, 56, nxt, MID)

fade_bar(57, 2, W - 3)
clear(10, 58, W - 11, 61)
greetz = "gREETZ: tRINITY (tHE hARDLINE) * tHE cHAOS eLEMENTAL (nO sPOON) * kOLODION * jAD * tHE wISE oLD mAN * vENGEANCE"
assert len(greetz) <= 128, len(greetz)
center(58, greetz, MID)
center(59, "nOT AFFILIATED WITH JAGEX. cACHE ASSETS STAY OUT OF GIT. aLWAYS.", DIM)
center(60, "  o p e r a t o r ,   i   n e e d   a n   e x i t .   i   k n o w   i c e   b a r r a g e .   s h o w   m e .  ", HEAD)
center(61, "-=[ mObAsCaPe // 2026 // nODE 16: tHE hARDLINE // 57.6k // sYSoP: rIFT // 6 sPELLS, 1 sCROLL, 1 cHAOS eLEMENTAL ]=-", AMBER)

# --- outputs ------------------------------------------------------------------
with open(out + ".txt", "w") as f:
    for row in grid:
        f.write("".join(c for c, _ in row).rstrip() + "\n")

with open(out + ".ans", "w") as f:
    for y, row in enumerate(grid):
        last = None
        line = []
        for x, (ch, fg) in enumerate(row):
            back = bg.get((x, y))
            if (fg, back) != last:
                line.append(f"\x1b[38;2;{fg[0]};{fg[1]};{fg[2]}m")
                line.append(f"\x1b[48;2;{back[0]};{back[1]};{back[2]}m" if back else "\x1b[49m")
                last = (fg, back)
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
        px, py = 8 + x * cw, 8 + y * ch_h
        if (x, y) in bg:
            d.rectangle((px, py, px + cw - 1, py + ch_h - 1), fill=bg[(x, y)])
        if c == " ":
            continue
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
        elif c in "●•":
            # LED dots as real discs, nearly the two-column dot pitch wide.
            r = cw * (0.78 if c == "●" else 0.3)
            cx, cy = px + cw / 2, py + ch_h / 2
            d.ellipse((cx - r, cy - r, cx + r, cy + r), fill=fg)
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
