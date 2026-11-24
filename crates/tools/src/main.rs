use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use rift_cache::font::FONT_INDEX;
use rift_cache::sprite::SPRITE_INDEX;
use rift_cache::{
    Cache, Font, ItemDefinition, Model, ObjectDefinition, SpriteDefinition, SpriteSheet, Terrain,
    TextureDefinition,
};
use rift_sim::Match as BattleMatch;

mod interface_dump;
mod model_sheet;
mod picture;

#[derive(Parser, Debug)]
#[command(name = "rift-tools", about = "OSRS cache inspection for Rift")]
struct Cli {
    /// Cache directory containing main_file_cache.dat2.
    #[arg(long, default_value = ".cache/osrs-235/cache", global = true)]
    cache: PathBuf,

    /// XTEA keys JSON. Defaults to keys.json next to the cache.
    #[arg(long, global = true)]
    keys: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Print cache index and reference table stats.
    CacheInfo,
    /// Decode terrain for a region.
    Terrain { region_x: u32, region_y: u32 },
    /// Decode placed locations for a region.
    Locations { region_x: u32, region_y: u32 },
    /// Scan underlay/overlay colors for suspicious values.
    ScanColors,
    /// Print an underlay definition.
    Underlay { id: u32 },
    /// Dump the raw bytes of the material definitions in the config archive.
    Materials {
        #[arg(long, default_value_t = 8)]
        limit: u32,
    },
    /// Print an overlay definition.
    Overlay { id: u32 },
    /// Print an NPC definition.
    Npc { id: u32 },
    /// Search NPC definitions by name substring.
    FindNpc {
        query: String,
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    /// Print an item definition and its inventory-icon fields.
    Item { id: u32 },
    /// List the items whose male wield/wear models include any of these
    /// model ids (which NPC body parts are really held weapons).
    Wielders {
        #[arg(value_delimiter = ',')]
        models: Vec<u32>,
    },
    /// Print an object definition.
    Object { id: u32 },
    /// Search object definitions by name substring.
    FindObject {
        query: String,
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    /// Print a sequence and its first frames.
    Sequence { id: u32 },
    /// Decode a synthesised sound effect (index 4), print its instruments,
    /// and optionally write it as a WAV file.
    Sound {
        id: u32,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Decode every sound effect and report failures; with `--sequences`,
    /// list the sounds those animations play instead.
    Sounds {
        #[arg(long, value_delimiter = ',')]
        sequences: Vec<u32>,
        /// Describe these sound effect ids directly (for example the ids
        /// named in `data/sounds.json`).
        #[arg(long, value_delimiter = ',')]
        ids: Vec<u32>,
        /// Also write each listed sound to this folder as `<id>.wav`.
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Animate a model with a sequence and print frame bounds.
    Animate { model: u32, sequence: u32 },
    /// Print a model summary.
    Model { id: u32 },
    /// Export a model as a Wavefront OBJ.
    ExportObj { id: u32, out: PathBuf },
    /// Export a sprite from index 8 as a PNG.
    Sprite { id: u32, out: PathBuf },
    /// Export the sprite behind a texture definition from index 9 as a PNG.
    Texture { id: u32, out: PathBuf },
    /// Scrape index 9 textures (and optionally every index 8 sprite) to a folder.
    DumpTextures {
        #[arg(long, default_value = "assets/cache-scrape")]
        out: PathBuf,
        /// Also write every index-8 sprite group.
        #[arg(long)]
        sprites: bool,
    },
    /// Export terrain height/colour data as an OBJ for quick inspection.
    TerrainObj {
        region_x: u32,
        region_y: u32,
        out: PathBuf,
    },
    /// Write sprites as PNGs with a report, to check the sprite decoder by
    /// eye (docs/INTERFACE.md, stage 1).
    DumpSprites {
        /// Folder to write into. It is made if it is not there.
        out: PathBuf,
        /// First sprite id. Without it, from the first there is.
        #[arg(long)]
        from: Option<u32>,
        /// Last sprite id. Without it, to the last there is.
        #[arg(long)]
        to: Option<u32>,
        /// Write only the overview sheets and the report, not a file for
        /// every frame.
        #[arg(long)]
        overview_only: bool,
    },
    /// Write every font's letters, measurements and a test line, to check
    /// the font decoder by eye.
    DumpFonts {
        /// Folder to write into.
        out: PathBuf,
    },
    /// Run a deterministic headless 5v5 bot match and print its state hash.
    /// Print a spot animation (projectile / spell graphic) definition.
    Spotanim { id: u32 },
    /// List spot animations in an id range, optionally as a contact sheet of
    /// their models posed half way through their sequence.
    Spotanims {
        #[arg(long, default_value_t = 0)]
        from: u32,
        #[arg(long, default_value_t = 200)]
        to: u32,
        /// Write a labelled PNG contact sheet here.
        #[arg(long)]
        sheet: Option<PathBuf>,
    },
    /// Find sequences that animate the same skeleton as `sequence` (an NPC's
    /// idle or walk), which is where its attack and death clips hide.
    SkeletonAnims {
        sequence: u32,
        /// Pose this model with each candidate and write a contact sheet.
        #[arg(long)]
        model: Option<u32>,
        #[arg(long)]
        sheet: Option<PathBuf>,
        /// Only list ids in this window around the reference sequence.
        #[arg(long, default_value_t = 100_000)]
        window: u32,
    },
    /// Bind each champion's parts to its skeleton and print the decision per
    /// part (native labels or follow the nearest limb) with the measured
    /// drift, plus the limb each label moves. `champion` is a file stem in
    /// data/champions, or `all`.
    Skeleton {
        champion: String,
        /// Write a before/after contact sheet: per hero, a row posed with the
        /// old merge (attachments frozen) and a row with the bound skeleton,
        /// across the walk and attack clips.
        #[arg(long)]
        sheet: Option<PathBuf>,
    },
    Match {
        #[arg(long, default_value_t = 1)]
        seed: u32,
        #[arg(long, default_value_t = 12000)]
        ticks: u64,
        #[arg(long, default_value_t = 64)]
        size: usize,
        /// Print every entity, not just the team summary.
        #[arg(long)]
        verbose: bool,
        /// Print `tick hash` every N ticks (0 = off).
        #[arg(long, default_value_t = 0)]
        trace_every: u64,
    },
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();
    // The headless match is pure simulation and deliberately does not touch
    // the cache: it must be runnable in CI without game assets.
    if let Command::Match {
        seed,
        ticks,
        size,
        verbose,
        trace_every,
    } = &cli.command
    {
        headless_match(*seed, *ticks, *size, *verbose, *trace_every)?;
        return Ok(());
    }
    let cache = Cache::open_with_keys(&cli.cache, cli.keys.as_ref())
        .with_context(|| format!("opening cache at {}", cli.cache.display()))?;

    match cli.command {
        Command::CacheInfo => cache_info(&cache)?,
        Command::Terrain { region_x, region_y } => terrain(&cache, region_x, region_y)?,
        Command::Locations { region_x, region_y } => locations(&cache, region_x, region_y)?,
        Command::ScanColors => scan_colors(&cache)?,
        Command::Underlay { id } => underlay(&cache, id)?,
        Command::Materials { limit } => materials(&cache, limit)?,
        Command::Overlay { id } => overlay(&cache, id)?,
        Command::Npc { id } => npc(&cache, id)?,
        Command::FindNpc { query, limit } => find_npc(&cache, &query, limit)?,
        Command::Item { id } => item(&cache, id)?,
        Command::Wielders { models } => wielders(&cache, &models)?,
        Command::Object { id } => object(&cache, id)?,
        Command::FindObject { query, limit } => find_object(&cache, &query, limit)?,
        Command::Sequence { id } => sequence(&cache, id)?,
        Command::Sound { id, out } => sound(&cache, id, out.as_deref())?,
        Command::Sounds {
            sequences,
            ids,
            out,
        } => {
            if ids.is_empty() {
                sounds(&cache, &sequences, out.as_deref())?
            } else {
                sound_ids(&cache, &ids, out.as_deref())?
            }
        }
        Command::Animate { model, sequence } => animate(&cache, model, sequence)?,
        Command::Skeleton { champion, sheet } => {
            skeleton_report(&cache, &champion, sheet.as_deref())?
        }
        Command::Model { id } => model(&cache, id)?,
        Command::ExportObj { id, out } => export_obj(&cache, id, &out)?,
        Command::Sprite { id, out } => sprite(&cache, id, &out)?,
        Command::DumpSprites {
            out,
            from,
            to,
            overview_only,
        } => dump_sprites(&cache, &out, from, to, overview_only)?,
        Command::DumpFonts { out } => dump_fonts(&cache, &out)?,
        Command::Texture { id, out } => texture(&cache, id, &out)?,
        Command::DumpTextures { out, sprites } => dump_textures(&cache, &out, sprites)?,
        Command::TerrainObj {
            region_x,
            region_y,
            out,
        } => terrain_obj(&cache, region_x, region_y, &out)?,
        Command::Spotanim { id } => spotanim(&cache, id)?,
        Command::Spotanims { from, to, sheet } => spotanims(&cache, from, to, sheet.as_deref())?,
        Command::SkeletonAnims {
            sequence,
            model,
            sheet,
            window,
        } => skeleton_anims(&cache, sequence, model, sheet.as_deref(), window)?,
        Command::Match { .. } => unreachable!("handled before opening the cache"),
    }

    Ok(())
}

fn headless_match(
    seed: u32,
    ticks: u64,
    size: usize,
    verbose: bool,
    trace_every: u64,
) -> Result<()> {
    let mut game = BattleMatch::new(seed, size);
    if trace_every > 0 {
        for _ in 0..ticks {
            if game.winner.is_some() {
                break;
            }
            game.step();
            if game.tick % trace_every == 0 {
                println!("trace tick={} hash={:016x}", game.tick, game.last_tick_hash);
            }
        }
    } else {
        game.run(ticks);
    }
    println!(
        "match seed={seed} size={size} ticks={} winner={:?} hash={:016x}",
        game.tick,
        game.winner,
        game.state_hash()
    );
    println!(
        "stats attacks={} crits={} deaths={} commands={} waves={}",
        game.stats.attacks,
        game.stats.crits,
        game.stats.deaths,
        game.stats.commands,
        game.stats.waves
    );
    for team in 0u8..2 {
        let nexus = game.nexus(team);
        let towers = game
            .entities
            .iter()
            .filter(|entity| {
                entity.team == team
                    && entity.kind == rift_sim::battle::EntityKind::Tower
                    && entity.alive
            })
            .count();
        let inhibitors = game
            .entities
            .iter()
            .filter(|entity| {
                entity.team == team
                    && entity.kind == rift_sim::battle::EntityKind::Inhibitor
                    && entity.alive
            })
            .count();
        let minions = game
            .entities
            .iter()
            .filter(|entity| {
                entity.team == team
                    && entity.kind == rift_sim::battle::EntityKind::Minion
                    && entity.alive
            })
            .count();
        let gold = game
            .entities
            .iter()
            .filter(|entity| entity.team == team)
            .map(|entity| entity.gold)
            .sum::<u32>();
        println!(
            "team {team}: heroes={}/5 towers={towers}/3 inhibitors={inhibitors}/3 minions={minions} gold={gold} nexus={}/{} vulnerable={}",
            game.alive_on(team),
            nexus.map(|entity| entity.hp.max(0)).unwrap_or(0),
            nexus.map(|entity| entity.max_hp).unwrap_or(0),
            game.nexus_vulnerable(team)
        );
    }
    if verbose {
        for entity in &game.entities {
            println!(
                "entity {:03} {:?} team {} lane {} pos=({},{}) tile=({},{}) hp={}/{} lvl={} gold={} {}",
                entity.id,
                entity.kind,
                entity.team,
                entity.lane,
                entity.x,
                entity.z,
                entity.tile().0,
                entity.tile().1,
                entity.hp.max(0),
                entity.max_hp,
                entity.level,
                entity.gold,
                if entity.alive { "alive" } else { "dead" }
            );
        }
    }
    Ok(())
}

fn cache_info(cache: &Cache) -> Result<()> {
    println!("cache: {}", cache.dir().display());
    println!(
        "keys: {}",
        cache
            .keys()
            .map(|k| format!("{} entries", k.len()))
            .unwrap_or_else(|| "none".to_string())
    );

    for index in 0..=255u32 {
        let count = cache.group_count(index);
        if count > 0 {
            let protocol = cache
                .reference(index)
                .ok()
                .map(|r| format!("protocol {}", r.protocol))
                .unwrap_or_else(|| "no reference".to_string());
            println!("index {index:3}: {count:6} groups  {protocol}");
        }
    }
    Ok(())
}

fn map_groups(cache: &Cache, region_x: u32, region_y: u32) -> Result<(u32, u32)> {
    let keys = cache
        .keys()
        .context("cache has no keys.json; cannot resolve map group names")?;
    let locations = keys
        .find_locations_group(region_x, region_y)
        .with_context(|| format!("no locations group for region {region_x},{region_y}"))?;
    if locations == 0 {
        bail!("locations group 0 has no terrain predecessor");
    }
    Ok((locations - 1, locations))
}

fn terrain(cache: &Cache, region_x: u32, region_y: u32) -> Result<()> {
    let (terrain_group, _) = map_groups(cache, region_x, region_y)?;
    let group = cache.group(5, terrain_group)?;
    let terrain = Terrain::load(region_x, region_y, &group.data)?;
    let stats = terrain.stats();

    println!(
        "region {region_x},{region_y}: terrain group {terrain_group}, {} bytes, compression {}",
        group.data.len(),
        group.compression
    );
    println!(
        "tiles: {}  overlays: {}  underlays: {}  blocked: {}",
        terrain.tiles().len(),
        stats.overlays,
        stats.underlays,
        stats.blocked
    );

    let mut top = stats
        .heights
        .iter()
        .enumerate()
        .filter(|&(_, &n)| n > 0)
        .collect::<Vec<_>>();
    top.sort_by_key(|(_, n)| std::cmp::Reverse(**n));
    println!("top heights: {:?}", top.iter().take(8).collect::<Vec<_>>());

    println!("\nplane 0 height map (64x64, one char per tile):");
    let chars = b" .:-=+*#%@";
    for y in (0..64).rev() {
        let mut line = String::with_capacity(64);
        for x in 0..64 {
            let h = terrain.height(0, x, y) as usize;
            line.push(chars[(h * (chars.len() - 1) / 255).min(chars.len() - 1)] as char);
        }
        println!("{line}");
    }

    Ok(())
}

fn locations(cache: &Cache, region_x: u32, region_y: u32) -> Result<()> {
    let (_, locations_group) = map_groups(cache, region_x, region_y)?;
    let group = cache.group(5, locations_group)?;
    let locations = rift_cache::Locations::load(region_x, region_y, &group.data)?;
    let mut counts = locations.counts_by_id().into_iter().collect::<Vec<_>>();
    counts.sort_by_key(|(_, n)| std::cmp::Reverse(*n));

    println!(
        "region {region_x},{region_y}: locations group {locations_group}, {} bytes, {} placements",
        group.data.len(),
        locations.locations.len()
    );
    for (id, count) in counts.iter().take(30) {
        let name = cache
            .config(6, *id)
            .ok()
            .and_then(|data| ObjectDefinition::parse(*id, &data).ok())
            .map(|def| def.name)
            .unwrap_or_default();
        println!("  object {id:6} x{count:<4} {name}");
    }
    Ok(())
}

fn fnv1a(text: &str) -> i32 {
    let mut hash: u32 = 0x811c9dc5;
    for byte in text.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash as i32
}

fn materials(cache: &Cache, limit: u32) -> Result<()> {
    let reference = cache.reference(2)?;
    let names = [
        "materials",
        "material",
        "underlay",
        "overlay",
        "object",
        "npc",
        "variance",
        "item",
        "particle",
        "skeleton",
        "sound",
        "spell",
        "textures",
        "animation",
        "graphic",
        "sound_effect",
    ];
    for archive in &reference.archives {
        let matched = names
            .iter()
            .find(|name| fnv1a(name) == archive.name_hash)
            .copied()
            .unwrap_or("?");
        println!(
            "archive {:2} files={:5} hash={:11} name={matched}",
            archive.id,
            archive.files.len(),
            archive.name_hash
        );
    }
    for archive in [1u32, 4u32] {
        let files = cache.archive_files_with_ids(2, archive)?;
        let mut ok = 0usize;
        let mut bad = Vec::new();
        for (id, data) in &files {
            let parsed = if archive == 1 {
                rift_cache::UnderlayDefinition::parse(*id, data).is_ok()
            } else {
                rift_cache::OverlayDefinition::parse(*id, data).is_ok()
            };
            if parsed {
                ok += 1;
            } else {
                bad.push((
                    *id,
                    data.len(),
                    data.iter().take(20).copied().collect::<Vec<u8>>(),
                ));
            }
        }
        println!("archive {archive}: {ok} parsed, {} failed", bad.len());
        for (id, len, bytes) in bad.iter().take(3) {
            let hex: Vec<String> = bytes.iter().map(|b| format!("{b:02x}")).collect();
            println!("  id {id} len {len}: {}", hex.join(" "));
        }
    }
    for archive in [3u32] {
        let Ok(files) = cache.archive_files_with_ids(2, archive) else {
            continue;
        };
        let lens: Vec<usize> = files.iter().map(|(_, d)| d.len()).collect();
        let min_len = lens.iter().min().copied().unwrap_or(0);
        let max_len = lens.iter().max().copied().unwrap_or(0);
        let uniform = min_len == max_len;
        println!(
            "probe archive {archive}: files={} len={min_len}..{max_len} uniform={uniform}",
            files.len()
        );
        if files.len() < 400 || uniform {
            let mut parsed = 0usize;
            let mut histogram: std::collections::BTreeMap<u16, usize> = Default::default();
            for (_id, data) in &files {
                if data.len() >= 10 {
                    let texture = u16::from_le_bytes([data[8], data[9]]);
                    histogram
                        .entry(texture)
                        .and_modify(|c| *c += 1)
                        .or_insert(1);
                    parsed += 1;
                }
            }
            println!("   parsed {parsed}/{} u16-texture histogram", files.len());
            let top: Vec<(u16, usize)> = histogram.iter().take(12).map(|(k, v)| (*k, *v)).collect();
            println!("   histogram {top:?}");

            for (id, data) in files.iter().take(6) {
                let ints: Vec<i32> = data
                    .chunks_exact(4)
                    .map(|c| i32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                    .collect();
                let bytes: Vec<String> = data.iter().map(|b| format!("{b:02x}")).collect();
                println!("   id {id}: bytes={} ints={ints:?}", bytes.join(" "));
            }
        }
    }
    let files = cache.archive_files_with_ids(2, 0)?;
    println!("material files: {}", files.len());
    for (id, data) in files.iter().take(limit as usize) {
        let head: Vec<String> = data.iter().take(24).map(|b| format!("{b:02x}")).collect();
        let ints: Vec<i32> = data
            .chunks_exact(4)
            .take(6)
            .map(|c| i32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        println!(
            "material {id:5} len={:3} head={} ints={ints:?}",
            data.len(),
            head.join(" ")
        );
    }
    Ok(())
}

fn scan_colors(cache: &Cache) -> Result<()> {
    for (label, archive) in [("underlay", 1u32), ("overlay", 4u32)] {
        let files = cache.archive_files_with_ids(2, archive)?;
        println!("{label}: {} files", files.len());
        for (id, data) in files {
            let color = if archive == 1 {
                rift_cache::UnderlayDefinition::parse(id, &data)
                    .ok()
                    .map(|d| d.color())
            } else {
                rift_cache::OverlayDefinition::parse(id, &data)
                    .ok()
                    .map(|d| d.color())
            };
            if let Some([r, g, b]) = color {
                if r > 140 && b > 140 && g < 110 {
                    println!("  {label} {id}: [{r}, {g}, {b}]");
                }
            }
        }
    }
    Ok(())
}

fn underlay(cache: &Cache, id: u32) -> Result<()> {
    let data = cache.file(2, 1, id)?;
    let def = rift_cache::UnderlayDefinition::parse(id, &data)?;
    println!(
        "underlay {id}: rgb=#{:06x} color={:?}",
        def.rgb,
        def.color()
    );
    Ok(())
}

fn overlay(cache: &Cache, id: u32) -> Result<()> {
    let data = cache.file(2, 4, id)?;
    let def = rift_cache::OverlayDefinition::parse(id, &data)?;
    println!(
        "overlay {id}: rgb=#{:06x} color={:?} texture={} hide_underlay={}",
        def.rgb,
        def.color(),
        def.texture,
        def.hide_underlay
    );
    Ok(())
}

fn npc(cache: &Cache, id: u32) -> Result<()> {
    let data = cache.file(2, 9, id)?;
    let def = rift_cache::NpcDefinition::parse(id, &data)?;
    println!(
        "npc {id}: {:?} models={:?} size={} combat={} stand={} walk={} run={}",
        def.name,
        def.models,
        def.size,
        def.combat_level,
        def.standing_animation,
        def.walking_animation,
        def.run_animation
    );
    println!(
        "  idle_l={} idle_r={} turn180={} turn_l={} turn_r={}",
        def.idle_rotate_left_animation,
        def.idle_rotate_right_animation,
        def.rotate180_animation,
        def.rotate_left_animation,
        def.rotate_right_animation
    );
    Ok(())
}

fn find_npc(cache: &Cache, query: &str, limit: usize) -> Result<()> {
    let files = cache.archive_files_with_ids(2, 9)?;
    let query_lower = query.to_lowercase();
    let mut found = 0usize;
    for (id, data) in files {
        if let Ok(def) = rift_cache::NpcDefinition::parse(id, &data) {
            if def.name.to_lowercase().contains(&query_lower) {
                println!(
                    "npc {:5}: {:?} models={:?} size={} stand={} walk={} run={}",
                    id,
                    def.name,
                    def.models,
                    def.size,
                    def.standing_animation,
                    def.walking_animation,
                    def.run_animation
                );
                found += 1;
                if found >= limit {
                    break;
                }
            }
        }
    }
    println!("{found} match(es)");
    Ok(())
}

fn wielders(cache: &Cache, models: &[u32]) -> Result<()> {
    let mut found = 0;
    for (id, data) in cache.archive_files_with_ids(2, 10)? {
        let Ok(def) = ItemDefinition::parse(id, &data) else {
            continue;
        };
        for model in def.male_models.iter().filter(|&&m| m != 0) {
            if models.contains(model) {
                println!("model {model}: item {id} {:?}", def.name);
                found += 1;
            }
        }
    }
    println!("{found} match(es)");
    Ok(())
}

fn item(cache: &Cache, id: u32) -> Result<()> {
    let data = cache.config(10, id)?;
    let def = ItemDefinition::parse(id, &data)?;
    println!(
        "item {id}: {:?} model={} zoom={} rot={}/{}/{} offset={},{} scale={}/{}/{}",
        def.name,
        def.model,
        def.zoom,
        def.rotation_x,
        def.rotation_y,
        def.rotation_z,
        def.offset_x,
        def.offset_y,
        def.scale_x,
        def.scale_y,
        def.scale_z
    );
    println!(
        "  len={} complete={} value={} stackable={} members={} recolors={:?}",
        data.len(),
        def.complete,
        def.value,
        def.stackable,
        def.members,
        def.recolors
    );
    println!(
        "  wield: male models={:?} offset={}",
        def.male_models, def.male_offset
    );
    Ok(())
}

fn object(cache: &Cache, id: u32) -> Result<()> {
    let data = cache.config(6, id)?;
    let def = ObjectDefinition::parse(id, &data)?;
    println!(
        "object {id}: {:?} models={:?} types={:?} size={}x{} anim={}",
        def.name, def.models, def.model_types, def.size_x, def.size_y, def.animation_id
    );
    println!(
        "  wall_or_door={} contoured={} merge_normals={} clipped={} ambient={} contrast={}",
        def.wall_or_door,
        def.contoured_ground,
        def.merge_normals,
        def.model_clipped,
        def.ambient,
        def.contrast
    );
    Ok(())
}

fn find_object(cache: &Cache, query: &str, limit: usize) -> Result<()> {
    let files = cache.archive_files_with_ids(2, 6)?;
    let query_lower = query.to_lowercase();
    let mut found = 0usize;
    for (id, data) in files {
        if let Ok(def) = ObjectDefinition::parse(id, &data) {
            if def.name.to_lowercase().contains(&query_lower) {
                println!(
                    "object {:6}: {:?} models={:?} types={:?} size={}x{}",
                    id, def.name, def.models, def.model_types, def.size_x, def.size_y
                );
                found += 1;
                if found >= limit {
                    break;
                }
            }
        }
    }
    println!("{found} match(es)");
    Ok(())
}

fn describe_sound(sound: &rift_cache::SoundEffect) -> String {
    let forms: Vec<String> = sound
        .instruments
        .iter()
        .map(|i| {
            let wave = match i.pitch.form {
                1 => "square",
                2 => "sine",
                3 => "saw",
                4 => "noise",
                _ => "off",
            };
            format!(
                "{wave}x{} {}ms@{}{}{}",
                i.oscillators.len(),
                i.duration,
                i.offset,
                if i.filter.pairs != [0, 0] {
                    " filt"
                } else {
                    ""
                },
                if i.pitch_modifier.is_some() {
                    " vib"
                } else {
                    ""
                },
            )
        })
        .collect();
    format!("{}ms [{}]", sound.length_ms(), forms.join(", "))
}

fn sound(cache: &Cache, id: u32, out: Option<&Path>) -> Result<()> {
    let sound = rift_cache::SoundEffect::load(cache, id)?;
    println!("sound {id}: {}", describe_sound(&sound));
    let samples = sound.render();
    let peak = samples.iter().map(|s| s.unsigned_abs()).max().unwrap_or(0);
    println!("  {} samples at 22050 Hz, peak {peak}", samples.len());
    if let Some(out) = out {
        std::fs::write(out, sound.to_wav())?;
        println!("  wrote {}", out.display());
    }
    Ok(())
}

fn sound_ids(cache: &Cache, ids: &[u32], out: Option<&Path>) -> Result<()> {
    if let Some(dir) = out {
        std::fs::create_dir_all(dir)?;
    }
    for &id in ids {
        match rift_cache::SoundEffect::load(cache, id) {
            Ok(sound) => {
                println!("sound {id}: {}", describe_sound(&sound));
                if let Some(dir) = out {
                    std::fs::write(dir.join(format!("{id}.wav")), sound.to_wav())?;
                }
            }
            Err(err) => println!("sound {id}: {err}"),
        }
    }
    Ok(())
}

fn sounds(cache: &Cache, sequences: &[u32], out: Option<&Path>) -> Result<()> {
    if let Some(dir) = out {
        std::fs::create_dir_all(dir)?;
    }
    if sequences.is_empty() {
        let count = cache.group_count(rift_cache::sound::SOUND_INDEX);
        let mut ok = 0;
        let mut failed = Vec::new();
        for id in 0..count {
            let Ok(data) = cache.file(rift_cache::sound::SOUND_INDEX, id, 0) else {
                continue;
            };
            match rift_cache::SoundEffect::decode(id, &data) {
                Ok(sound) => {
                    let _ = sound.render();
                    ok += 1;
                }
                Err(err) => failed.push((id, err.to_string())),
            }
        }
        println!("decoded {ok} sounds, {} failed", failed.len());
        let mut with_sounds = 0;
        for (id, data) in cache.archive_files_with_ids(2, 12)? {
            if let Ok(seq) = rift_cache::Sequence::load(id, &data)
                && !seq.sounds.is_empty()
            {
                with_sounds += 1;
                let ids: Vec<u16> = seq.sounds.iter().map(|s| s.id).collect();
                println!("  sequence {id}: sounds {ids:?}");
            }
        }
        println!("{with_sounds} sequences play sounds");
        for (id, err) in failed.iter().take(10) {
            println!("  {id}: {err}");
        }
        return Ok(());
    }
    for &seq_id in sequences {
        let seq = rift_cache::Sequence::load(seq_id, &cache.file(2, 12, seq_id)?)?;
        if seq.sounds.is_empty() {
            println!("sequence {seq_id}: no sounds");
        }
        for frame_sound in &seq.sounds {
            let id = frame_sound.id as u32;
            match rift_cache::SoundEffect::load(cache, id) {
                Ok(sound) => {
                    println!(
                        "sequence {seq_id} frame {}: sound {id} {}",
                        frame_sound.frame,
                        describe_sound(&sound)
                    );
                    if let Some(dir) = out {
                        std::fs::write(dir.join(format!("{id}.wav")), sound.to_wav())?;
                    }
                }
                Err(err) => println!(
                    "sequence {seq_id} frame {}: sound {id}: {err}",
                    frame_sound.frame
                ),
            }
        }
    }
    Ok(())
}

fn sequence(cache: &Cache, id: u32) -> Result<()> {
    let data = cache.file(2, 12, id)?;
    let seq = rift_cache::Sequence::load(id, &data)?;
    println!(
        "sequence {id}: {} frames, step={} loops={} priority={} maya={} name={:?}",
        seq.frame_ids.len(),
        seq.frame_step,
        seq.max_loops,
        seq.priority,
        seq.anim_maya_id,
        seq.debug_name
    );
    println!("  lengths={:?}", seq.frame_lengths);
    for sound in &seq.sounds {
        println!(
            "  sound at frame {}: id={} loops={} range={} retain={} weight={}",
            sound.frame, sound.id, sound.loops, sound.range, sound.retain, sound.weight
        );
    }
    println!(
        "  frame_ids={:?}",
        &seq.frame_ids[..seq.frame_ids.len().min(12)]
    );

    for &frame_id in seq.frame_ids.iter().take(3) {
        let archive_id = frame_id >> 16;
        let file_id = frame_id & 0xffff;
        let frame_data = cache.file(0, archive_id, file_id)?;
        let framemap_id = u16::from_be_bytes([frame_data[0], frame_data[1]]) as u32;
        let fm_group = cache.group(1, framemap_id)?;
        let framemap = rift_cache::Framemap::load(framemap_id, &fm_group.data)?;
        let frame = rift_cache::Frame::load(file_id, &framemap, &frame_data)?;
        println!(
            "  frame {frame_id}: archive={archive_id} file={file_id} framemap={framemap_id} types={:?} entries={} first={:?}",
            framemap.types,
            frame.index_frame_ids.len(),
            frame.index_frame_ids.iter().take(8).collect::<Vec<_>>()
        );
    }
    Ok(())
}

fn spotanim(cache: &Cache, id: u32) -> Result<()> {
    let data = cache.config(rift_cache::spotanim::SPOTANIM_ARCHIVE, id)?;
    let def = rift_cache::SpotAnimDefinition::parse(id, &data)?;
    println!(
        "spotanim {id}: model={} sequence={} resize={}x{} rotation={} ambient={} contrast={} complete={}",
        def.model_id,
        def.animation_id,
        def.resize_x,
        def.resize_y,
        def.rotation,
        def.ambient,
        def.contrast,
        def.complete
    );
    if !def.recolors.is_empty() {
        println!("  recolors={:?}", def.recolors);
    }
    if !def.retextures.is_empty() {
        println!("  retextures={:?}", def.retextures);
    }
    if def.model_id > 0 {
        let group = cache.group(7, def.model_id)?;
        let model = Model::load(def.model_id, &group.data)?;
        let (min, max) = model.bounds();
        let translucent = model
            .face_alphas
            .as_ref()
            .map_or(0, |alphas| alphas.iter().filter(|&&a| a > 0).count());
        println!(
            "  model: {} verts, {} faces ({translucent} translucent), bounds {min:?}..{max:?}",
            model.vertex_count, model.face_count
        );
    }
    if def.animation_id >= 0 {
        let clip = rift_cache::AnimationClip::load(cache, def.animation_id as u32)?;
        println!(
            "  sequence: {} frames, {:.0} ms",
            clip.frames.len(),
            clip.loop_ms()
        );
    }
    Ok(())
}

/// Load a spot animation's model with its palette swaps applied, posed at
/// `t` (0..1) of its sequence.
fn spotanim_pose(
    cache: &Cache,
    def: &rift_cache::SpotAnimDefinition,
    t: f32,
) -> Result<(Model, Vec<[f32; 3]>)> {
    let group = cache.group(7, def.model_id)?;
    let mut model = Model::load(def.model_id, &group.data)?;
    if !def.recolors.is_empty() {
        let (find, replace): (Vec<i16>, Vec<i16>) = def
            .recolors
            .iter()
            .map(|&(f, r)| (f as i16, r as i16))
            .unzip();
        model.recolor(&find, &replace);
    }
    let positions = if def.animation_id >= 0 {
        match rift_cache::AnimationClip::load(cache, def.animation_id as u32) {
            Ok(clip) => model.animated_positions(&clip, clip.loop_ms() * t),
            Err(_) => model.animated_positions(&rift_cache::AnimationClip::default(), 0.0),
        }
    } else {
        model.animated_positions(&rift_cache::AnimationClip::default(), 0.0)
    };
    Ok((model, positions))
}

fn spotanims(cache: &Cache, from: u32, to: u32, sheet: Option<&Path>) -> Result<()> {
    let files = cache.archive_files_with_ids(2, rift_cache::spotanim::SPOTANIM_ARCHIVE)?;
    let total = files.len();
    let mut incomplete = 0usize;
    let mut tiles = Vec::new();
    for (id, data) in files {
        let def = match rift_cache::SpotAnimDefinition::parse(id, &data) {
            Ok(def) => def,
            Err(err) => {
                incomplete += 1;
                println!("spotanim {id}: error {err}");
                continue;
            }
        };
        if !def.complete {
            incomplete += 1;
        }
        if id < from || id > to {
            continue;
        }
        println!(
            "spotanim {id:5}: model={:5} seq={:5} resize={}x{} rot={} complete={}",
            def.model_id, def.animation_id, def.resize_x, def.resize_y, def.rotation, def.complete
        );
        if sheet.is_some() && def.model_id > 0 {
            if let Ok((model, positions)) = spotanim_pose(cache, &def, 0.5) {
                let tile = model_sheet::render_pose(&model, &positions, 96, [44, 44, 52, 255]);
                tiles.push((id.to_string(), tile));
            }
        }
    }
    println!("{total} spot animations, {incomplete} stopped at an unknown opcode");
    if let Some(path) = sheet {
        model_sheet::sheet(&tiles, 12, 96).save(path)?;
        println!("wrote {} tiles to {}", tiles.len(), path.display());
    }
    Ok(())
}

fn skeleton_anims(
    cache: &Cache,
    sequence_id: u32,
    model_id: Option<u32>,
    sheet: Option<&Path>,
    window: u32,
) -> Result<()> {
    let framemap_of = |frame_id: u32| -> Option<u16> {
        let data = cache.file(0, frame_id >> 16, frame_id & 0xffff).ok()?;
        (data.len() >= 2).then(|| u16::from_be_bytes([data[0], data[1]]))
    };
    let reference = rift_cache::Sequence::load(sequence_id, &cache.file(2, 12, sequence_id)?)?;
    let skeleton = reference
        .frame_ids
        .first()
        .and_then(|&frame| framemap_of(frame))
        .context("reference sequence has no frames")?;
    println!("sequence {sequence_id} animates skeleton {skeleton}");

    // One lookup per frame archive: every frame in an archive shares a
    // skeleton, so the first file answers for all of them.
    let mut archive_skeleton: std::collections::HashMap<u32, Option<u16>> =
        std::collections::HashMap::new();
    let mut matches = Vec::new();
    for (id, data) in cache.archive_files_with_ids(2, 12)? {
        if id.abs_diff(sequence_id) > window {
            continue;
        }
        let Ok(seq) = rift_cache::Sequence::load(id, &data) else {
            continue;
        };
        let Some(&first) = seq.frame_ids.first() else {
            continue;
        };
        let found = *archive_skeleton
            .entry(first >> 16)
            .or_insert_with(|| framemap_of(first));
        if found == Some(skeleton) {
            let ms: f32 = seq
                .frame_lengths
                .iter()
                .map(|&l| l.max(1) as f32 * 20.0)
                .sum();
            println!(
                "  sequence {id:5}: {:3} frames {:6.0} ms loops={} priority={} {}",
                seq.frame_ids.len(),
                ms,
                seq.max_loops,
                seq.priority,
                seq.debug_name
            );
            matches.push(id);
        }
    }
    println!("{} sequence(s) share the skeleton", matches.len());

    if let (Some(model_id), Some(path)) = (model_id, sheet) {
        let group = cache.group(7, model_id)?;
        let model = Model::load(model_id, &group.data)?;
        let mut tiles = Vec::new();
        for &id in &matches {
            let Ok(clip) = rift_cache::AnimationClip::load(cache, id) else {
                continue;
            };
            for t in [0.2f32, 0.45, 0.7] {
                let positions = model.animated_positions(&clip, clip.loop_ms() * t);
                let tile = model_sheet::render_pose(&model, &positions, 96, [44, 44, 52, 255]);
                tiles.push((id.to_string(), tile));
            }
        }
        model_sheet::sheet(&tiles, 12, 96).save(path)?;
        println!("wrote {} tiles to {}", tiles.len(), path.display());
    }
    Ok(())
}

fn skeleton_report(cache: &Cache, which: &str, sheet: Option<&Path>) -> Result<()> {
    const POSES: usize = 6;
    let mut tiles = Vec::new();
    use rift_cache::skeleton::{Binding, assemble, identify_limbs};
    let mut files: Vec<PathBuf> = std::fs::read_dir("data/champions")?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .filter(|p| which == "all" || p.file_stem().is_some_and(|s| s == which))
        .collect();
    files.sort();
    for path in files {
        let json: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path)?)?;
        let name = json["name"].as_str().unwrap_or("?");
        let ids: Vec<u32> = std::iter::once(json["model"].as_u64().unwrap_or(0) as u32)
            .chain(
                json["attachments"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|v| v.as_u64().map(|v| v as u32)),
            )
            .collect();
        let parts: Vec<Model> = ids
            .iter()
            .map(|&id| Model::load(id, &cache.group(7, id)?.data))
            .collect::<std::result::Result<_, _>>()?;
        let clips: Vec<rift_cache::AnimationClip> = ["idle", "walk", "attack_anim"]
            .iter()
            .filter_map(|key| json[*key].as_u64())
            .filter_map(|seq| rift_cache::AnimationClip::load(cache, seq as u32).ok())
            .collect();
        let refs: Vec<&rift_cache::AnimationClip> = clips.iter().collect();
        let (model, reports) = assemble(&parts, &refs);
        println!("== {name} ({} parts, {} clips)", parts.len(), clips.len());
        for r in &reports {
            let how = match r.binding {
                Binding::Base => "base  ",
                Binding::Native => "NATIVE",
                Binding::Follow => "FOLLOW",
                Binding::Rigid => "RIGID ",
            };
            if r.binding == Binding::Base {
                println!(
                    "  part {} model {:>6}: {how} {} verts, {} labelled",
                    r.index, r.model_id, r.vertices, r.labelled
                );
            } else {
                println!(
                    "  part {} model {:>6}: {how} {} verts, {} labelled, {} touching | drift static {:.3} native {:.3} follow {:.3} rigid {:.3} | stretch native {:.3} follow {:.3} | tear {:.3}",
                    r.index,
                    r.model_id,
                    r.vertices,
                    r.labelled,
                    r.contacts,
                    r.static_drift,
                    r.native_drift,
                    r.follow_drift,
                    r.rigid_drift,
                    r.native_stretch,
                    r.follow_stretch,
                    r.native_tear
                );
            }
        }
        if sheet.is_some() && parts.len() > 1 {
            let old = Model::merge(&parts);
            // Walk (second clip) when there is one, then the attack.
            let posed: Vec<&rift_cache::AnimationClip> = clips.iter().skip(1).collect();
            for (label, m) in [("old", &old), ("new", &model)] {
                for clip in &posed {
                    for i in 0..POSES / posed.len().max(1) {
                        let t =
                            clip.loop_ms() * (i as f32 + 0.5) / (POSES / posed.len().max(1)) as f32;
                        let positions = m.animated_positions(clip, t);
                        let tile = model_sheet::render_pose(m, &positions, 160, [44, 44, 52, 255]);
                        let short: String = name.chars().take(10).collect();
                        tiles.push((format!("{label} {short}"), tile));
                    }
                }
            }
        }
        let limbs = identify_limbs(&model);
        let named: Vec<String> = limbs
            .iter()
            .enumerate()
            .filter(|(_, l)| **l != rift_cache::skeleton::Limb::Empty)
            .map(|(i, l)| format!("{i}:{}", l.label()))
            .collect();
        println!("  limbs {}", named.join(" "));
    }
    if let Some(path) = sheet {
        model_sheet::sheet(&tiles, POSES, 160).save(path)?;
        println!("wrote {} tiles to {}", tiles.len(), path.display());
    }
    Ok(())
}

fn animate(cache: &Cache, model_id: u32, sequence_id: u32) -> Result<()> {
    let group = cache.group(7, model_id)?;
    let model = Model::load(model_id, &group.data)?;
    let clip = rift_cache::AnimationClip::load(cache, sequence_id)?;
    println!(
        "model {model_id} x sequence {sequence_id}: {} frames, lengths {:?}",
        clip.frames.len(),
        clip.sequence.frame_lengths
    );
    for time in [0.0, 100.0, 200.0, 400.0, 600.0, 800.0, 1000.0] {
        let positions = model.animated_positions(&clip, time);
        let mut min = [f32::MAX; 3];
        let mut max = [f32::MIN; 3];
        for p in &positions {
            for i in 0..3 {
                min[i] = min[i].min(p[i]);
                max[i] = max[i].max(p[i]);
            }
        }
        println!("  t={time:>6.0}ms min={min:?} max={max:?}");
    }
    Ok(())
}

fn model(cache: &Cache, id: u32) -> Result<()> {
    let group = cache.group(7, id)?;
    let model = Model::load(id, &group.data)?;
    let (min, max) = model.bounds();
    println!(
        "model {id}: {} verts, {} faces, {} texture faces, {} bytes",
        model.vertex_count,
        model.face_count,
        model.num_texture_faces,
        group.data.len()
    );
    println!("  bounds min={min:?} max={max:?}");
    println!(
        "  groups={:?} textures={}",
        model.vertex_groups.as_ref().map(Vec::len),
        model.face_textures.is_some()
    );
    Ok(())
}

fn export_obj(cache: &Cache, id: u32, out: &PathBuf) -> Result<()> {
    let group = cache.group(7, id)?;
    let model = Model::load(id, &group.data)?;
    let mut f = std::fs::File::create(out)?;

    writeln!(f, "# rift model {id}")?;
    for i in 0..model.vertex_count {
        writeln!(
            f,
            "v {} {} {}",
            model.vertex_x[i], model.vertex_y[i], model.vertex_z[i]
        )?;
    }
    for face in &model.faces {
        writeln!(f, "f {} {} {}", face[0] + 1, face[1] + 1, face[2] + 1)?;
    }
    println!(
        "wrote {} ({} verts, {} faces)",
        out.display(),
        model.vertex_count,
        model.face_count
    );
    Ok(())
}

fn save_all(out: &std::path::Path, files: Vec<(String, picture::Picture)>) -> Result<usize> {
    std::fs::create_dir_all(out)?;
    let count = files.len();
    for (path, picture) in files {
        let path = out.join(path);
        picture
            .save(&path)
            .with_context(|| format!("writing {}", path.display()))?;
    }
    Ok(count)
}

fn dump_sprites(
    cache: &Cache,
    out: &std::path::Path,
    from: Option<u32>,
    to: Option<u32>,
    overview_only: bool,
) -> Result<()> {
    let reference = cache
        .reference(SPRITE_INDEX)
        .context("reading the list of sprites")?;
    let wanted: Vec<(u32, i32)> = reference
        .archives
        .iter()
        .filter(|a| from.is_none_or(|from| a.id >= from) && to.is_none_or(|to| a.id <= to))
        .map(|a| (a.id, a.name_hash))
        .collect();
    if wanted.is_empty() {
        bail!("no sprites in that range");
    }

    let mut report = interface_dump::sprite_report_heading();
    let mut failed = String::new();
    let mut thumbs: Vec<interface_dump::Thumb> = Vec::new();
    let (mut read, mut frames, mut files, mut unread) = (0usize, 0usize, 0usize, 0usize);
    // one overview sheet for every hundred ids
    let flush = |thumbs: &mut Vec<interface_dump::Thumb>| -> Result<()> {
        if let (Some(first), Some(last)) = (thumbs.first(), thumbs.last()) {
            let path = out.join(format!("overview/{:05}-{:05}.png", first.id, last.id));
            interface_dump::overview(thumbs)
                .save(&path)
                .with_context(|| format!("writing {}", path.display()))?;
        }
        thumbs.clear();
        Ok(())
    };
    for (id, hash) in wanted {
        if thumbs
            .first()
            .is_some_and(|first| first.id / 100 != id / 100)
        {
            flush(&mut thumbs)?;
        }
        let sheet = cache
            .group(SPRITE_INDEX, id)
            .map_err(anyhow::Error::from)
            .and_then(|group| SpriteSheet::decode(id, &group.data).map_err(anyhow::Error::from));
        let sheet = match sheet {
            Ok(sheet) => sheet,
            Err(error) => {
                failed.push_str(&format!("{id}\t{error}\n"));
                continue;
            }
        };
        read += 1;
        frames += sheet.frames.len();
        if sheet.unread != 0 {
            unread += 1;
        }
        let dump = interface_dump::dump_sprite(&sheet, &interface_dump::name_of(hash));
        report.push_str(&dump.report);
        if !overview_only {
            files += save_all(out, dump.files)?;
        }
        thumbs.push(interface_dump::Thumb::of(&sheet));
    }
    flush(&mut thumbs)?;

    std::fs::create_dir_all(out)?;
    std::fs::write(out.join("sprites.tsv"), report)?;
    std::fs::write(out.join("sprites_failed.tsv"), &failed)?;
    println!(
        "sprites: {read} read, {frames} frames, {files} pictures written to {}",
        out.display()
    );
    println!(
        "sprites: {} could not be read (sprites_failed.tsv); {unread} left bytes unread, which a right reading does not",
        failed.lines().count()
    );
    Ok(())
}

fn dump_fonts(cache: &Cache, out: &std::path::Path) -> Result<()> {
    let reference = cache
        .reference(FONT_INDEX)
        .context("reading the list of fonts")?;
    let mut report = interface_dump::font_report_heading();
    let mut summary =
        String::from("font\tname\tbytes of measurements\tletters\tbaseline\tabove\tbelow\tnote\n");
    let (mut read, mut failed) = (0usize, 0usize);
    for archive in &reference.archives {
        let id = archive.id;
        let name = interface_dump::name_of(archive.name_hash);
        let font = (|| -> Result<(Font, SpriteSheet, usize)> {
            let measurements = cache.group(FONT_INDEX, id)?.data;
            let letters = SpriteSheet::decode(id, &cache.group(SPRITE_INDEX, id)?.data)?;
            let font = Font::decode(id, &measurements, &letters)?;
            Ok((font, letters, measurements.len()))
        })();
        match font {
            Ok((font, letters, bytes)) => {
                read += 1;
                summary.push_str(&format!(
                    "{id}\t{name}\t{bytes}\t{}\t{}\t{}\t{}\t{}\n",
                    letters.frames.len(),
                    font.ascent,
                    font.max_ascent,
                    font.max_descent,
                    font.note.as_deref().unwrap_or("")
                ));
                let dump = interface_dump::dump_font(&font, &letters, &name);
                report.push_str(&dump.report);
                save_all(out, dump.files)?;
            }
            Err(error) => {
                failed += 1;
                summary.push_str(&format!(
                    "{id}\t{name}\t-\t-\t-\t-\t-\tnot read: {error:#}\n"
                ));
            }
        }
    }
    std::fs::create_dir_all(out)?;
    std::fs::write(out.join("fonts.tsv"), &summary)?;
    std::fs::write(out.join("font_glyphs.tsv"), report)?;
    print!("{summary}");
    println!(
        "fonts: {read} read, {failed} not read; pictures in {}",
        out.join("fonts").display()
    );
    Ok(())
}

fn sprite(cache: &Cache, id: u32, out: &PathBuf) -> Result<()> {
    let group = cache.group(8, id)?;
    let sprite = SpriteDefinition::load(id, &group.data)?;
    let Some(frame) = sprite.frames.first() else {
        bail!("sprite {id} has no frames");
    };
    let image = image::RgbaImage::from_raw(frame.width, frame.height, frame.pixels.clone())
        .context("sprite frame buffer does not match its dimensions")?;
    image.save(out)?;
    println!(
        "sprite {id}: {} frame(s), first {}x{} -> {}",
        sprite.frames.len(),
        frame.width,
        frame.height,
        out.display()
    );
    Ok(())
}

fn dump_textures(cache: &Cache, out: &Path, dump_sprites: bool) -> Result<()> {
    let texture_dir = out.join("textures");
    std::fs::create_dir_all(&texture_dir)?;
    let files = cache.archive_files_with_ids(9, 0)?;
    let mut csv = String::from(
        "texture,sprite,animation_direction,animation_speed,width,height,frames,path\n",
    );
    let mut written = 0usize;
    for (id, data) in &files {
        let definition = match TextureDefinition::load(*id, data) {
            Ok(definition) => definition,
            Err(err) => {
                tracing::warn!("texture {id}: {err:#}");
                continue;
            }
        };
        let group = match cache.group(8, definition.sprite_id) {
            Ok(group) => group,
            Err(err) => {
                tracing::warn!("texture {id}: sprite {}: {err:#}", definition.sprite_id);
                continue;
            }
        };
        let sprite = match SpriteDefinition::load(definition.sprite_id, &group.data) {
            Ok(sprite) => sprite,
            Err(err) => {
                tracing::warn!("texture {id}: {err:#}");
                continue;
            }
        };
        let Some(frame) = sprite.frames.first() else {
            continue;
        };
        let path = texture_dir.join(format!("texture_{id:04}.png"));
        let image = image::RgbaImage::from_raw(frame.width, frame.height, frame.pixels.clone())
            .context("texture frame buffer does not match its dimensions")?;
        image.save(&path)?;
        csv.push_str(&format!(
            "{id},{},{},{},{},{},{},{}\n",
            definition.sprite_id,
            definition.animation_direction,
            definition.animation_speed,
            frame.width,
            frame.height,
            sprite.frames.len(),
            path.display()
        ));
        written += 1;
    }
    std::fs::write(out.join("textures.csv"), csv)?;
    println!(
        "textures: {written}/{} definitions -> {}",
        files.len(),
        texture_dir.display()
    );

    if dump_sprites {
        let sprite_dir = out.join("sprites");
        std::fs::create_dir_all(&sprite_dir)?;
        let mut sprite_csv = String::from("sprite,frames,width,height,path\n");
        let mut sprite_count = 0usize;
        for id in 0..cache.group_count(8) {
            let Ok(group) = cache.group(8, id) else {
                continue;
            };
            let Ok(sprite) = SpriteDefinition::load(id, &group.data) else {
                continue;
            };
            let Some(frame) = sprite.frames.first() else {
                continue;
            };
            if frame.width == 0 || frame.height == 0 {
                continue;
            }
            let path = sprite_dir.join(format!("sprite_{id:05}.png"));
            let Some(image) =
                image::RgbaImage::from_raw(frame.width, frame.height, frame.pixels.clone())
            else {
                continue;
            };
            image.save(&path)?;
            sprite_csv.push_str(&format!(
                "{id},{},{},{},{}\n",
                sprite.frames.len(),
                frame.width,
                frame.height,
                path.display()
            ));
            sprite_count += 1;
        }
        std::fs::write(out.join("sprites.csv"), sprite_csv)?;
        println!(
            "sprites: {sprite_count}/{} groups -> {}",
            cache.group_count(8),
            sprite_dir.display()
        );
    }
    Ok(())
}

fn texture(cache: &Cache, id: u32, out: &PathBuf) -> Result<()> {
    let files = cache.archive_files_with_ids(9, 0)?;
    let Some((_, data)) = files.iter().find(|(file_id, _)| *file_id == id) else {
        bail!("texture {id} not found in index 9 archive 0");
    };
    let definition = TextureDefinition::load(id, data)?;
    let group = cache.group(8, definition.sprite_id)?;
    let sprite = SpriteDefinition::load(definition.sprite_id, &group.data)?;
    let Some(frame) = sprite.frames.first() else {
        bail!(
            "texture {id}: sprite {} has no frames",
            definition.sprite_id
        );
    };
    let image = image::RgbaImage::from_raw(frame.width, frame.height, frame.pixels.clone())
        .context("texture frame buffer does not match its dimensions")?;
    image.save(out)?;
    println!(
        "texture {id}: sprite {} {}x{} -> {}",
        definition.sprite_id,
        frame.width,
        frame.height,
        out.display()
    );
    Ok(())
}

fn terrain_obj(cache: &Cache, region_x: u32, region_y: u32, out: &PathBuf) -> Result<()> {
    let (terrain_group, _) = map_groups(cache, region_x, region_y)?;
    let group = cache.group(5, terrain_group)?;
    let terrain = Terrain::load(region_x, region_y, &group.data)?;
    let mut f = std::fs::File::create(out)?;

    writeln!(f, "# rift terrain {region_x},{region_y}")?;
    for y in 0..64 {
        for x in 0..64 {
            let wx = x as i32 * 128;
            let wz = y as i32 * 128;
            let h = terrain.height(0, x, y) as i32 * 8;
            writeln!(f, "v {wx} {h} {wz}")?;
        }
    }
    for y in 0..63 {
        for x in 0..63 {
            let a = y * 64 + x + 1;
            let b = a + 1;
            let c = a + 64;
            let d = c + 1;
            writeln!(f, "f {a} {c} {b}")?;
            writeln!(f, "f {b} {c} {d}")?;
        }
    }
    println!("wrote {} ({} tiles)", out.display(), terrain.tiles().len());
    Ok(())
}
