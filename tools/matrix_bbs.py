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
GOLD = (255, 210, 90)
STONE = (206, 192, 156)
STONE_DIM = (128, 116, 92)
ORANGE = (255, 152, 31)
YELLOW = (255, 255, 0)
LED_OFF = (0, 44, 12)
LED_GRAD = [WHITE, HEAD, (120, 255, 150), BRIGHT, BRIGHT, MID, (0, 160, 40)]

# A 5x7 dot-matrix font, the scoreboard kind.
DOTS = {
    "M": ["#...#", "##.##", "#.#.#", "#.#.#", "#...#", "#...#", "#...#"],
    "O": [".###.", "#...#", "#...#", "#...#", "#...#", "#...#", ".###."],
    "B": ["####.", "#...#", "#...#", "####.", "#...#", "#...#", "####."],
    "A": [".###.", "#...#", "#...#", "#####", "#...#", "#...#", "#...#"],
    "S": [".####", "#....", "#....", ".###.", "....#", "....#", "####."],
    "C": [".###.", "#...#", "#....", "#....", "#....", "#...#", ".###."],
    "P": ["####.", "#...#", "#...#", "####.", "#....", "#....", "#...."],
    "E": ["#####", "#....", "#....", "####.", "#....", "#....", "#####"],
    "5": ["#####", "#....", "####.", "....#", "....#", "#...#", ".###."],
}


def led_logo(y, word):
    """The logo on an LED board: every dot of the panel is there, dark or
    lit, two columns apart, with one dark column between letters and a dark
    margin. Lit dots take a top-down gradient; where two lit dots sit side
    by side the gap between them glows faintly, like LED bleed."""
    cols = len(word) * 6 - 1 + 2
    x0 = (W - cols * 2) // 2
    lit = set()
    for i, letter in enumerate(word):
        for r, line in enumerate(DOTS[letter]):
            for c, px in enumerate(line):
                if px == "#":
                    lit.add((1 + i * 6 + c, r))
    clear(x0 - 1, y, x0 + cols * 2, y + 6)
    for r in range(7):
        for c in range(cols):
            x = x0 + c * 2
            if (c, r) in lit:
                put(x, y + r, "●", LED_GRAD[r])
                if (c + 1, r) in lit:
                    put(x + 1, y + r, "•", MID if r > 3 else BRIGHT)
            else:
                put(x, y + r, "·", LED_OFF)
    return x0, cols * 2


def led_digit(x0, y0, ch, colour):
    for r, line in enumerate(DOTS[ch]):
        for c, px in enumerate(line):
            put(x0 + c * 2, y0 + r, "●" if px == "#" else "·", colour if px == "#" else LED_OFF)


def phone(x0, y0):
    """The hardline: a desk phone, handset on its cradle."""
    art = [
        "▄▄▄▄▄▄▄▄▄▄▄▄",
        "█▀▀▀▀▀▀▀▀▀▀█",
        "▀▀█▄▄▄▄▄▄█▀▀",
        "  █ ▪ ▪ ▪ █",
        "  █ ▪ ▪ ▪ █",
        "  █ ▪ ▪ ▪ █",
        " ▀▀▀▀▀▀▀▀▀▀",
    ]
    clear(x0 - 1, y0, x0 + 13, y0 + len(art) - 1)
    for k, line in enumerate(art):
        for j, ch in enumerate(line):
            if ch != " ":
                put(x0 + j, y0 + k, ch, GOLD if ch == "▪" else (HEAD if k < 3 else MID))


clear(2, 2, W - 3, 2)
lx, lw = led_logo(3, "MOBASCAPE")
phone(5, 3)
clear(W - 15, 3, W - 3, 9)
led_digit(W - 13, 3, "5", YELLOW)
text(3, 2, "▐ hARDLINE ▌", DIM)
text(W - 17, 2, "▐ sTARTING iN ▌", DIM)
center(2, "iNCOMING oN tHE hARDLINE...   pICK uP.   tHE aRENA iS lOADING.", MID)
fade_bar(1, 2, W - 3)
clear(lx - 1, 10, lx + lw, 10)
ticker = " ◄◄ ALL HEROES LOCKED IN ◄◄ GAME STARTING IN 5 ◄◄ LANE CALLS STILL OPEN ◄◄ NO LOBBY"
run = (ticker * 3)[: lw]
for i, ch in enumerate(run):
    put(lx + i, 10, ch, ORANGE if ch == "◄" else AMBER)
center(11, "  ░▒▓  n O D E   1 4 :   w A K E   u P   //   t H E R E   i S   n O   l O B B Y  ▓▒░  ", AMBER)
center(12, "  " + elite("all heroes locked in. game starting in 5. knock, knock.") + "  ", WHITE)
fade_bar(13, 2, W - 3)

# --- release info -------------------------------------------------------------
header(14, elite("release info"))
clear(4, 15, W - 5, 18)
info = [
    ("rELEASE dATE", "2026-11-24", "cOMMIT", "92ae4f0  ->  sam/main"),
    ("tYPE", "hERO sELECT: tHE lOCKED-iN cOUNT", "tESTS", "429 pass // clippy clean"),
    ("wAS", "A FLAT 5V5 CARD LOBBY, AN 8 S WAIT", "nOW", "the reference's own screen, 5 s"),
    ("wAS", "BLUE/RED SLABS, A LOADING BUTTON", "nOW", "438 lines out, 289 in, 0 new screens"),
]
for i, (k1, v1, k2, v2) in enumerate(info):
    yy = 15 + i
    text(8, yy, f"{k1:.<16}", DIM)
    text(25, yy, v1, RED if k1 == "wAS" else BRIGHT)
    text(76, yy, f"{k2:.<12}", DIM)
    text(89, yy, v2, HEAD)

# --- left: the terminal ------------------------------------------------------------
box(4, 20, 74, 45, elite("terminal // knock, knock"))
LX = 6
IW = 56


def wall(left, middle, right, fill="═"):
    run = IW + 4 - len(left) - len(middle) - len(right)
    half = run // 2
    return " " + left + fill * half + middle + fill * (run - half) + right + " "


def row(inner):
    assert len(inner) <= IW, (len(inner), inner)
    return " ║ " + inner.ljust(IW) + " ║ "


bar = [
    wall("╔═[958]", "[954 x n]", "[959]═╗"),
    row("Game mode         " + "All heroes locked in".ljust(IW - 18 - 7) + "[Leave]"),
    row("Bot match - 5v5    Game starting in 5"),
    wall("╚═[960]", "[956 x n]", "[961]═╝"),
]
for k, line in enumerate(bar):
    for j, ch in enumerate(line):
        if ch == " ":
            continue
        if ch in "╔╗╚╝═║":
            col = STONE
        elif k in (0, 3):
            col = DIM
        elif 19 <= j < 44:
            col = ORANGE
        elif ch in "[]" or line[j - 1:j + 6].startswith("Leave") or "Leave" in line[max(0, j - 5):j + 1]:
            col = ORANGE
        else:
            col = (190, 176, 150) if k == 1 else (255, 214, 120)
        put(LX + j, 22 + k, ch, col)
put(LX + bar[2].index("in 5") + 3, 24, "5", YELLOW)

text(LX + 1, 27, "STATE", HEAD)
text(LX + 7, 27, "// hero_select.locked: Option<f32>", DIM)
s28 = "picking ──Lock in / 0:00──► locked 5.0 ──tick──► 0.0 ──► match"
pp, pl, pm = s28.index("picking") + 2, s28.index("locked") + 2, s28.index("match") + 2
s29 = [" "] * (pm + 1)
s29[pp], s29[pl], s29[pm] = "│", "│", "▲"
s30 = [" "] * (pm + 1)
for i, ch in enumerate("└─ Leave ─► menu"):
    s30[pp + i] = ch
for i, ch in enumerate("└─ Enter: cut to 1e-3 "):
    s30[pl + i] = ch
for i in range(pl + len("└─ Enter: cut to 1e-3 "), pm):
    s30[i] = "─"
s30[pm] = "┘"
for k, line in enumerate([s28, "".join(s29), "".join(s30)]):
    for j, ch in enumerate(line):
        if ch == " ":
            continue
        col = DIM if ch in "─│└┘►▲" else CYAN
        if line is s28 and (ch.isdigit() or ch in ".:/") :
            col = AMBER
        put(LX + 2 + j, 28 + k, ch, col)
text(LX + 2 + pl + 1, 29, " lanes: still open", MID)

text(LX + 1, 32, "THE LAST FIVE SECONDS", HEAD)
text(LX + 23, 32, "// one tick per second, then a drawn frame", DIM)
for k, n in enumerate([5, 4, 3, 2, 1]):
    x = LX + 2 + k * 7
    text(x, 33, str(n), YELLOW if k == 0 else AMBER)
    text(x, 34, "█" * n, BRIGHT if k < 2 else MID if k < 4 else DIM)
text(LX + 37, 33, "0", AMBER)
text(LX + 37, 34, "Loading the arena...", ORANGE)
text(LX + 58, 33, "next", DIM)
text(LX + 58, 34, "load", HEAD)

term = [
    ("> ", "wake up, rift..."),
    ("> ", "the lobby is gone."),
    ("> ", "follow the countdown: lock in."),
    ("> ", "knock, knock."),
]
for k, (prompt, line) in enumerate(term):
    yy = 36 + k
    text(LX + 1, yy, prompt, DIM)
    text(LX + 3, yy, line, BRIGHT if k < 3 else HEAD)
put(LX + 3 + len(term[-1][1]) + 1, 39, "█", BRIGHT)
notes = [
    "roster dims around your pick  //  Locked in: grey 293, inert",
    "lane calls stay open: a note to the team, never a pick",
    "--lock-in   opens on the count, for screenshots",
]
for k, line in enumerate(notes):
    assert len(line) <= 66, line
    text(LX + 1, 41 + k, line, MID)

# --- right: reconciled with the reference ---------------------------------------------
box(76, 20, W - 5, 45, elite("diff.dat // the reference @ 0:11"))
RX = 79
text(RX, 22, "REFERENCE", HEAD)
text(RX + 24, 22, "OURS", HEAD)
text(RX + 60, 22, "OK", HEAD)
text(RX, 23, "─" * 64, FAINT)
pairs = [
    ("All heroes locked in", "same: b12 x2, orange"),
    ("Game starting in 5", "same: the 5 in yellow"),
    ("roster dimmed", "55%, your pick outlined"),
    ("Locked in, grey", "sprite 293, inert"),
    ("final pick per row", "green names, pulse stops"),
    ("lane tabs", "still open: notes, not picks"),
    ("no lobby", "no lobby"),
]
for k, (ref, ours) in enumerate(pairs):
    yy = 24 + k
    text(RX, yy, ref, WHITE)
    text(RX + 24, yy, ours, BRIGHT)
    text(RX + 60, yy, "ok", HEAD)
text(RX, 31, "─" * 64, FAINT)
text(RX, 32, "DELETED", RED)
text(RX + 34, 32, "ADDED", BRIGHT)
gone = ["build_lobby  (-221 lines)", "lobby_teams, lobby_card_rects", "lobby_side_tabs, button_*",
        "lobby_open, lobby_deadline", "tick_lobby, \"Rift - Lobby\""]
new = ["LOCKED_SECONDS = 5.0", "lock_in, tick_locked", "starting_in, skip_countdown",
       "ready_to_load", "starting_line, --lock-in"]
for k, (g, n) in enumerate(zip(gone, new)):
    yy = 33 + k
    assert len(g) + 2 < 34 and RX + 34 + len(n) + 2 < W - 6, (g, n)
    text(RX, yy, "- " + g, (200, 90, 80))
    text(RX + 34, yy, "+ " + n, MID)
text(RX, 39, "DIFF", HEAD)
text(RX + 6, 39, "+289", BRIGHT)
text(RX + 11, 39, "▓" * 13, BRIGHT)
text(RX + 26, 39, "-438", RED)
text(RX + 31, 39, "▓" * 19, RED)
text(RX + 52, 39, "net -149", DIM)
text(RX, 41, "TESTS", HEAD)
text(RX + 7, 41, "429 pass  +2: the count to a drawn frame, the skip", MID)
text(RX, 42, "SEEN", HEAD)
text(RX + 7, 42, "1920x1080 + 1280x720, then on into the match", MID)
text(RX, 43, "BOTS", HEAD)
text(RX + 7, 43, "jungle called -> blue bots top, mid, bottom x2", MID)

# --- sysop's note ------------------------------------------------------------
header(47, elite("sysop's note // there is no lobby"))
clear(4, 48, W - 5, 49)
center(48, "sTILL oPEN: bATTLE sPELLS (p4.3), pASSIVE rOW + pOPULAR bUILD (p4.4-5), mUSIC tOGGLE (p4.6), cLIFFS + bRIDGES + gOD sTATUES (p5).", HEAD)
center(49, "hero_select::LOCKED_SECONDS // lock_in // tick_locked // starting_in // skip_countdown // ready_to_load // starting_line", DIM)

# --- roadmap -------------------------------------------------------------------
header(51, elite("roadmap // docs/ROADMAPPLAN.md"))
clear(4, 52, W - 5, 56)
phases = [
    ("P1 LIGHT", 5), ("P2 MINIMAP", 6), ("P3 SHOP", 7), ("P4 SELECT", 6),
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
text(8, 56, ">> next: battle spells (P4.3), passive row + popular build (P4.4-5), cliffs/bridges/statues from cache objects (P5)", MID)

fade_bar(57, 2, W - 3)
clear(10, 58, W - 11, 61)
greetz = "gREETZ: tHE rEFERENCE aT 0:11 (nO lOBBY, jUST a cOUNT) * sPRITE 293 (tHE gREY bUTTON) * b12 aT x2 (tHE bIG 5)"
assert len(greetz) <= 128, len(greetz)
center(58, greetz, MID)
center(59, "nOT AFFILIATED WITH JAGEX. cACHE ASSETS STAY OUT OF GIT. aLWAYS.", DIM)
center(60, "  k n o c k ,   k n o c k .   a l l   h e r o e s   l o c k e d   i n .   g a m e   s t a r t i n g   i n   5 .  ", HEAD)
center(61, "-=[ mObAsCaPe // 2026 // nODE 14: wAKE uP // 57.6k // sYSoP: rIFT // 1 cOMMIT, 0 lOBBIES, 5 sECONDS ]=-", AMBER)

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
