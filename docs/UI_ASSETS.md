# Cache-native UI assets

Every visual asset in both the legacy HUD and the assembled `--ui-live`
interface is scraped from the pinned OSRS cache. No external image tree is
read, and nothing derived from Jagex art is committed.

## Fonts

`crates/client/src/ui_assets.rs` decodes the font measurements from cache index
13 and the matching glyph sheet from index 8:

| Role | Cache font | Use |
|---|---:|---|
| `p11` | 494 `p11_full` | small interface text |
| `p12` | 495 `p12_full` | default interface and overhead names |
| `b12` | 496 `b12_full` | bold hitmark numbers |

`--ui-live` names the same fonts in `data/ui/theme.json`; the legacy HUD asks
`ui_assets` for them directly.

## Combat art

The exact cache hitmark sprites from index 8:

| Role | Sprite |
|---|---:|
| `icon.hitmark.damage` | 1359 |
| `icon.hitmark.max` | 3571 |
| `icon.hitmark.poison` | 1360 |
| `icon.hitmark.disease` | 1361 |
| `icon.hitmark.venom` | 1632 |
| `icon.hitmark.heal` | 1629 |
| `icon.hitmark.miss` | 1358 |
| `icon.skull` | 439 |

## Item icons

OSRS does not store item icons as sprites: the client renders the item's
inventory model. `crates/client/src/item_icons.rs` does the same with the cache
model decoder and a small CPU rasteriser. `main.rs` bakes every item in
`data/items/items.json` that has an `obj` id into the legacy atlas under the
item id, and `theme_file.rs` places the same renders in the live theme under
`icon.item.<key>`.

## Champion ability icons

Champion files name four cache sprite ids in `ability_icons`. The live theme
bakes them under `icon.ability.<champion>.<slot>`; the legacy ability bar asks
for the same key, so a champion swap needs no atlas rebuild.

## Score-strip and overhead icons

The legacy HUD's crossed swords (`icon_swords`, sprite 442) and PK skull
(`icon_skull`, sprite 439) come from cache index 8. The assembled theme names
the skull directly in `data/ui/theme.json`.

## Inspecting art

```sh
# One cache sprite to a PNG
cargo run -p rift-tools -- sprite 1359 /tmp/damage.png

# One cache font and its glyph sheet
cargo run -p rift-tools -- dump-fonts /tmp/fonts

# Every index-8 sprite
cargo run -p rift-tools -- dump-sprites /tmp/sprites
```

The old `assets/osrs-ui` wiki/PNG tree is retired and is not read by the
client. Panels and buttons that have no clean cache interface sprite keep
procedural OSRS-styled geometry rather than borrowing an external image.
