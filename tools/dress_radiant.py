#!/usr/bin/env python3
"""Radiant-pocket set dressing for map.json, from cache objects only.

Writes the `props.place` set pieces and biome-scoped `props.scatter` layers
for the four radiant pockets (Human City, Human Keep, Elven Lands, Gnome
Stronghold) and trims their transplanted-scenery budgets.

Every placement is checked first against the lanes, river, jungle camps,
structures and its pocket's bounds; if any fails, the script prints the
problems and writes nothing.

Idempotent: it first drops every place inside a radiant pocket and every
scatter scoped to one, then appends its own, so re-running after an edit
replaces the radiant dressing and leaves the dire side untouched.

Usage: python3 tools/dress_radiant.py
"""
import json
import math
import sys
from pathlib import Path

PATH = Path(__file__).resolve().parent.parent / 'data/maps/karamja_rift/map.json'
m = json.load(open(PATH))
props = m['props']

def segd(p, a, b):
    ax, ay = a; bx, by = b; px, py = p
    dx, dy = bx - ax, by - ay; L = dx * dx + dy * dy
    t = 0 if L == 0 else max(0, min(1, ((px - ax) * dx + (py - ay) * dy) / L))
    return math.hypot(px - ax - t * dx, py - ay - t * dy)

def pd(p, pts):
    return min(segd(p, pts[i], pts[i + 1]) for i in range(len(pts) - 1))

lanes = [l['waypoints'] for l in m['lanes']]
structs = [s['tile'] for l in m['lanes'] for k in ('towers', 'inhibitors', 'watchtowers') for s in l.get(k, [])]
camps = [c['tile'] for c in m['camps']]
riv = m['river']['points']; rw = m['river']['width']
biomes = {b['id']: b for b in m['biomes']}

def pocket_of(x, y):
    for b in m['biomes']:
        ox, oy = b['origin']; w, h = b['rect'][2], b['rect'][3]
        if ox <= x < ox + w and oy <= y < oy + h:
            return b['id']
    return None

problems = []
places = []

def P(model, x, y, yaw=0, scale=1.5, block=1, plaza=0.0, pocket=None):
    p = (x, y)
    lane = min(pd(p, l) for l in lanes)
    river = pd(p, riv)
    camp = min(math.hypot(x - c[0], y - c[1]) for c in camps)
    struct = min(math.hypot(x - s[0], y - s[1]) for s in structs)
    where = pocket_of(x, y)
    if block and lane < 6.5: problems.append(f'{model}@{p} lane {lane:.1f}')
    if river < rw + 1.5: problems.append(f'{model}@{p} river {river:.1f}')
    if block and camp < 5.0: problems.append(f'{model}@{p} camp {camp:.1f}')
    if block and struct < 4.0: problems.append(f'{model}@{p} structure {struct:.1f}')
    if pocket and where != pocket: problems.append(f'{model}@{p} outside {pocket} ({where})')
    places.append({
        'model': model, 'at': [float(x), float(y)], 'yaw': yaw, 'scale': scale,
        'block': block, 'texture': 'plain', 'plaza': plaza,
    })

def S(pocket, models, count, scale, seed, block=0, lane_clear=5.5):
    return {'models': models, 'biome': pocket, 'count': count, 'scale': scale,
            'block': block, 'texture': 'plain', 'lane_clear': lane_clear, 'seed': seed}

# ---------------------------------------------------------------- Human City
# A Varrock market street along the west side of the top lane, stalls facing
# the road, and a little well square between the lane and the river.
C = 'radiant_human_city'
# The lane runs from (2718,3060) to (2760,2980): direction (42,-80).
dx, dy = 42 / math.hypot(42, 80), -80 / math.hypot(42, 80)
nx, ny = -dy, dx  # perpendicular; flip to point west of the lane
if nx > 0: nx, ny = -nx, -ny
road_yaw = round(math.degrees(math.atan2(dy, dx)))
stalls = [  # (base, goods)
    (24905, None), (1421, 1445), (1421, 1485), (24903, None), (1421, 1524),
    (1421, 4631), (1421, 1484),
]
for i, (base, goods) in enumerate(stalls):
    s = 3.0 + i * 3.3
    # point on the lane at distance s from y=2998 heading south-east
    lx = 2718 + 42 * (3060 - 2998) / 80 + dx * s
    ly = 2998 + dy * s
    x, y = round(lx + nx * 7.6), round(ly + ny * 7.6)
    P(base, x, y, yaw=road_yaw, scale=1.45, block=2, plaza=1.5, pocket=C)
    if goods:
        P(goods, x, y, yaw=road_yaw, scale=1.45, block=0, pocket=C)
# lamps and barrels between the stalls, on the road side
for i in range(4):
    s = 3.3 + i * 6.8
    lx = 2718 + 42 * (3060 - 2998) / 80 + dx * s
    ly = 2998 + dy * s
    P(1324, round(lx + nx * 6.9), round(ly + ny * 6.9), yaw=road_yaw + 90, scale=1.5, block=1, pocket=C)
P(4615, 2746, 2991, yaw=20, scale=1.5, block=1, pocket=C)
P(7699, 2745, 2988, yaw=5, scale=1.4, block=1, pocket=C)
P(3817, 2746, 2985, yaw=0, scale=1.4, block=1, pocket=C)
P(1460, 2747, 2979, yaw=0, scale=1.5, block=1, pocket=C)
P(1460, 2746, 2977, yaw=40, scale=1.5, block=1, pocket=C)
P(7699, 2749, 2971, yaw=30, scale=1.4, block=1, pocket=C)
P(1453, 2746, 2973, yaw=0, scale=1.4, block=2, plaza=1, pocket=C)
P(2040, 2763, 2994, yaw=0, scale=1.6, block=1, pocket=C)
# the well square
P(1410, 2768, 2989, yaw=0, scale=1.6, block=2, plaza=3, pocket=C)
P(1324, 2765, 2992, yaw=0, scale=1.5, block=1, pocket=C)
P(1324, 2769, 2986, yaw=180, scale=1.5, block=1, pocket=C)
P(1496, 2771, 2992, yaw=0, scale=1.4, block=1, pocket=C)
P(1496, 2765, 2986, yaw=180, scale=1.4, block=1, pocket=C)
P(1527, 2774, 2993, yaw=225, scale=1.8, block=1, plaza=1, pocket=C)

# ---------------------------------------------------------------- Human Keep
# A Falador knights' training yard: the White Knight statue between blue
# banners, an archery range down the west side, a dummy line down the east.
K = 'radiant_human_keep'
cx, cy = 2762, 3057
P(40158, cx, cy, yaw=180, scale=1.4, block=3, plaza=4.5, pocket=K)
for bx, by in [(cx - 3, cy - 3), (cx + 3, cy - 3), (cx - 3, cy + 3), (cx + 3, cy + 3)]:
    P(14664, bx, by, yaw=180, scale=1.7, block=1, pocket=K)
for y in [3050, 3054, 3058, 3062]:
    P(1451, 2750, y, yaw=90, scale=1.7, block=1, pocket=K)
P(12896, 2750, 3066, yaw=90, scale=1.6, block=1, pocket=K)
for x, y, mdl in [(2774, 3050, 1286), (2774, 3054, 25177), (2774, 3058, 32307), (2774, 3062, 1286), (2774, 3066, 25177)]:
    P(mdl, x, y, yaw=270, scale=1.7, block=1, pocket=K)
P(5946, 2757, 3047, yaw=0, scale=1.6, block=1, pocket=K)
P(5944, 2757, 3047, yaw=0, scale=1.6, block=0, pocket=K)
P(5946, 2767, 3047, yaw=0, scale=1.6, block=1, pocket=K)
P(5945, 2767, 3047, yaw=0, scale=1.6, block=0, pocket=K)
P(12561, 2760, 3046, yaw=0, scale=1.5, block=1, pocket=K)
P(47426, 2764, 3046, yaw=0, scale=1.5, block=1, pocket=K)
P(30875, 2778, 3046, yaw=270, scale=1.6, block=1, pocket=K)
P(30875, 2746, 3046, yaw=90, scale=1.6, block=1, pocket=K)
P(40149, cx, cy + 11, yaw=0, scale=1.3, block=3, plaza=3, pocket=K)
P(1496, 2756, 3068, yaw=0, scale=1.4, block=1, pocket=K)
P(1496, 2768, 3068, yaw=0, scale=1.4, block=1, pocket=K)
P(4615, 2777, 3070, yaw=15, scale=1.5, block=1, pocket=K)
P(1460, 2747, 3070, yaw=0, scale=1.5, block=1, pocket=K)

# --------------------------------------------------------------- Elven Lands
# A crystal grove round the Fragment of Seren: elven lamps, the statues of
# Glarial and Baxtorian, crystal trees and arcane crystal clusters.
E = 'radiant_elven_lands'
ex, ey = 2804, 3082
P(38604, ex, ey, yaw=0, scale=1.5, block=3, plaza=4.5, pocket=E)
for i in range(6):
    a = math.radians(30 + i * 60)
    P(37359, round(ex + math.cos(a) * 5.5), round(ey + math.sin(a) * 5.5), scale=1.05, block=1, pocket=E)
P(38323, ex - 8, ey, yaw=90, scale=1.7, block=2, plaza=1.5, pocket=E)
P(38322, ex + 8, ey, yaw=270, scale=1.7, block=2, plaza=1.5, pocket=E)
for x, y in [(2796, 3075), (2812, 3074), (2797, 3091), (2811, 3090), (2804, 3096)]:
    P(37353, x, y, yaw=(x * 37) % 360, scale=1.8, block=2, plaza=1.5, pocket=E)
P(30686, 2793, 3083, yaw=20, scale=0.55, block=2, pocket=E)
P(30691, 2816, 3081, yaw=200, scale=0.6, block=1, pocket=E)
P(30693, 2804, 3073, yaw=0, scale=0.7, block=2, pocket=E)
P(37905, 2808, 3088, yaw=180, scale=1.5, block=1, pocket=E)
P(13191, 2800, 3088, yaw=0, scale=1.4, block=2, plaza=1, pocket=E)

# ---------------------------------------------------------- Gnome Stronghold
# The spirit tree, a glider landing pad, a gnomeball pitch, a cafe under the
# tree and an agility corner, all east of the mid lane.
G = 'radiant_gnome_stronghold'
gx, gy = 2853, 3087
P(8110, gx, gy, yaw=0, scale=2.3, block=3, plaza=4.5, pocket=G)
P(1323, 2857, 3076, yaw=90, scale=1.4, block=3, plaza=3, pocket=G)
P(5647, 2852, 3079, yaw=0, scale=1.3, block=1, pocket=G)
P(5647, 2862, 3079, yaw=0, scale=1.3, block=1, pocket=G)
P(1317, 2846, 3099, yaw=90, scale=1.6, block=1, pocket=G)
P(1317, 2861, 3099, yaw=270, scale=1.6, block=1, pocket=G)
P(10348, 2853, 3102, yaw=0, scale=1.6, block=1, pocket=G)
for x, y, mdl in [(2847, 3091, 1302), (2859, 3091, 1303), (2847, 3083, 1304), (2859, 3083, 1302)]:
    P(mdl, x, y, yaw=(225 if x < gx else 135), scale=1.6, block=1, pocket=G)
for x, y in [(2843, 3095), (2839, 3100)]:
    P(17144, x, y, scale=1.5, block=1, pocket=G)
    P(17136, x + 1, y, yaw=270, scale=1.4, block=0, pocket=G)
    P(17136, x - 1, y, yaw=90, scale=1.4, block=0, pocket=G)
P(17137, 2836, 3096, yaw=0, scale=1.5, block=1, pocket=G)
P(17128, 2835, 3102, yaw=0, scale=1.5, block=1, pocket=G)
P(17126, 2833, 3099, yaw=0, scale=1.5, block=1, pocket=G)
P(17140, 2862, 3085, yaw=0, scale=1.5, block=1, pocket=G)
P(17139, 2862, 3087, yaw=90, scale=1.5, block=1, pocket=G)
P(24742, 2862, 3094, yaw=90, scale=1.6, block=1, pocket=G)
P(1439, 2860, 3071, yaw=0, scale=1.6, block=1, pocket=G)

# ------------------------------------------------------------------ scatter
scatter = [
    S(C, [1460, 1457, 1495, 7699, 3817], 14, [1.2, 1.5], 501, block=1),
    S(C, [1569, 1610], 16, [1.2, 1.6], 502),
    S(K, [1460, 4615, 7699, 3817], 10, [1.3, 1.5], 601, block=1),
    S(K, [1569, 1610, 1720], 24, [1.2, 1.6], 602),
    S(E, [2168, 2169, 2170], 16, [0.9, 1.4], 701, block=1),
    S(E, [30691, 30693], 6, [0.35, 0.5], 702, block=1),
    S(E, [1569, 1610, 15806], 40, [1.2, 1.7], 703),
    S(E, [37351, 37355, 37358], 10, [1.5, 2.0], 704, block=1),
    S(G, [42533, 42532], 14, [1.1, 1.7], 801),
    S(G, [1720, 1577, 1680, 1569], 26, [1.2, 1.6], 802),
    S(G, [17126, 17128, 17139, 17140], 6, [1.3, 1.5], 803, block=1),
]

if problems:
    print('\n'.join(problems)); sys.exit(1)

radiant = {b for b in biomes if b.startswith('radiant')}
props['place'] = [p for p in props['place'] if pocket_of(*p['at']) not in radiant]
props['scatter'] = [s for s in props['scatter'] if s.get('biome') not in radiant]
props['place'] += places
props['scatter'] += scatter
budgets = {K: 35, E: 60, G: 50}
for b in m['biomes']:
    if b['id'] in budgets: b['budget'] = budgets[b['id']]
open(PATH, 'w').write(json.dumps(m, indent=2) + '\n')
print(f'{len(places)} places, {len(scatter)} scatters')
