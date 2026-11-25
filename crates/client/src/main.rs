mod animated;
mod audio;
mod balance;
mod champion;
mod devtools;
mod font;
mod fx;
mod gpu_timing;
mod grass_textures;
mod hero_select;
mod hud;
mod item_icons;
mod items;
mod kit;
mod lighting;
mod menu;
mod minimap;
mod model_textures;
mod motion;
mod portraits;
mod renderer;
mod scene;
mod spotlight;
mod stone;
mod terrain_textures;
mod theme_file;
mod ui_assets;
mod ui_live;
mod ui_pass;
mod wield;

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use clap::Parser;
use glam::Vec3;
use rift_cache::Cache;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

use crate::animated::{AnimatedScene, BuyOutcome, FloatStyle, UnitKind};
use crate::champion::ChampionConfig;
use crate::grass_textures::GrassTextures;
use crate::hero_select::HeroSelect;
use crate::hud::{HeroMarker, HoverInfo, Hud, PlayerHud, minimap_world_at};
use crate::items::{
    ItemShop, delivery_button_at, point_in_rect, shop_button_at, shop_buy_at, shop_card_at,
    shop_card_rect, shop_category_at, shop_close_at, shop_sell_at, shop_window_rect,
};
use crate::renderer::{Camera, Renderer};
use crate::scene::{CampBuff, HeightGrid, SceneData, WorldCollision};
use crate::terrain_textures::TerrainTextures;

#[derive(Parser, Debug)]
#[command(name = "rift-client", about = "Rift native client (M1 vertical slice)")]
struct Cli {
    #[arg(long, default_value = ".cache/osrs-235/cache")]
    cache: PathBuf,
    #[arg(long, default_value_t = 44)]
    region_x: u32,
    #[arg(long, default_value_t = 47)]
    region_y: u32,
    /// Radius in 64-tile regions (2 = 5x5 block, required by the full map).
    #[arg(long, default_value_t = 2)]
    radius: u32,
    /// Initial camera distance in world units.
    #[arg(long, default_value_t = 3200.0)]
    distance: f32,
    /// Initial window size in pixels, e.g. 1280x720 or 1920x1080.
    #[arg(long, default_value = "1280x720")]
    window_size: String,
    /// Scene render scale: `auto` targets 1.4M scene pixels, or a factor
    /// from 0.5 to 1.0. Post and HUD stay at native window resolution.
    #[arg(long, default_value = "auto")]
    render_scale: String,
    /// Static model and water draw distance in world units.
    #[arg(long, default_value_t = 16_000.0)]
    draw_distance: f32,
    /// Optional initial camera pitch in radians (dev screenshots).
    #[arg(long)]
    pitch: Option<f32>,
    /// Map design data.
    #[arg(long, default_value = "data/maps/karamja_rift/map.json")]
    map: PathBuf,
    /// Champion config name (monkey, jad, kree). Skips the hero-select screen.
    #[arg(long)]
    champion: Option<String>,
    /// Override King Awowogei's walk torso side movement (0..1) for tuning.
    #[arg(long)]
    walk_sway: Option<f32>,
    /// Override King Awowogei's walk arm movement (0..1) for tuning.
    #[arg(long)]
    walk_arm: Option<f32>,
    /// Start directly in the match with the default champion.
    #[arg(long)]
    skip_hero_select: bool,
    /// Dev: skip the main menu and open hero select in this mode (`bot`,
    /// `public` or `private`), for example for screenshots. With
    /// `--champion` that hero starts highlighted.
    #[arg(long)]
    start_select: Option<String>,
    /// Call this lane on hero select (`top`, `jungle`, `mid`, `carry` or
    /// `support`). A note to your team, not a rule: you can change it on the
    /// screen, and nothing in the match holds you to it.
    #[arg(long)]
    lane: Option<String>,
    /// Dev: lock in as soon as hero select opens, for screenshots of the
    /// "All heroes locked in" countdown (use with `--start-select`).
    #[arg(long)]
    lock_in: bool,
    /// Dev: start with the cursor at this window pixel (x y), for
    /// screenshots of hover states such as tooltips.
    #[arg(long, num_args = 2, value_names = ["X", "Y"])]
    cursor: Option<Vec<f64>>,
    /// Demo: walk to this tile coordinate after one second (x y).
    #[arg(long, num_args = 2, value_names = ["X", "Y"])]
    demo_target: Option<Vec<f32>>,
    /// Debug: spawn the hero at this tile coordinate (x y).
    #[arg(long, num_args = 2, value_names = ["X", "Y"])]
    spawn: Option<Vec<f32>>,
    /// Demo: auto-target a dummy and fire Q to verify combat.
    #[arg(long)]
    demo_combat: bool,
    /// Demo: loop through King Awowogei's idle, walk, turn-around, stop and
    /// attack animations with a close camera.
    #[arg(long)]
    demo_anim: bool,
    /// Demo: spawn a sparring enemy champion beside the hero.
    #[arg(long)]
    demo_duel: bool,
    /// Fill both teams with bot heroes at boot (four allies, five enemies),
    /// as a match from hero select does.
    #[arg(long)]
    bots: bool,
    /// Demo: King Awowogei's kit against a rival champion: Bananarang,
    /// a peel thrown under the rival, Greegree, then Vine Swing.
    #[arg(long)]
    demo_awowogei: bool,
    /// Play no sound (also off for `--shots` and `--benchmark`).
    #[arg(long)]
    mute: bool,
    /// Demo: pop floating damage numbers, a kill shout and a minimap ping.
    #[arg(long)]
    demo_fx: bool,
    /// Demo: pin one OSRS hitmark of every kind over the hero.
    #[arg(long)]
    demo_hitsplats: bool,
    /// Demo: teleport to a jungle camp, attack it, and verify the buff.
    #[arg(long)]
    demo_camp: bool,
    /// Demo: verify watchtower aggro, lock range and target reset.
    #[arg(long)]
    demo_watchtower: bool,
    /// Demo: buy away from base, call the guard and verify delivery.
    #[arg(long)]
    demo_delivery: bool,
    /// Demo: cast W/E/R and print the resulting player stats.
    #[arg(long)]
    demo_kit: bool,
    /// Demo: stand by a jungle camp and cast Q, W, E and R from the
    /// champion's kit, logging what each did.
    #[arg(long)]
    demo_abilities: bool,
    /// Demo: play these cache spot animations (comma separated ids) in a
    /// labelled grid beside the hero, over and over, for checking spell
    /// graphics by eye.
    #[arg(long, value_delimiter = ',')]
    demo_gfx: Vec<u32>,
    /// Demo: stand beside a jungle camp and basic-attack it, to check
    /// attack animations, reach and projectiles.
    #[arg(long)]
    demo_attack: bool,
    /// Demo: level the hero to 7, spend four skill points (Q twice, W and R
    /// once), leave two unspent so the slots glow, and hover this slot (0-3)
    /// so its ability tooltip shows.
    #[arg(long)]
    demo_ranks: Option<usize>,
    /// Write the paper balance report (Markdown) to this path and exit.
    #[arg(long)]
    balance_report: Option<PathBuf>,
    /// Dev: show the held ability indicator for this slot (0-3) aimed at
    /// the nearest jungle camp, as if right mouse were held.
    #[arg(long)]
    hold_indicator: Option<usize>,
    /// Demo: grant gold, open the shop, and buy one of every item.
    #[arg(long)]
    demo_shop: bool,
    /// Demo: damage the player and verify fountain healing plus passive gold.
    #[arg(long)]
    demo_base: bool,
    /// Demo: destroy an inhibitor and verify super minions plus respawn.
    #[arg(long)]
    demo_inhibitor: bool,
    /// Graphics preset: 117hd or classic.
    #[arg(long, default_value = "117hd")]
    graphics: String,
    /// Debug view: lit, albedo, normal, lighting, fog, lights, shadow, ao.
    #[arg(long, default_value = "lit")]
    debug_view: String,
    /// Shadows: off, blob, or map.
    #[arg(long, default_value = "map")]
    shadows: String,
    /// Present mode: fifo (vsync), immediate (uncapped), or mailbox.
    #[arg(long, default_value = "fifo")]
    present_mode: String,
    /// Scene MSAA: 1 (off) or 4 (default).
    #[arg(long, default_value_t = 4)]
    msaa: u32,
    /// Static-mesh culling chunk size in tiles.
    #[arg(long, default_value_t = 24)]
    chunk_tiles: u32,
    /// Run a fixed-duration frame-time benchmark, then exit (seconds).
    #[arg(long)]
    benchmark: Option<f32>,
    /// Measure per-pass GPU time with timestamp queries when the adapter
    /// supports them. The benchmark enables this automatically.
    #[arg(long)]
    gpu_timing: bool,
    /// Dev: write a PNG of the presented frame, then exit.
    #[arg(long)]
    screenshot: Option<PathBuf>,
    /// Dev: frames to draw before `--screenshot` captures (default 90).
    /// Also the warm-up per shot in a `--shots` run.
    #[arg(long, default_value_t = 90)]
    screenshot_after: u32,
    /// Dev: batch screenshots. A text file of camera presets, one per line:
    /// `name tile_x tile_y [pitch=R] [yaw=R] [distance=U] [view=NAME]`.
    /// The client loads once, captures every shot into `--shot-dir`, then
    /// exits. See docs/DEV_TOOLS.md.
    #[arg(long)]
    shots: Option<PathBuf>,
    /// Dev: output directory for `--shots` (gitignored by default).
    #[arg(long, default_value = ".cache/shots")]
    shot_dir: PathBuf,
    /// Dev: watch crates/client/src/scene.wgsl and recompile it on save.
    /// F5 reloads it on demand with or without this flag.
    #[arg(long)]
    hot_shaders: bool,
    /// Simulation speed multiplier for verification.
    #[arg(long, default_value_t = 1.0)]
    timescale: f32,
    /// Draw the interface renderer's test screen over the game.
    #[arg(long)]
    ui_test: bool,
    /// Draw the stage 3 theme sheet over the game: every role by name, and
    /// the pieces together.
    #[arg(long)]
    ui_theme_sheet: bool,
    /// Draw the assembled stage 3 interface over the game.
    #[arg(long)]
    ui_layout: bool,
    /// Draw the assembled interface with the live score strip.
    #[arg(long)]
    ui_live: bool,
    /// Force the interface's scale (1 to 4) instead of taking it from the
    /// window's size.
    #[arg(long)]
    ui_scale: Option<u32>,
    /// The theme: which images, fonts and colours the interface is made of.
    #[arg(long, default_value = "data/ui/theme.json")]
    ui_theme: PathBuf,
    /// Show the engineer stat sheet and visibility counters in the HUD.
    #[arg(long)]
    debug_hud: bool,
    /// Start in spectator mode, cycling cameras over both teams' champions.
    #[arg(long)]
    spectate: bool,
    /// Champion index to start spectating (dev verification).
    #[arg(long)]
    spectate_index: Option<usize>,
    /// Dev: kill the player after one second to verify the respawn overlay.
    #[arg(long)]
    demo_death: bool,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();
    if let Some(path) = cli.balance_report.as_ref() {
        let champions = ChampionConfig::load_all()?;
        std::fs::write(path, balance::report(&champions))
            .with_context(|| format!("writing {}", path.display()))?;
        println!("balance report: {}", path.display());
        return Ok(());
    }
    let window_size = parse_window_size(&cli.window_size)
        .with_context(|| format!("window size {:?} (expected WIDTHxHEIGHT)", cli.window_size))?;
    let render_scale = resolve_render_scale(&cli.render_scale, window_size).with_context(|| {
        format!(
            "render scale {:?} (expected auto or 0.5..1.0)",
            cli.render_scale
        )
    })?;
    let render_scale_auto = cli.render_scale.eq_ignore_ascii_case("auto");
    let startup = Instant::now();
    let shot_queue: VecDeque<devtools::ShotSpec> = match cli.shots.as_ref() {
        Some(path) => {
            let text = std::fs::read_to_string(path)
                .with_context(|| format!("reading shot list {}", path.display()))?;
            devtools::parse_shot_list(&text)?.into()
        }
        None => VecDeque::new(),
    };
    let cache = Cache::open_with_keys(&cli.cache, None::<&PathBuf>)
        .with_context(|| format!("opening cache at {}", cli.cache.display()))?;
    tracing::info!(
        "startup: cache open {:.0} ms",
        startup.elapsed().as_secs_f32() * 1000.0
    );

    tracing::info!(
        "building {0}x{0} region block around {1},{2}",
        cli.radius * 2 + 1,
        cli.region_x,
        cli.region_y
    );
    let mut champions = ChampionConfig::load_all()?;
    // The animation demo shows King Awowogei unless a hero is named.
    let requested_champion = if cli.demo_anim {
        cli.champion.as_deref().or(Some("monkey"))
    } else {
        cli.champion.as_deref()
    };
    let selected_champion = match requested_champion {
        Some(name) => champions
            .iter()
            .position(|champion| {
                champion.id.eq_ignore_ascii_case(name) || champion.name.eq_ignore_ascii_case(name)
            })
            .with_context(|| format!("unknown champion '{name}'"))?,
        None => champions
            .iter()
            .position(|champion| champion.id == "monkey")
            .unwrap_or(0),
    };
    if let Some(value) = cli.walk_sway {
        champions[selected_champion].walk_sway = value.clamp(0.0, 1.0);
    }
    if let Some(value) = cli.walk_arm {
        champions[selected_champion].walk_arm = value.clamp(0.0, 1.0);
    }
    let champion = champions[selected_champion].clone();
    // The front end (main menu, then hero select) runs unless a champion or a
    // dev demo was asked for. `--ui-live` only picks the in-match interface.
    let hero_select_open = (cli.champion.is_none() || cli.start_select.is_some())
        && !cli.skip_hero_select
        && !has_dev_demo_flags(&cli);
    let start_select = cli.start_select.as_deref().map(|mode| match mode {
        "public" => menu::MatchMode::Public,
        "private" => menu::MatchMode::Private,
        _ => menu::MatchMode::Bot,
    });
    let mut shop = ItemShop::load()?;
    let key = shop.recommend_for(&champion.id, &champion.role, &champion.class);
    tracing::info!(
        "shop: {} recommended items for {} (list {})",
        shop.recommended.len(),
        champion.name,
        key.as_deref().unwrap_or("none")
    );
    let phase = Instant::now();
    let scene = scene::build(&cache, cli.region_x, cli.region_y, cli.radius, &cli.map)?;
    tracing::info!(
        "startup: scene build {:.0} ms",
        phase.elapsed().as_secs_f32() * 1000.0
    );
    let phase = Instant::now();
    let animated = AnimatedScene::new(&cache, &scene, &champion)?;
    tracing::info!(
        "startup: animated scene {:.0} ms",
        phase.elapsed().as_secs_f32() * 1000.0
    );
    let height = scene.height.clone();
    let collision = scene.collision.clone();
    let mut animated = animated;
    if let Some(spawn) = cli.spawn.as_ref().filter(|v| v.len() == 2) {
        let x = (spawn[0] + 0.5) * 128.0;
        let z = (spawn[1] + 0.5) * 128.0;
        let point = Vec3::new(x, height.height_at(x, z), z);
        animated.teleport_player(point);
        tracing::info!("spawned hero at tile {},{}", spawn[0], spawn[1]);
    } else if cli.demo_anim {
        // A quiet stretch of the radiant lane; friendly base behind it and
        // enough room for the east/west walk and turn drills.
        let x = 2748.5 * 128.0;
        let z = 3010.5 * 128.0;
        let point = Vec3::new(x, height.height_at(x, z), z);
        animated.teleport_player(point);
        tracing::info!("animation demo: spawned hero at tile 2748,3010");
    }
    if cli.bots {
        let me = champions
            .iter()
            .position(|c| c.id == champion.id)
            .unwrap_or(0);
        let others: Vec<usize> = (0..champions.len()).filter(|&i| i != me).collect();
        let pick = |n: usize| others[n % others.len().max(1)];
        let blue: Vec<usize> = std::iter::once(me).chain((0..4).map(pick)).collect();
        let red: Vec<usize> = (4..9).map(pick).collect();
        // No pick phase on this path: the default calls (blue's bots leave
        // mid open, red covers every lane).
        let blue_lanes: Vec<Option<hero_select::Lane>> = std::iter::once(None)
            .chain(hero_select::bot_lanes(None).map(Some))
            .collect();
        let red_lanes = hero_select::RED_LANES.map(Some);
        let lineup = bot_lineup(&champions, &blue, &red, (&blue_lanes, &red_lanes));
        let builds = bot_builds(&shop, &lineup);
        let count = animated.spawn_bots(&cache, &lineup, &builds, &height);
        tracing::info!("bots: {count} bot heroes in the match");
    }
    if cli.demo_duel || cli.demo_awowogei {
        let spawn = animated.player_pos() + Vec3::new(3.0 * 128.0, 0.0, 0.0);
        let spawn = Vec3::new(spawn.x, height.height_at(spawn.x, spawn.z), spawn.z);
        match animated.spawn_enemy_champion(&cache, &champion, spawn) {
            Ok(index) => {
                tracing::info!("demo duel: enemy champion {index} at {spawn:?}");
            }
            Err(err) => tracing::warn!("demo duel: failed to spawn enemy champion: {err:#}"),
        }
    }
    tracing::info!(
        "scene ready: {} placements, {} animated units",
        scene.placements,
        animated.units.len()
    );

    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let demo_target = cli.demo_target.as_ref().and_then(|v| {
        if v.len() == 2 {
            Some((v[0], v[1]))
        } else {
            None
        }
    });

    let mut ui_atlas = ui_assets::UiAtlas::load_with_cache(&cache)?;
    // Cache-rendered inventory icons for every shop item, so the legacy HUD
    // never falls back to a coloured placeholder or an external PNG.
    for item in &shop.items {
        let Some(obj) = item.obj else {
            continue;
        };
        match crate::item_icons::bake(&cache, obj) {
            Ok(icon) => {
                let mut raw = Vec::with_capacity(icon.pixels.len() * 4);
                for pixel in &icon.pixels {
                    raw.extend_from_slice(pixel);
                }
                if let Some(image) =
                    image::RgbaImage::from_raw(icon.width as u32, icon.height as u32, raw)
                {
                    ui_atlas.add_item_icon(&item.id, &image);
                }
            }
            Err(err) => tracing::warn!("item icon {} ({obj}): {err:#}", item.id),
        }
    }
    // Every champion's cache ability icons, keyed like the assembled
    // interface's `icon.ability.*` roles. The legacy bar asks for the same
    // key, so hero select and champion swaps need no atlas rebuild.
    for champion in &champions {
        for (slot, sprite) in champion.ability_icons.iter().take(4).enumerate() {
            let key = crate::ui_live::ability_icon_role(&champion.name, slot);
            if let Err(err) = ui_atlas.add_cache_ui_sprite(&cache, *sprite, &key) {
                tracing::warn!(
                    "ability icon {} slot {slot} ({sprite}): {err:#}",
                    champion.name
                );
            }
        }
    }
    let mut live_portrait = None;
    if hero_select_open {
        for champion in &champions {
            match portraits::bake_portrait(
                &cache,
                champion.model,
                &champion.attachments,
                champion.idle,
            ) {
                Ok(image) => ui_atlas.add_hero_portrait(&champion.id, &image),
                Err(err) => tracing::warn!("hero portrait {}: {err:#}", champion.id),
            }
        }
    } else if cli.ui_live {
        // One cache-baked portrait fills the player's team frame without
        // forcing every champion through the CPU rasteriser.
        match portraits::bake_portrait(&cache, champion.model, &champion.attachments, champion.idle)
        {
            Ok(image) => {
                tracing::info!("team portrait: {} baked for the live frames", champion.id);
                live_portrait = Some(image);
            }
            Err(err) => tracing::warn!("team portrait {}: {err:#}", champion.id),
        }
    }
    // Bake a real terrain map for the minimap from the scene colours.
    // Coloured from the same cache textures the world shader paints with.
    let palette = minimap::Palette::from_cache(&cache, &scene)
        .inspect_err(|err| tracing::warn!("minimap palette: {err:#}; using floor colours"))
        .ok();
    let minimap_image = minimap::render(
        &scene,
        animated.minimap_bounds,
        minimap::MINIMAP_SIZE,
        palette.as_ref(),
    );
    ui_atlas.add_minimap(&minimap_image);
    let ui_atlas = Arc::new(ui_atlas);
    // The stage 3 interface's theme, only when its dev screen was asked for.
    let item_icons: Vec<(String, u32)> = if cli.ui_live {
        shop.items
            .iter()
            .filter_map(|item| item.obj.map(|obj| (item.id.clone(), obj)))
            .collect()
    } else {
        Vec::new()
    };
    // Every champion's ability icons, so the bar is right after hero select.
    let ability_icon_sprites: Vec<(String, u32)> = champions
        .iter()
        .flat_map(|champion| {
            champion
                .ability_icons
                .iter()
                .take(4)
                .enumerate()
                .map(|(slot, sprite)| {
                    (
                        crate::ui_live::ability_icon_role(&champion.name, slot),
                        *sprite,
                    )
                })
        })
        .collect();
    let live_theme_inputs = cli.ui_live.then(|| LiveThemeInputs {
        path: cli.ui_theme.clone(),
        item_icons: item_icons.clone(),
        minimap: minimap_image.clone(),
        sprites: ability_icon_sprites.clone(),
        portrait_for: live_portrait.as_ref().map(|_| champion.id.clone()),
    });
    let ui_theme = if cli.ui_test || cli.ui_theme_sheet || cli.ui_layout || cli.ui_live {
        let theme = theme_file::load_with_items_minimap_and_portrait(
            &cache,
            &cli.ui_theme,
            &item_icons,
            Some(&minimap_image),
            live_portrait.as_ref(),
            &ability_icon_sprites,
        )
        .with_context(|| format!("loading theme {}", cli.ui_theme.display()))?;
        tracing::info!(
            "interface theme {}: atlas {} x {}, fonts {:?}, panels {:?}, sprites {:?}, colours {:?}",
            cli.ui_theme.display(),
            theme.atlas.width,
            theme.atlas.height,
            theme.font_roles().collect::<Vec<_>>(),
            theme.panel_roles().collect::<Vec<_>>(),
            theme.sprite_roles().collect::<Vec<_>>(),
            theme.colour_roles().collect::<Vec<_>>()
        );
        Some(theme)
    } else {
        None
    };
    let initial_player = animated.player_index;
    let champion_indices: Vec<usize> = animated
        .units
        .iter()
        .enumerate()
        .filter(|(_, unit)| unit.is_champion)
        .map(|(index, _)| index)
        .collect();
    let initial_spectator = if let Some(slot) = cli.spectate_index {
        champion_indices.get(slot).copied()
    } else if cli.spectate {
        Some(initial_player)
    } else {
        None
    };
    let mut audio = audio::Audio::new(cli.mute || cli.shots.is_some() || cli.benchmark.is_some());
    audio.preload(&cache, animated.sound_ids());
    let mut app = App {
        audio,
        cache,
        scene: Some(scene),
        animated: Some(animated),
        height: Some(height),
        collision: Some(collision),
        hero_select: {
            let mut select = HeroSelect::new(champions, selected_champion, false);
            if let (true, Some(mode)) = (hero_select_open, start_select) {
                select.begin(mode);
            }
            match cli.lane.as_deref().map(hero_select::Lane::parse) {
                Some(Some(lane)) => select.set_lane(Some(lane)),
                Some(None) => tracing::warn!(
                    "--lane {:?}: expected top, jungle, mid, carry or support",
                    cli.lane.as_deref().unwrap_or_default()
                ),
                None => {}
            }
            if cli.lock_in && select.open {
                select.lock_in();
            }
            select
        },
        menu: menu::MainMenu::new(hero_select_open && start_select.is_none()),
        spotlight: spotlight::Spotlight::default(),
        ui_sounds: champion::load_default_sounds(),
        last_tick_second: None,
        live_theme_inputs,
        weapon_models: HashMap::new(),
        window: None,
        window_size,
        render_scale,
        render_scale_auto,
        draw_distance: cli.draw_distance,
        renderer: None,
        held: HashSet::new(),
        focused: false,
        follow: true,
        mouse_pos: match cli.cursor.as_deref() {
            Some(&[x, y]) => (x, y),
            _ => (640.0, 360.0),
        },
        lmb_down: false,
        rmb_down: false,
        lmb_press: None,
        initial_distance: cli.distance,
        initial_pitch: cli.pitch,
        debug_hud: cli.debug_hud,
        last_health: 0.0,
        damage_flash: 0.0,
        spectator: initial_spectator,
        selected: None,
        demo_target,
        demo_timer: 0.0,
        demo_combat: cli.demo_combat,
        demo_anim: cli.demo_anim,
        demo_anim_timer: 0.0,
        demo_anim_stage: 0,
        demo_anim_targeted: false,
        demo_awowogei: cli.demo_awowogei,
        demo_awowogei_timer: 0.0,
        demo_awowogei_stage: 0,
        demo_fx: cli.demo_fx,
        demo_hitsplats: cli.demo_hitsplats,
        demo_camp: cli.demo_camp,
        demo_watchtower: cli.demo_watchtower,
        demo_watchtower_timer: 0.0,
        demo_watchtower_stage: 0,
        demo_watchtower_target: None,
        demo_delivery: cli.demo_delivery,
        demo_delivery_timer: 0.0,
        demo_delivery_stage: 0,
        demo_kit: cli.demo_kit,
        demo_kit_timer: 0.0,
        demo_kit_stage: 0,
        demo_abilities: cli.demo_abilities,
        demo_abilities_timer: 0.0,
        demo_abilities_stage: 0,
        demo_gfx: cli.demo_gfx.clone(),
        demo_gfx_timer: 0.0,
        demo_gfx_loaded: false,
        demo_attack: cli.demo_attack,
        demo_attack_stage: 0,
        demo_ranks: cli.demo_ranks,
        demo_ranks_done: false,
        hold_indicator: cli.hold_indicator,
        shop,
        event_log: vec![
            "FOUNTAIN HEALS".to_string(),
            "PRESS B FOR SHOP".to_string(),
            "WAVE INBOUND".to_string(),
        ],
        last_level: 1,
        last_buff_label: None,
        last_inhib0: (3, 3),
        last_inhib1: (3, 3),
        demo_shop: cli.demo_shop,
        demo_shop_timer: 0.0,
        demo_shop_stage: 0,
        demo_death: cli.demo_death,
        demo_death_timer: 0.0,
        demo_base: cli.demo_base,
        demo_base_timer: 0.0,
        demo_base_stage: 0,
        demo_inhibitor: cli.demo_inhibitor,
        demo_inhibitor_timer: 0.0,
        demo_inhibitor_stage: 0,
        demo_camp_timer: 0.0,
        demo_camp_target: None,
        demo_camp_cast: false,
        demo_camp_done: false,
        timescale: cli.timescale,
        demo_combat_timer: 0.0,
        demo_combat_targeted: false,
        demo_combat_cast: false,
        demo_combat_target: None,
        hud: Hud::new(ui_atlas),
        ui_theme,
        ui_theme_sheet: cli.ui_theme_sheet,
        ui_layout: cli.ui_layout,
        ui_live: cli.ui_live,
        ui_scale: cli.ui_scale,
        ui_shown: (0, 0, 0),
        ui_placed: None,
        graphics_classic: cli.graphics.eq_ignore_ascii_case("classic"),
        debug_view: parse_debug_view(&cli.debug_view),
        shadow_mode: cli.shadows.clone(),
        present_mode: cli.present_mode.clone(),
        msaa: cli.msaa,
        chunk_tiles: cli.chunk_tiles,
        benchmark_remaining: cli.benchmark,
        benchmark_samples: Vec::new(),
        benchmark_sim_ms: 0.0,
        benchmark_mesh_ms: 0.0,
        benchmark_hud_ms: 0.0,
        benchmark_ui_ms: 0.0,
        benchmark_submit_ms: 0.0,
        benchmark_phase_frames: 0,
        benchmark_done: false,
        benchmark_profiling: false,
        gpu_timing: cli.gpu_timing || cli.benchmark.is_some(),
        screenshot_request: cli
            .screenshot
            .clone()
            .map(|path| (path, cli.screenshot_after)),
        shot_queue,
        shot_dir: cli.shot_dir.clone(),
        shot_warmup: cli.screenshot_after,
        exit_after_capture: false,
        hot_shaders: cli.hot_shaders,
        shader_poll_frames: 0,
        paused: false,
        startup,
        first_frame_logged: false,
        app_time: 0.0,
        lane_log_timer: 0.0,
        last_frame: Instant::now(),
        title_timer: 0.0,
        title_frames: 0,
    };
    event_loop.run_app(&mut app)?;
    Ok(())
}

/// The inputs of the `--ui-live` theme, kept to rebuild it with another
/// hero's team portrait.
struct LiveThemeInputs {
    path: PathBuf,
    item_icons: Vec<(String, u32)>,
    minimap: image::RgbaImage,
    sprites: Vec<(String, u32)>,
    /// The champion whose portrait the current theme carries.
    portrait_for: Option<String>,
}

struct App {
    /// Sound effect output (silent without a device or with `--mute`).
    audio: audio::Audio,
    cache: Cache,
    scene: Option<SceneData>,
    animated: Option<AnimatedScene>,
    height: Option<HeightGrid>,
    collision: Option<WorldCollision>,
    hero_select: HeroSelect,
    /// The main menu the game boots into.
    menu: menu::MainMenu,
    /// The animated hero in the middle of hero select.
    spotlight: spotlight::Spotlight,
    /// Shared interface sounds (`data/sounds.json`).
    ui_sounds: champion::SoundTable,
    /// Last whole second the pick timer ticked on (for the warning beeps).
    last_tick_second: Option<u32>,
    /// What the `--ui-live` theme was built from, so it can be rebuilt with
    /// the picked hero's portrait.
    live_theme_inputs: Option<LiveThemeInputs>,
    /// Wield models already loaded, by cache item id.
    weapon_models: HashMap<u32, Vec<rift_cache::Model>>,
    window: Option<Arc<Window>>,
    window_size: (u32, u32),
    render_scale: f32,
    render_scale_auto: bool,
    draw_distance: f32,
    renderer: Option<Renderer>,
    held: HashSet<KeyCode>,
    focused: bool,
    follow: bool,
    mouse_pos: (f64, f64),
    lmb_down: bool,
    lmb_press: Option<Instant>,
    /// Right mouse held: preview the last ability's range at the cursor.
    rmb_down: bool,
    initial_distance: f32,
    initial_pitch: Option<f32>,
    debug_hud: bool,
    /// Previous frame health, for the red damage vignette.
    last_health: f32,
    /// 0..1 red screen-edge flash after taking damage.
    damage_flash: f32,
    /// When set, the camera follows this champion index instead of the player.
    spectator: Option<usize>,
    /// The unit whose details the live target panel is showing.
    selected: Option<usize>,
    demo_target: Option<(f32, f32)>,
    demo_timer: f32,
    demo_combat: bool,
    demo_anim: bool,
    demo_anim_timer: f32,
    demo_anim_stage: u8,
    demo_anim_targeted: bool,
    demo_awowogei: bool,
    demo_awowogei_timer: f32,
    demo_awowogei_stage: u8,
    demo_fx: bool,
    demo_hitsplats: bool,
    demo_camp: bool,
    demo_watchtower: bool,
    demo_watchtower_timer: f32,
    demo_watchtower_stage: u8,
    demo_watchtower_target: Option<usize>,
    demo_delivery: bool,
    demo_delivery_timer: f32,
    demo_delivery_stage: u8,
    demo_kit: bool,
    demo_kit_timer: f32,
    demo_kit_stage: u8,
    demo_abilities: bool,
    demo_abilities_timer: f32,
    demo_abilities_stage: u8,
    demo_gfx: Vec<u32>,
    /// Seconds until the `--demo-gfx` grid plays again.
    demo_gfx_timer: f32,
    demo_gfx_loaded: bool,
    demo_attack: bool,
    demo_attack_stage: u8,
    demo_ranks: Option<usize>,
    demo_ranks_done: bool,
    hold_indicator: Option<usize>,
    shop: ItemShop,
    event_log: Vec<String>,
    last_level: u32,
    last_buff_label: Option<&'static str>,
    last_inhib0: (usize, usize),
    last_inhib1: (usize, usize),
    demo_shop: bool,
    demo_shop_timer: f32,
    demo_shop_stage: u8,
    demo_death: bool,
    demo_death_timer: f32,
    demo_base: bool,
    demo_base_timer: f32,
    demo_base_stage: u8,
    demo_inhibitor: bool,
    demo_inhibitor_timer: f32,
    demo_inhibitor_stage: u8,
    demo_camp_timer: f32,
    demo_camp_target: Option<usize>,
    demo_camp_cast: bool,
    demo_camp_done: bool,
    timescale: f32,
    demo_combat_timer: f32,
    demo_combat_targeted: bool,
    demo_combat_cast: bool,
    demo_combat_target: Option<usize>,
    hud: Hud,
    /// The stage 3 interface theme, when its dev screen was asked for.
    ui_theme: Option<rift_ui::Theme>,
    /// Whether the theme sheet was asked for instead of the test screen.
    ui_theme_sheet: bool,
    /// Whether the assembled interface was asked for.
    ui_layout: bool,
    /// Whether the assembled interface takes the live score strip.
    ui_live: bool,
    ui_scale: Option<u32>,
    /// The window size and scale the interface was last built for.
    ui_shown: (u32, u32, u32),
    /// The last placed live interface, so its controls can be hit-tested.
    ui_placed: Option<rift_ui::hud::Layout>,
    graphics_classic: bool,
    debug_view: u32,
    shadow_mode: String,
    present_mode: String,
    msaa: u32,
    chunk_tiles: u32,
    benchmark_remaining: Option<f32>,
    benchmark_samples: Vec<f32>,
    benchmark_sim_ms: f32,
    benchmark_mesh_ms: f32,
    benchmark_hud_ms: f32,
    benchmark_ui_ms: f32,
    benchmark_submit_ms: f32,
    benchmark_phase_frames: u32,
    benchmark_done: bool,
    /// Whether the GPU timing totals have been reset for this benchmark.
    benchmark_profiling: bool,
    gpu_timing: bool,
    screenshot_request: Option<(std::path::PathBuf, u32)>,
    /// Dev batch screenshots still to capture (`--shots`).
    shot_queue: VecDeque<devtools::ShotSpec>,
    shot_dir: PathBuf,
    shot_warmup: u32,
    /// `--screenshot` / `--shots`: exit once every capture is written.
    exit_after_capture: bool,
    hot_shaders: bool,
    shader_poll_frames: u32,
    /// F10: freeze the simulation while rendering continues.
    paused: bool,
    startup: Instant,
    first_frame_logged: bool,
    app_time: f32,
    lane_log_timer: f32,
    last_frame: Instant,
    title_timer: f32,
    title_frames: u32,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attributes = Window::default_attributes()
            .with_title("Rift - Karamja")
            .with_inner_size(winit::dpi::LogicalSize::new(
                self.window_size.0,
                self.window_size.1,
            ));
        let window = match event_loop.create_window(attributes) {
            Ok(window) => Arc::new(window),
            Err(err) => {
                tracing::error!("failed to create window: {err}");
                event_loop.exit();
                return;
            }
        };

        let scene = self.scene.take().expect("scene missing");
        let animated = self.animated.take().expect("animated scene missing");
        let camera = Camera::new(animated.player_pos() + Vec3::Y * 140.0);
        let gpu_phase = Instant::now();
        let model_textures = match crate::model_textures::ModelTextures::load(&self.cache) {
            Ok(textures) => textures,
            Err(err) => {
                tracing::error!("failed to load model textures: {err:#}");
                event_loop.exit();
                return;
            }
        };
        let terrain_textures = match TerrainTextures::load_with_fallback(&self.cache) {
            Ok(textures) => textures,
            Err(err) => {
                tracing::error!("failed to load ground textures: {err:#}");
                event_loop.exit();
                return;
            }
        };
        let grass_textures = match GrassTextures::load_with_fallback(&self.cache) {
            Ok(textures) => textures,
            Err(err) => {
                tracing::error!("failed to load grass textures: {err:#}");
                event_loop.exit();
                return;
            }
        };
        let mut renderer = match pollster::block_on(Renderer::new(
            window.clone(),
            &scene,
            camera,
            parse_present_mode(&self.present_mode),
            self.msaa,
            self.chunk_tiles,
            self.render_scale,
            self.render_scale_auto,
            self.draw_distance,
            model_textures,
            terrain_textures,
            grass_textures,
            self.hud.atlas.clone(),
            self.gpu_timing,
        )) {
            Ok(renderer) => renderer,
            Err(err) => {
                tracing::error!("failed to create renderer: {err:#}");
                event_loop.exit();
                return;
            }
        };

        tracing::info!(
            "startup: textures + renderer {:.0} ms",
            gpu_phase.elapsed().as_secs_f32() * 1000.0
        );
        tracing::info!(
            "renderer: window {}x{}, scene {}x{} ({:.2}x)",
            renderer.size.0,
            renderer.size.1,
            renderer.render_size.0,
            renderer.render_size.1,
            renderer.render_scale
        );
        self.scene = Some(scene);
        renderer.camera.distance = if self.demo_anim {
            780.0
        } else {
            self.initial_distance
        };
        if let Some(pitch) = self.initial_pitch {
            renderer.camera.pitch = pitch;
        }
        if self.demo_anim {
            renderer.camera.pitch = 0.52;
        }
        if self.menu.open {
            window.set_title("MobaScape");
        }
        if self.hero_select.open {
            window.set_title("Rift - Choose Your Hero");
        }
        renderer.set_graphics_mode(!self.graphics_classic);
        renderer.set_shadow_mode(parse_shadow_mode(&self.shadow_mode));
        renderer.set_debug_view(self.debug_view);
        if let Some(theme) = self.ui_theme.as_ref() {
            renderer.set_ui_atlas(&theme.atlas);
        }
        if let Some((path, after)) = self.screenshot_request.take() {
            renderer.request_screenshot(path, after);
            self.exit_after_capture = true;
        }
        if self.hot_shaders {
            renderer.enable_shader_hot_reload();
        }
        renderer.set_lighting(&lighting::Lighting::load_default());
        renderer.update_camera();
        tracing::info!(
            "camera target {:?}, distance {:.0}; click the ground to move",
            renderer.camera.target,
            renderer.camera.distance
        );
        self.window = Some(window);
        self.renderer = Some(renderer);
        self.animated = Some(animated);
        self.last_frame = Instant::now();
        tracing::info!("{}", devtools::HOTKEY_HELP);
        if !self.shot_queue.is_empty() {
            tracing::info!(
                "shots: {} presets into {}",
                self.shot_queue.len(),
                self.shot_dir.display()
            );
            self.exit_after_capture = true;
            self.next_shot();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.resize(size.width, size.height);
                }
            }
            WindowEvent::Focused(focused) => self.focused = focused,
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_pos = (position.x, position.y);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if button == MouseButton::Right {
                    // Casting stays instant; holding right mouse only shows
                    // the last ability's range and aim shape.
                    self.rmb_down = state == ElementState::Pressed;
                    return;
                }
                if button != MouseButton::Left {
                    return;
                }
                let (w, h) = self
                    .renderer
                    .as_ref()
                    .map(|r| (r.size.0 as f32, r.size.1 as f32))
                    .unwrap_or((1280.0, 720.0));
                if self.menu.open {
                    if state == ElementState::Pressed
                        && let Some(index) = menu::button_at(w, h, self.mouse_pos)
                    {
                        self.menu_activate(menu::BUTTONS[index], event_loop);
                    }
                    return;
                }
                if self.hero_select.open {
                    if state == ElementState::Pressed {
                        let card = hero_select::card_at(
                            w,
                            h,
                            self.mouse_pos,
                            &self.hero_select.visible_classes(),
                        );
                        if hero_select::leave_at(w, h, self.mouse_pos) {
                            self.back_to_menu();
                        } else if let Some(filter) = hero_select::filter_at(w, h, self.mouse_pos) {
                            self.hero_select.set_filter(filter);
                        } else if let Some(lane) = hero_select::lane_at(w, h, self.mouse_pos) {
                            self.hero_select.toggle_lane(lane);
                        } else if let Some(slot) = card {
                            self.hero_select.select_slot(slot);
                        } else if hero_select::confirm_at(w, h, self.mouse_pos)
                            && !self.hero_select.is_locked()
                        {
                            self.lock_in();
                        }
                    }
                    return;
                }
                if self.demo_camp
                    || self.demo_combat
                    || self.demo_kit
                    || self.demo_shop
                    || self.demo_base
                    || self.demo_inhibitor
                {
                    return;
                }
                if state == ElementState::Pressed && self.live_interface_click(w, h) {
                    return;
                }
                if state == ElementState::Pressed && self.shop.open {
                    if shop_close_at(w, h, self.mouse_pos) {
                        self.shop.open = false;
                        return;
                    }
                    if let Some(category) = shop_category_at(w, h, self.mouse_pos) {
                        self.shop.category = category;
                        self.shop.selected = self.shop.filtered_items().first().copied();
                        self.shop.scroll = 0;
                        return;
                    }
                    if let Some(item) = shop_card_at(w, h, self.mouse_pos, &self.shop) {
                        self.shop.selected = Some(item);
                        return;
                    }
                    if shop_buy_at(w, h, self.mouse_pos) {
                        if let Some(item) = self.shop.shown() {
                            self.purchase_item(item);
                        }
                        return;
                    }
                    if shop_sell_at(w, h, self.mouse_pos) {
                        if let Some(item) = self.shop.shown() {
                            self.sell_item(item);
                        }
                        return;
                    }
                    if point_in_rect(shop_window_rect(w, h), self.mouse_pos) {
                        return;
                    }
                    self.shop.open = false;
                    return;
                }
                if state == ElementState::Pressed && delivery_button_at(w, h, self.mouse_pos) {
                    if let Some(animated) = self.animated.as_mut() {
                        if animated.call_delivery() {
                            log_event(
                                &mut self.event_log,
                                "ARMADYLEAN GUARD DISPATCHED".to_string(),
                            );
                            tracing::info!("delivery: guard dispatched");
                        }
                    }
                    return;
                }
                if state == ElementState::Pressed
                    && !self.shop.open
                    && shop_button_at(w, h, self.mouse_pos)
                {
                    self.shop.open = true;
                    self.shop.selected = self.shop.filtered_items().first().copied();
                    self.shop.scroll = 0;
                    return;
                }
                match state {
                    ElementState::Pressed => {
                        self.lmb_down = true;
                        self.lmb_press = Some(Instant::now());
                    }
                    ElementState::Released => {
                        let Some(press) = self.lmb_press.take() else {
                            return;
                        };
                        let held = press.elapsed().as_secs_f32();
                        self.lmb_down = false;
                        if held < 0.18 && !self.click_minimap() {
                            self.pick_and_move();
                        }
                    }
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let PhysicalKey::Code(code) = event.physical_key else {
                    return;
                };

                if self.menu.open {
                    if event.state == ElementState::Pressed {
                        self.menu.hover = None;
                        match code {
                            KeyCode::Escape => event_loop.exit(),
                            KeyCode::ArrowUp | KeyCode::KeyW => self.menu.move_focus(-1),
                            KeyCode::ArrowDown | KeyCode::KeyS => self.menu.move_focus(1),
                            KeyCode::Digit1 => self.menu_activate(menu::BUTTONS[0], event_loop),
                            KeyCode::Digit2 => self.menu_activate(menu::BUTTONS[1], event_loop),
                            KeyCode::Digit3 => self.menu_activate(menu::BUTTONS[2], event_loop),
                            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
                                let action = self.menu.focused();
                                self.menu_activate(action, event_loop);
                            }
                            _ => {}
                        }
                    }
                    return;
                }
                if self.hero_select.open {
                    if event.state == ElementState::Pressed {
                        match code {
                            KeyCode::Escape => self.back_to_menu(),
                            KeyCode::ArrowLeft | KeyCode::KeyA => self.hero_select.cycle(-1),
                            KeyCode::ArrowRight | KeyCode::KeyD => self.hero_select.cycle(1),
                            KeyCode::Digit1 => self.hero_select.select_slot(0),
                            KeyCode::Digit2 => self.hero_select.select_slot(1),
                            KeyCode::Digit3 => self.hero_select.select_slot(2),
                            KeyCode::Digit4 => self.hero_select.select_slot(3),
                            KeyCode::KeyF => self.hero_select.set_filter(0),
                            KeyCode::KeyM => self.hero_select.set_filter(1),
                            KeyCode::KeyR => self.hero_select.set_filter(2),
                            KeyCode::KeyC => self.hero_select.set_filter(3),
                            // Lock in; once locked, skip the rest of the
                            // "Game starting in" count.
                            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
                                if self.hero_select.is_locked() {
                                    self.hero_select.skip_countdown();
                                } else {
                                    self.lock_in();
                                }
                            }
                            _ => {}
                        }
                    }
                    return;
                }

                // The shop's search box takes the keyboard while it is
                // focused, so typed letters never cast or open things.
                if self.shop.open && self.shop.typing {
                    if event.state == ElementState::Pressed {
                        match code {
                            KeyCode::Escape => {
                                self.shop.set_query("");
                                self.shop.typing = false;
                            }
                            KeyCode::Enter | KeyCode::NumpadEnter => self.shop.typing = false,
                            KeyCode::Backspace => self.shop.backspace(),
                            _ => {
                                if let Some(text) = event.text.as_ref() {
                                    for ch in text.chars() {
                                        self.shop.type_char(ch);
                                    }
                                }
                            }
                        }
                    } else {
                        // A key held before typing began must not stick.
                        self.held.remove(&code);
                    }
                    return;
                }

                if event.state == ElementState::Pressed {
                    self.held.insert(code);
                    if code == KeyCode::Escape {
                        if self.shop.open {
                            self.shop.open = false;
                        } else {
                            event_loop.exit();
                        }
                    } else if self.dev_hotkey(code) {
                        // Developer keys work in play and spectator mode.
                    } else if code == KeyCode::KeyV {
                        self.toggle_spectator();
                    } else if code == KeyCode::KeyZ {
                        self.cycle_spectator(-1);
                    } else if code == KeyCode::KeyX {
                        self.cycle_spectator(1);
                    } else if self.spectator.is_some() {
                        // Spectator is view-only: gameplay keys stay asleep.
                    } else if code == KeyCode::KeyF {
                        self.follow = !self.follow;
                        println!("follow={}", self.follow);
                    } else if code == KeyCode::KeyQ {
                        self.cast_q();
                    } else if code == KeyCode::KeyW {
                        self.cast_w();
                    } else if code == KeyCode::KeyE {
                        self.cast_e();
                    } else if code == KeyCode::KeyR {
                        self.cast_r();
                    } else if code == KeyCode::KeyB {
                        self.shop.open = !self.shop.open;
                        if self.shop.open {
                            self.shop.selected = self.shop.filtered_items().first().copied();
                            self.shop.scroll = 0;
                        }
                        println!("shop open={}", self.shop.open);
                    } else if let Some(index) = digit_index(code) {
                        if self.shop.open {
                            self.purchase_item(index);
                        }
                    } else if code == KeyCode::Space {
                        self.follow = true;
                        if let (Some(renderer), Some(animated)) =
                            (self.renderer.as_mut(), self.animated.as_ref())
                        {
                            renderer.camera.target = animated.player_pos() + Vec3::Y * 140.0;
                            renderer.update_camera();
                            print_camera(renderer);
                        }
                    }
                } else {
                    self.held.remove(&code);
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let amount = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 120.0,
                };
                if self.ui_live && self.shop.open {
                    let state = crate::ui_live::shop(&self.shop);
                    let max = rift_ui::hud::shop::max_scroll(&state);
                    let step = (amount * 58.0).round() as i32;
                    self.shop.scroll = (self.shop.scroll - step).clamp(0, max);
                    return;
                }
                if !self.focused
                    || !(self.held.contains(&KeyCode::ControlLeft)
                        || self.held.contains(&KeyCode::ControlRight))
                {
                    return;
                }
                let Some(renderer) = self.renderer.as_mut() else {
                    return;
                };
                renderer.camera.distance =
                    (renderer.camera.distance * (1.0 - amount * 0.12)).clamp(900.0, 12_000.0);
                renderer.update_camera();
                print_camera(renderer);
            }
            WindowEvent::RedrawRequested => {
                self.tick();
                let render_start = Instant::now();
                let mut render_error = None;
                if let Some(renderer) = self.renderer.as_mut() {
                    if let Err(err) = renderer.render() {
                        render_error = Some(err);
                    }
                }
                if self.benchmark_remaining.is_some() && self.app_time > 1.0 {
                    self.benchmark_submit_ms += render_start.elapsed().as_secs_f32() * 1000.0;
                }
                if let Some(err) = render_error {
                    tracing::error!("render error: {err:#}");
                    event_loop.exit();
                }
                if !self.first_frame_logged && self.renderer.is_some() {
                    self.first_frame_logged = true;
                    tracing::info!(
                        "startup: first frame after {:.1} s",
                        self.startup.elapsed().as_secs_f32()
                    );
                }
                if self.hot_shaders {
                    self.shader_poll_frames += 1;
                    if self.shader_poll_frames >= 15 {
                        self.shader_poll_frames = 0;
                        if let Some(renderer) = self.renderer.as_mut() {
                            renderer.poll_shader_reload();
                        }
                    }
                }
                if self.exit_after_capture
                    && self
                        .renderer
                        .as_ref()
                        .is_some_and(|renderer| !renderer.screenshot_pending())
                {
                    if self.shot_queue.is_empty() {
                        let written = self
                            .renderer
                            .as_ref()
                            .map_or(0, |renderer| renderer.screenshots_written());
                        tracing::info!(
                            "captures done: {written} written in {:.1} s; exiting",
                            self.startup.elapsed().as_secs_f32()
                        );
                        event_loop.exit();
                    } else {
                        self.next_shot();
                    }
                }
                if let Some(window) = self.window.as_ref() {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.benchmark_done {
            event_loop.exit();
            return;
        }
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }
}

impl App {
    /// Teleport to the next `--shots` preset, frame it and queue a capture.
    fn next_shot(&mut self) {
        let Some(shot) = self.shot_queue.pop_front() else {
            return;
        };
        let (Some(renderer), Some(animated), Some(height)) = (
            self.renderer.as_mut(),
            self.animated.as_mut(),
            self.height.as_ref(),
        ) else {
            return;
        };
        let x = (shot.tile.0 + 0.5) * 128.0;
        let z = (shot.tile.1 + 0.5) * 128.0;
        animated.teleport_player(Vec3::new(x, height.height_at(x, z), z));
        self.follow = true;
        renderer.camera.target = animated.player_pos() + Vec3::Y * 140.0;
        renderer.camera.pitch = shot.pitch.or(self.initial_pitch).unwrap_or(0.95);
        renderer.camera.yaw = shot.yaw.unwrap_or(0.0);
        renderer.camera.distance = shot.distance.unwrap_or(self.initial_distance);
        renderer.set_debug_view(shot.view.unwrap_or(self.debug_view));
        renderer.update_camera();
        let path = shot.output_path(&self.shot_dir);
        tracing::info!("shot {}: {}", shot.name, shot.to_line());
        renderer.request_screenshot(path, self.shot_warmup);
    }

    /// Developer function keys. Returns true when the key was handled.
    fn dev_hotkey(&mut self, code: KeyCode) -> bool {
        let shift =
            self.held.contains(&KeyCode::ShiftLeft) || self.held.contains(&KeyCode::ShiftRight);
        match code {
            KeyCode::F1 => tracing::info!("{}", devtools::HOTKEY_HELP),
            KeyCode::F2 => {
                if let Some(renderer) = self.renderer.as_mut() {
                    let count = devtools::DEBUG_VIEWS.len() as u32;
                    let view = if shift {
                        (renderer.debug_view() + count - 1) % count
                    } else {
                        (renderer.debug_view() + 1) % count
                    };
                    renderer.set_debug_view(view);
                    self.debug_view = view;
                    tracing::info!("debug view: {}", devtools::debug_view_name(view));
                }
            }
            KeyCode::F5 => {
                if let Some(renderer) = self.renderer.as_mut() {
                    if let Err(err) = renderer.reload_scene_shader() {
                        tracing::error!("shader reload failed, keeping the old shader:\n{err:#}");
                    }
                    // The light tuning reloads with the shader, so one key
                    // applies both while iterating on the look.
                    match lighting::Lighting::load(lighting::LIGHTING_PATH) {
                        Ok(tuning) => {
                            renderer.set_lighting(&tuning);
                            tracing::info!("lighting: reloaded {}", lighting::LIGHTING_PATH);
                        }
                        Err(err) => tracing::error!(
                            "lighting reload failed, keeping the old values: {err:#}"
                        ),
                    }
                }
            }
            KeyCode::F7 => {
                if let (Some(renderer), Some(animated)) =
                    (self.renderer.as_ref(), self.animated.as_ref())
                {
                    let pos = animated.player_pos();
                    let shot = devtools::ShotSpec {
                        name: format!("shot_{}", self.app_time as u32),
                        tile: (pos.x / 128.0 - 0.5, pos.z / 128.0 - 0.5),
                        pitch: Some(renderer.camera.pitch),
                        yaw: Some(renderer.camera.yaw),
                        distance: Some(renderer.camera.distance),
                        view: (renderer.debug_view() != 0).then(|| renderer.debug_view()),
                    };
                    tracing::info!("position (paste into a --shots file): {}", shot.to_line());
                }
            }
            KeyCode::F9 => {
                if let Some(renderer) = self.renderer.as_mut() {
                    let path = devtools::hotkey_screenshot_path();
                    tracing::info!("capturing {}", path.display());
                    renderer.request_screenshot(path, 0);
                }
            }
            KeyCode::F10 => {
                self.paused = !self.paused;
                tracing::info!(
                    "simulation {}",
                    if self.paused { "paused" } else { "resumed" }
                );
            }
            _ => return false,
        }
        true
    }

    fn tick(&mut self) {
        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f32().min(0.1);
        self.last_frame = now;
        self.app_time += dt;
        let profiling = self.benchmark_remaining.is_some() && self.app_time > 1.0;
        if profiling && !self.benchmark_profiling {
            self.benchmark_profiling = true;
            if let Some(renderer) = self.renderer.as_mut() {
                renderer.reset_gpu_timing();
            }
        }
        let mut phase_sim = 0.0f32;
        let mut phase_mesh = 0.0f32;
        let mut phase_hud = 0.0f32;
        let mut phase_ui = 0.0f32;

        if let Some(remaining) = self.benchmark_remaining.as_mut() {
            *remaining -= dt;
            // Ignore the first second of shader/present warm-up.
            if self.app_time > 1.0 {
                self.benchmark_samples.push(dt);
            }
            if *remaining <= 0.0 {
                let mut samples = std::mem::take(&mut self.benchmark_samples);
                samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                let percentile = |p: f32| {
                    if samples.is_empty() {
                        0.0
                    } else {
                        samples[((samples.len() - 1) as f32 * p) as usize] * 1000.0
                    }
                };
                let avg = if samples.is_empty() {
                    0.0
                } else {
                    samples.iter().sum::<f32>() / samples.len() as f32 * 1000.0
                };
                tracing::info!(
                    "benchmark: frames={} avg={:.2}ms p50={:.2}ms p95={:.2}ms p99={:.2}ms fps={:.1}",
                    samples.len(),
                    avg,
                    percentile(0.50),
                    percentile(0.95),
                    percentile(0.99),
                    1000.0 / avg.max(0.001)
                );
                let frames = self.benchmark_phase_frames.max(1) as f32;
                let (shadow_ms, mesh_build_ms, mesh_units) = self
                    .animated
                    .as_ref()
                    .map(|a| (a.last_shadow_ms, a.last_mesh_ms, a.last_mesh_units))
                    .unwrap_or((0.0, 0.0, 0));
                tracing::info!(
                    "benchmark phases: sim={:.2}ms mesh={:.2}ms hud={:.2}ms ui={:.2}ms submit={:.2}ms shadow={:.2}ms build={:.2}ms units={}",
                    self.benchmark_sim_ms / frames,
                    self.benchmark_mesh_ms / frames,
                    self.benchmark_hud_ms / frames,
                    self.benchmark_ui_ms / frames,
                    self.benchmark_submit_ms / frames,
                    shadow_ms,
                    mesh_build_ms,
                    mesh_units
                );
                if let Some(renderer) = self.renderer.as_ref()
                    && let Some(passes) = renderer.gpu_pass_ms()
                {
                    let detail = passes
                        .iter()
                        .map(|(name, ms)| format!("{name}={ms:.2}ms"))
                        .collect::<Vec<_>>()
                        .join(" ");
                    let total = renderer.gpu_frame_ms().unwrap_or(0.0);
                    tracing::info!("benchmark gpu: frame={total:.2}ms {detail}");
                }
                self.benchmark_remaining = None;
                self.benchmark_done = true;
            }
        }

        if self.menu.open {
            if self.renderer.is_some() {
                self.tick_menu(dt);
            }
            return;
        }
        if self.hero_select.open {
            if self.renderer.is_some() {
                self.tick_hero_select(dt);
            }
            return;
        }

        self.sync_weapon();
        if self.demo_shop {
            self.demo_shop_timer += dt;
            if self.demo_shop_stage == 0 && self.demo_shop_timer >= 0.5 {
                if let Some(animated) = self.animated.as_mut() {
                    animated.debug_grant_gold(8000);
                }
                self.shop.open = true;
                self.shop.scroll = 0;
                tracing::info!("demo shop: granted 8000 gold, shop open");
                self.demo_shop_stage = 1;
            }
            // Three plain buys, then a sword bought up its ladder (the
            // iron and steel swords are upgrades of the one before).
            const DEMO_BUYS: [&str; 6] = [
                "shortbow",
                "staff_of_air",
                "bronze_platebody",
                "bronze_sword",
                "iron_sword",
                "steel_sword",
            ];
            let demo_index =
                |shop: &ItemShop, id: &str| shop.items.iter().position(|item| item.id == id);
            if (1..=6).contains(&self.demo_shop_stage) {
                let due = self.demo_shop_stage as f32 * 0.5;
                if self.demo_shop_timer >= due {
                    let id = DEMO_BUYS[(self.demo_shop_stage - 1) as usize];
                    if let Some(index) = demo_index(&self.shop, id) {
                        self.purchase_item(index);
                    }
                    self.demo_shop_stage += 1;
                }
            }
            if self.shop.open {
                // Keep the demo cursor over the next unowned row so the
                // hovered-item tooltip is part of the captured verification.
                let (w, h) = self
                    .renderer
                    .as_ref()
                    .map(|r| (r.size.0 as f32, r.size.1 as f32))
                    .unwrap_or((1280.0, 720.0));
                let filtered = self.shop.filtered_items();
                let slot = filtered
                    .iter()
                    .position(|index| !self.shop.is_owned(*index))
                    .unwrap_or(0);
                let (x, y, rw, rh) = shop_card_rect(w, h, slot);
                self.mouse_pos = ((x + rw * 0.5) as f64, (y + rh * 0.5) as f64);
            }
            if self.demo_shop_stage == 7 && self.demo_shop_timer >= 3.5 {
                // Sell the staff, then Undo the steel-sword upgrade, in one
                // frame so passive gold cannot blur the refunds in the log.
                let gold = |app: &Self| app.animated.as_ref().map(|a| a.player_stats().gold);
                let staff = demo_index(&self.shop, "staff_of_air").unwrap_or(0);
                let steel = demo_index(&self.shop, "steel_sword").unwrap_or(0);
                let before = gold(self);
                self.sell_item(staff);
                let sold = gold(self);
                self.undo_purchase();
                let undone = gold(self);
                if let (Some(a), Some(b), Some(c)) = (before, sold, undone) {
                    tracing::info!(
                        "demo shop: sell refunded {}g ({}% of {}), undo refunded {}g, {} undo entries left",
                        b - a,
                        items::SELL_REFUND_PERCENT,
                        self.shop.items[staff].cost,
                        c - b,
                        self.shop.undo.len()
                    );
                }
                let bag: Vec<&str> = self
                    .shop
                    .owned
                    .iter()
                    .flatten()
                    .map(|&i| self.shop.items[i].name.as_str())
                    .collect();
                tracing::info!(
                    "demo shop: bag {bag:?}; steel sword now costs {}g",
                    self.shop.upgrade(steel).price
                );
                self.shop.selected = Some(steel);
                // Search every item for swords: the steel sword stays
                // selected.
                self.shop.category = 1;
                self.shop.typing = true;
                self.shop.set_query("sword");
                tracing::info!(
                    "demo shop: search \"sword\" shows {} items",
                    self.shop.filtered_items().len()
                );
                self.demo_shop_stage = 8;
            }
            if self.demo_shop_stage == 8 && self.demo_shop_timer >= 4.5 {
                if let Some(animated) = self.animated.as_ref() {
                    let s = animated.player_stats();
                    tracing::info!(
                        "demo shop: items {}/6 dmg {:.0} hp {:.0}/{:.0} mp {:.0}/{:.0} speed {:.0} ap {:.0}% gold {}",
                        self.shop.item_count(),
                        s.attack_damage,
                        s.health,
                        s.max_health,
                        s.mana,
                        s.max_mana,
                        s.speed,
                        s.ability_power,
                        s.gold
                    );
                }
                self.demo_shop = false;
            }
        }

        if self.demo_death {
            self.demo_death_timer += dt;
            if self.demo_death_timer >= 1.0 {
                if let Some(animated) = self.animated.as_mut() {
                    animated.demo_kill_player();
                }
                self.demo_death = false;
            }
        }

        if self.demo_base {
            self.demo_base_timer += dt;
            if self.demo_base_stage == 0 && self.demo_base_timer >= 0.5 {
                if let Some(animated) = self.animated.as_mut() {
                    animated.debug_damage_player(300.0);
                    let s = animated.player_stats();
                    tracing::info!(
                        "demo base: damaged to {:.0}/{:.0} hp, {:.0}/{:.0} mp, gold {}",
                        s.health,
                        s.max_health,
                        s.mana,
                        s.max_mana,
                        s.gold
                    );
                }
                self.demo_base_stage = 1;
            }
            if self.demo_base_stage == 1 && self.demo_base_timer >= 4.0 {
                if let Some(animated) = self.animated.as_ref() {
                    let s = animated.player_stats();
                    tracing::info!(
                        "demo base: fountain healed to {:.0}/{:.0} hp, {:.0}/{:.0} mp, passive gold {}",
                        s.health,
                        s.max_health,
                        s.mana,
                        s.max_mana,
                        s.gold
                    );
                }
                self.demo_base = false;
            }
        }

        if self.demo_inhibitor {
            self.demo_inhibitor_timer += dt;
            if self.demo_inhibitor_stage == 0 && self.demo_inhibitor_timer >= 0.5 {
                if let Some(animated) = self.animated.as_mut() {
                    let ok = animated.debug_destroy_inhibitor(1, 0, 30.0);
                    tracing::info!(
                        "demo inhibitor: destroyed={ok} state={:?} inhib1={:?} vulnerable1={} supers={}",
                        animated.debug_inhibitor_state(1, 0),
                        animated.inhibitor_count(1),
                        animated.nexus_vulnerable(1),
                        animated.super_minion_count()
                    );
                }
                self.demo_inhibitor_stage = 1;
            }
            if self.demo_inhibitor_stage == 1 && self.demo_inhibitor_timer >= 2.0 {
                if let Some(animated) = self.animated.as_ref() {
                    tracing::info!(
                        "demo inhibitor: after wave state={:?} inhib1={:?} vulnerable1={} supers={}",
                        animated.debug_inhibitor_state(1, 0),
                        animated.inhibitor_count(1),
                        animated.nexus_vulnerable(1),
                        animated.super_minion_count()
                    );
                }
                self.demo_inhibitor_stage = 2;
            }
            if self.demo_inhibitor_stage == 2 && self.demo_inhibitor_timer >= 5.5 {
                if let Some(animated) = self.animated.as_ref() {
                    tracing::info!(
                        "demo inhibitor: after respawn state={:?} inhib1={:?} vulnerable1={} supers={}",
                        animated.debug_inhibitor_state(1, 0),
                        animated.inhibitor_count(1),
                        animated.nexus_vulnerable(1),
                        animated.super_minion_count()
                    );
                }
                self.demo_inhibitor = false;
            }
        }

        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };

        let over_minimap = self.animated.as_ref().is_some_and(|animated| {
            minimap_world_at(
                renderer.size.0 as f32,
                renderer.size.1 as f32,
                self.mouse_pos,
                animated.minimap_bounds,
            )
            .is_some()
        });
        let steering = self.lmb_down
            && self.spectator.is_none()
            && !over_minimap
            && !self.demo_camp
            && !self.demo_combat
            && !self.demo_kit
            && !self.demo_shop
            && !self.demo_base
            && !self.demo_inhibitor
            && self
                .lmb_press
                .map(|t| t.elapsed().as_secs_f32() >= 0.18)
                .unwrap_or(false);

        let eye = renderer.camera.eye();
        let camera_forward = (eye - renderer.camera.target).normalize_or_zero();
        let camera_right = camera_forward.cross(Vec3::Y).normalize_or_zero();
        let camera_up = camera_right.cross(camera_forward).normalize_or_zero();

        if let (Some(animated), Some(height), Some(collision)) = (
            self.animated.as_mut(),
            self.height.as_ref(),
            self.collision.as_ref(),
        ) {
            if self.demo_combat {
                self.demo_combat_timer += dt;
                if !self.demo_combat_targeted && self.demo_combat_timer >= 1.0 {
                    if let Some(index) = animated.nearest_enemy(animated.player_pos()) {
                        let ok = animated.set_attack_target(index, collision, height);
                        tracing::info!("demo combat: target {index}, attack={ok}");
                        self.demo_combat_target = Some(index);
                    }
                    self.demo_combat_targeted = true;
                }
                if !self.demo_combat_cast && self.demo_combat_timer >= 2.6 {
                    if let Some(index) = self.demo_combat_target {
                        if let Some(pos) = animated.unit_pos(index) {
                            let ok = animated.cast_q(pos);
                            tracing::info!("demo combat: Q at {index}, cast={ok}");
                        }
                    }
                    self.demo_combat_cast = true;
                }
                if self.demo_combat_timer >= 3.5 {
                    if let Some(index) = self.demo_combat_target {
                        if let Some(health) = animated.unit_health(index) {
                            tracing::info!("demo combat: target {index} health {health:.0}");
                        }
                    }
                    self.demo_combat = false;
                }
            }

            if self.demo_camp && !self.demo_camp_done {
                self.demo_camp_timer += dt;
                if self.demo_camp_timer >= 1.0 && self.demo_camp_target.is_none() {
                    if let Some(index) = animated.nearest_camp(animated.player_pos()) {
                        if let Some(camp_pos) = animated.unit_pos(index) {
                            let spawn = camp_pos + Vec3::new(200.0, 0.0, 0.0);
                            animated.teleport_player(spawn);
                            animated.refresh_visibility(1.0, collision);
                            let ok = animated.set_attack_target(index, collision, height);
                            tracing::info!("demo camp: target {index}, attack={ok}");
                            self.demo_camp_target = Some(index);
                        }
                    }
                }
                if !self.demo_camp_cast && self.demo_camp_timer >= 2.6 {
                    if let Some(index) = self.demo_camp_target {
                        if let Some(pos) = animated.unit_pos(index) {
                            let ok = animated.cast_q(pos);
                            tracing::info!("demo camp: Q cast={ok}");
                        }
                    }
                    self.demo_camp_cast = true;
                }
                if self.demo_camp_timer >= 10.0 {
                    if let Some(index) = self.demo_camp_target {
                        let health = animated.unit_health(index).unwrap_or(-1.0);
                        let buff = animated.team_buff(0);
                        let camp_pos = animated.unit_pos(index).unwrap_or(Vec3::ZERO);
                        let home = animated.unit_home(index).unwrap_or(Vec3::ZERO);
                        let aggro = animated.unit_camp_aggro(index).unwrap_or(false);
                        let player_health = animated.player_stats().health;
                        tracing::info!(
                            "demo camp: health {health:.0}, player {player_health:.0}, aggro {aggro}, camp {camp_pos:?}, home {home:?}, drift {:.0}, buff {buff:?}",
                            camp_pos.distance(home)
                        );
                    }
                    self.demo_camp_done = true;
                }
            }
            if self.demo_delivery {
                self.demo_delivery_timer += dt;
                if self.demo_delivery_stage == 0 && self.demo_delivery_timer >= 1.0 {
                    let away = animated.player_pos() + Vec3::new(26.0 * 128.0, 0.0, 0.0);
                    animated.teleport_player(away);
                    animated.debug_grant_gold(8000);
                    let first = self.shop.items[0].clone();
                    let second = self.shop.items[1].clone();
                    let _ = animated.buy_item(0, &first);
                    let _ = animated.buy_item(1, &second);
                    tracing::info!(
                        "demo delivery: pending {}, shop items {}",
                        animated.delivery_count(),
                        self.shop.item_count()
                    );
                    self.demo_delivery_stage = 1;
                }
                if self.demo_delivery_stage == 1 && self.demo_delivery_timer >= 2.0 {
                    let ok = animated.call_delivery();
                    tracing::info!("demo delivery: guard dispatched={ok}");
                    self.demo_delivery_stage = 2;
                }
                if self.demo_delivery_stage == 2 && self.demo_delivery_timer >= 9.0 {
                    let stats = animated.player_stats();
                    tracing::info!(
                        "demo delivery: pending {}, shop items {}, damage {:.0}, hp {:.0}/{:.0}, state {:?}",
                        animated.delivery_count(),
                        self.shop.item_count(),
                        stats.attack_damage,
                        stats.health,
                        stats.max_health,
                        animated.delivery_state
                    );
                    self.demo_delivery = false;
                }
            }
            if self.demo_watchtower {
                self.demo_watchtower_timer += dt;
                if self.demo_watchtower_stage == 0 && self.demo_watchtower_timer >= 1.0 {
                    if let Some(index) = animated.nearest_watchtower(1, animated.player_pos()) {
                        if let Some(pos) = animated.unit_pos(index) {
                            let spawn = pos + Vec3::new(0.0, 0.0, 4.0 * 128.0);
                            animated.teleport_player(spawn);
                            self.demo_watchtower_target = Some(index);
                            tracing::info!(
                                "demo watchtower: target {index}, player inside aggro at 4 tiles"
                            );
                        }
                    }
                    self.demo_watchtower_stage = 1;
                }
                if self.demo_watchtower_stage == 1 && self.demo_watchtower_timer >= 4.0 {
                    let hp = animated.player_stats().health;
                    let target = self
                        .demo_watchtower_target
                        .and_then(|i| animated.unit_attack_target(i));
                    tracing::info!(
                        "demo watchtower: after aggro, player hp {hp:.0}, tower target {target:?}"
                    );
                    if let Some(index) = self.demo_watchtower_target {
                        if let Some(pos) = animated.unit_pos(index) {
                            // 8 tiles away: outside the 620 aggro radius but
                            // inside the 1200 lock range.
                            animated.teleport_player(pos + Vec3::new(0.0, 0.0, 8.0 * 128.0));
                        }
                    }
                    self.demo_watchtower_stage = 2;
                }
                if self.demo_watchtower_stage == 2 && self.demo_watchtower_timer >= 8.0 {
                    let hp = animated.player_stats().health;
                    let target = self
                        .demo_watchtower_target
                        .and_then(|i| animated.unit_attack_target(i));
                    tracing::info!(
                        "demo watchtower: at 8 tiles (outside aggro, inside lock) hp {hp:.0}, tower target {target:?}"
                    );
                    if let Some(index) = self.demo_watchtower_target {
                        if let Some(pos) = animated.unit_pos(index) {
                            // 24 tiles away: outside the lock range.
                            animated.teleport_player(pos + Vec3::new(0.0, 0.0, 24.0 * 128.0));
                        }
                    }
                    self.demo_watchtower_stage = 3;
                }
                if self.demo_watchtower_stage == 3 && self.demo_watchtower_timer >= 12.0 {
                    let hp = animated.player_stats().health;
                    let target = self
                        .demo_watchtower_target
                        .and_then(|i| animated.unit_attack_target(i));
                    tracing::info!(
                        "demo watchtower: at 24 tiles (outside lock) hp {hp:.0}, tower target {target:?}"
                    );
                    self.demo_watchtower = false;
                }
            }
            if !self.demo_gfx.is_empty() {
                if !self.demo_gfx_loaded {
                    animated.preload_spotanims(&self.cache, &self.demo_gfx);
                    self.demo_gfx_loaded = true;
                    self.demo_gfx_timer = 0.5;
                }
                self.demo_gfx_timer -= dt;
                if self.demo_gfx_timer <= 0.0 {
                    // Centred a little past the hero so the bottom row
                    // stays clear of the ability bar.
                    let centre = animated.player_pos() + Vec3::new(0.0, 0.0, -60.0);
                    let longest = animated.demo_gfx(&self.demo_gfx, centre, height);
                    self.demo_gfx_timer = longest + 0.6;
                    // The hero calls the sheet out in overhead chat, which
                    // also checks that a speaker's line follows them.
                    let player = animated.player_index;
                    animated.say(player, &format!("{} spot animations", self.demo_gfx.len()));
                }
            }
            if self.demo_abilities {
                self.demo_abilities_timer += dt;
                let t = self.demo_abilities_timer;
                if self.demo_abilities_stage == 0 && t >= 0.5 {
                    if let Some(camp) = animated.nearest_camp(animated.player_pos())
                        && let Some(pos) = animated.unit_pos(camp)
                    {
                        let offset = Vec3::new(-420.0, 0.0, 0.0);
                        let spot = pos + offset;
                        animated.teleport_player(Vec3::new(
                            spot.x,
                            height.height_at(spot.x, spot.z),
                            spot.z,
                        ));
                        // The camp outlives all four abilities, so each one
                        // (and its graphics) lands on a live target.
                        animated.debug_sturdy(camp, 20_000.0);
                        tracing::info!("demo abilities: beside camp {camp}");
                    }
                    animated.debug_grant_gold(0);
                    self.demo_abilities_stage = 1;
                }
                let slot_times = [1.5f32, 2.8, 4.1, 5.4];
                for (slot, &at) in slot_times.iter().enumerate() {
                    if self.demo_abilities_stage == 1 + slot as u8 && t >= at {
                        // Aim at the camp beside the hero; `nearest_enemy`
                        // would pick a team-1 unit anywhere on the map.
                        let aim = animated
                            .nearest_camp(animated.player_pos())
                            .and_then(|index| animated.unit_pos(index))
                            .unwrap_or(animated.player_pos() + Vec3::new(400.0, 0.0, 0.0));
                        animated.debug_refill_mana();
                        animated.kit_aim = aim;
                        let ok = match slot {
                            0 => animated.cast_q(aim),
                            1 => animated.cast_w(aim, collision, height),
                            2 => animated.cast_e(collision, height),
                            _ => animated.cast_r(aim),
                        };
                        tracing::info!("demo abilities: slot {slot} cast={ok}");
                        self.demo_abilities_stage += 1;
                    }
                }
                if self.demo_abilities_stage == 5 && t >= 7.0 {
                    tracing::info!("demo abilities: {}", animated.debug_kit_report());
                    self.demo_abilities = false;
                }
            }

            if let Some(slot) = self.demo_ranks {
                if !self.demo_ranks_done {
                    animated.dev_level_to(7);
                    let spent = [0, 0, 1, 3].map(|slot| animated.upgrade_ability(slot));
                    tracing::info!(
                        "demo ranks: level 7, spent {spent:?}, ranks {:?}, {} points left",
                        animated.ability_ranks(),
                        animated.skill_points
                    );
                    self.demo_ranks_done = true;
                }
                // Keep the cursor on the slot so the tooltip is captured.
                if let Some(area) = self.ui_placed.as_ref().and_then(|placed| {
                    placed
                        .get(rift_ui::hud::Piece::Ability)
                        .map(|a| (a, placed.scale as i32))
                }) {
                    let (area, scale) = area;
                    let x = area.x + (92 + slot.min(3) as i32 * 42 + 20) * scale;
                    let y = area.y + 30 * scale;
                    self.mouse_pos = (x as f64, y as f64);
                }
            }

            if self.demo_attack && self.demo_attack_stage == 0 {
                if let Some(camp) = animated.nearest_camp(animated.player_pos())
                    && let Some(pos) = animated.unit_pos(camp)
                {
                    let spot = pos + Vec3::new(-1000.0, 0.0, 160.0);
                    animated.teleport_player(Vec3::new(
                        spot.x,
                        height.height_at(spot.x, spot.z),
                        spot.z,
                    ));
                    let ok = animated.set_attack_target(camp, collision, height);
                    tracing::info!("demo attack: camp {camp} ordered={ok}");
                }
                self.demo_attack_stage = 1;
            }

            if self.demo_kit {
                self.demo_kit_timer += dt;
                if self.demo_kit_stage == 0 && self.demo_kit_timer >= 1.0 {
                    let dest = animated.player_pos() + Vec3::new(500.0, 0.0, 300.0);
                    let ok = animated.cast_w(dest, collision, height);
                    tracing::info!("demo kit: W cast={ok}");
                    self.demo_kit_stage = 1;
                }
                if self.demo_kit_stage == 1 && self.demo_kit_timer >= 2.0 {
                    let ok = animated.cast_e(collision, height);
                    tracing::info!("demo kit: E cast={ok}");
                    self.demo_kit_stage = 2;
                }
                if self.demo_kit_stage == 2 && self.demo_kit_timer >= 3.0 {
                    if let Some(index) = animated.nearest_enemy(animated.player_pos()) {
                        let pos = animated.unit_pos(index).unwrap_or(animated.player_pos());
                        let ok = animated.cast_r(pos);
                        tracing::info!("demo kit: R cast={ok}");
                    }
                    self.demo_kit_stage = 3;
                }
                if self.demo_kit_stage == 3 && self.demo_kit_timer >= 4.0 {
                    let s = animated.player_stats();
                    tracing::info!(
                        "demo kit: level {} xp {:.0}/{:.0} gold {} mana {:.0}/{:.0} cds {:?}",
                        s.level,
                        s.xp,
                        s.xp_next,
                        s.gold,
                        s.mana,
                        s.max_mana,
                        s.cooldowns
                    );
                    self.demo_kit = false;
                }
            }

            if self.demo_awowogei {
                self.demo_awowogei_timer += dt;
                let t = self.demo_awowogei_timer;
                let rival = animated
                    .units
                    .iter()
                    .position(|unit| unit.is_champion && unit.team == 1 && unit.health > 0.0);
                let rival_pos = rival.and_then(|index| animated.unit_pos(index));
                let aim = rival_pos.unwrap_or(animated.player_pos() + Vec3::new(400.0, 0.0, 0.0));
                let steps = [1.0f32, 3.2, 4.4, 5.6];
                let stage = self.demo_awowogei_stage as usize;
                if stage < steps.len() && t >= steps[stage] {
                    animated.debug_refill_mana();
                    animated.kit_aim = aim;
                    let ok = match stage {
                        0 => animated.cast_q(aim),
                        1 => animated.cast_e(collision, height),
                        2 => animated.cast_r(aim),
                        _ => {
                            let past =
                                aim + (aim - animated.player_pos()).normalize_or_zero() * 500.0;
                            animated.cast_w(past, collision, height)
                        }
                    };
                    tracing::info!(
                        "demo awowogei: slot {} cast={ok} aim={aim:?}",
                        [0, 2, 3, 1][stage]
                    );
                    self.demo_awowogei_stage += 1;
                }
                if self.demo_awowogei_stage == 4 && t >= 9.0 {
                    let stats = animated.player_stats();
                    let rival = rival.map(|index| {
                        let unit = &animated.units[index];
                        (unit.health, unit.max_health)
                    });
                    tracing::info!(
                        "demo awowogei: king hp={:.0}/{:.0} at {:?} rival={rival:?} bananas={} peels={} gorilla={}",
                        stats.health,
                        stats.max_health,
                        animated.player_pos(),
                        animated.awowogei.bananas.len(),
                        animated.awowogei.peels.len(),
                        animated.awowogei.greegree.is_some(),
                    );
                    self.demo_awowogei = false;
                }
            }

            if self.demo_anim {
                self.demo_anim_timer += dt;
                let stage = self.demo_anim_stage;
                match stage {
                    1 => animated.steer_player(Vec3::X),
                    2 => animated.steer_player(-Vec3::X),
                    4 if !self.demo_anim_targeted => {
                        if let Some(index) = animated
                            .units
                            .iter()
                            .position(|unit| unit.kind == UnitKind::Dummy && unit.health > 0.0)
                        {
                            animated.set_attack_target(index, collision, height);
                        }
                        self.demo_anim_targeted = true;
                    }
                    _ => {}
                }
                const STAGE_SECONDS: [f32; 5] = [1.4, 1.6, 1.6, 1.0, 2.6];
                if self.demo_anim_timer >= STAGE_SECONDS[stage as usize] {
                    self.demo_anim_timer = 0.0;
                    self.demo_anim_stage = (stage + 1) % STAGE_SECONDS.len() as u8;
                    self.demo_anim_targeted = false;
                    if matches!(self.demo_anim_stage, 0 | 3) {
                        animated.clear_player_orders();
                    }
                    const LABELS: [&str; 5] = [
                        "idle",
                        "walk east",
                        "turn and walk west",
                        "coast to stop",
                        "attack dummy",
                    ];
                    let label = LABELS[self.demo_anim_stage as usize];
                    log_event(&mut self.event_log, format!("ANIM DEMO: {label}"));
                    tracing::info!("animation demo: stage {} ({label})", self.demo_anim_stage);
                }
            }

            if steering && !self.demo_anim {
                if let Some(point) =
                    renderer.pick_ground(self.mouse_pos.0, self.mouse_pos.1, height)
                {
                    let dir = point - animated.player_pos();
                    animated.steer_player(dir);
                }
            }
            if let Some((tx, ty)) = self.demo_target {
                self.demo_timer += dt;
                if self.demo_timer >= 1.0 {
                    let x = tx * 128.0 + 64.0;
                    let z = ty * 128.0 + 64.0;
                    let point = Vec3::new(x, height.height_at(x, z), z);
                    let waypoints = animated.set_player_target(point, collision, height);
                    tracing::info!(
                        "demo move target tile {:.0},{:.0} -> {waypoints} waypoints",
                        tx,
                        ty
                    );
                    self.demo_target = None;
                }
            }
            let steps = if self.paused {
                0
            } else {
                self.timescale.round().clamp(1.0, 20.0) as u32
            };
            let sim_start = Instant::now();
            for _ in 0..steps {
                animated.update(dt, height, collision);
                if self.demo_fx {
                    // Re-pin the demo numbers so they do not drift off screen.
                    animated.demo_fx();
                }
                if self.demo_hitsplats {
                    animated.demo_hitsplats();
                }
            }
            phase_sim = sim_start.elapsed().as_secs_f32() * 1000.0;
            let cues = std::mem::take(&mut animated.sound_cues);
            if !cues.is_empty() {
                // Screen right in a right-handed view: look x up.
                let look = renderer.camera.target - renderer.camera.eye();
                let listener = audio::Listener {
                    pos: renderer.camera.target,
                    right: look.cross(Vec3::Y).normalize_or_zero(),
                };
                self.audio.play_cues(&self.cache, &listener, cues);
            }
            let current_health = animated.player_stats().health;
            if self.last_health > current_health + 0.5 {
                self.damage_flash = (self.damage_flash + 0.55).min(0.9);
            }
            self.last_health = current_health;
            self.damage_flash = (self.damage_flash - dt * 2.4).max(0.0);
            animated.refresh_visibility(dt, collision);
            let indicator = if let Some(slot) = self.hold_indicator {
                animated.last_cast_slot = Some(slot.min(3));
                let from = animated.player_pos();
                animated
                    .nearest_camp(from)
                    .and_then(|camp| animated.unit_pos(camp))
                    .or(Some(from + Vec3::new(600.0, 0.0, 0.0)))
            } else if self.rmb_down && !self.shop.open {
                renderer.pick_ground(self.mouse_pos.0, self.mouse_pos.1, height)
            } else {
                None
            };
            animated.set_indicator_aim(indicator);
            let mesh_start = Instant::now();
            animated.prepare_render(
                height,
                renderer.camera.target,
                camera_cull_radius(renderer.size, &renderer.camera),
                camera_right,
                camera_up,
                camera_forward,
            );
            renderer.set_dynamic_mesh(&animated.vertices, &animated.indices);
            renderer.set_fx_mesh(&animated.fx.vertices, &animated.fx.indices);
            renderer.set_shadow_mesh(&animated.shadow_vertices, &animated.shadow_indices);
            renderer.set_caster_mesh(&animated.caster_vertices, &animated.caster_indices);
            renderer.set_lights(&animated.lights());
            renderer.set_time(self.app_time);
            // Fog of war is a shader-side circle around the hero, so it has
            // no geometry to fight with the terrain's z levels.
            renderer.set_vision(animated.player_pos(), 2600.0);
            if animated.visibility_dirty {
                renderer.set_vision_grid(
                    animated.vision_origin,
                    animated.visibility.width,
                    animated.visibility.height,
                    &animated.visibility.tiles,
                );
            }
            animated.visibility_dirty = false;
            phase_mesh = mesh_start.elapsed().as_secs_f32() * 1000.0;

            let nexus0 = animated.nexus_health(0).unwrap_or((0.0, 1.0));
            let nexus1 = animated.nexus_health(1).unwrap_or((0.0, 1.0));
            self.lane_log_timer += dt;
            if self.lane_log_timer >= 5.0 {
                self.lane_log_timer = 0.0;
                tracing::debug!("minion states: {:?}", animated.debug_minion_states());
                tracing::info!(
                    "lane: minions={} supers={} visible_enemies={} towers0={:?} towers1={:?} inhib0={:?} inhib1={:?} nexus0={:?} nexus1={:?} vulnerable0={} vulnerable1={}",
                    animated.minion_count(),
                    animated.super_minion_count(),
                    animated.visible_enemy_count(),
                    animated.tower_healths(0),
                    animated.tower_healths(1),
                    animated.inhibitor_count(0),
                    animated.inhibitor_count(1),
                    animated.nexus_health(0),
                    animated.nexus_health(1),
                    animated.nexus_vulnerable(0),
                    animated.nexus_vulnerable(1)
                );
            }
            let buff_data = animated.team_buff(0).map(|(remaining, duration, kind)| {
                let (color, label) = match kind {
                    CampBuff::Might => ([255, 160, 60], "MIGHT"),
                    CampBuff::Swift => ([90, 220, 255], "SWIFT"),
                };
                (remaining, duration, color, label)
            });
            let buff =
                buff_data.map(|(remaining, duration, color, _)| (remaining, duration, color));
            let buff_label = buff_data.map(|(_, _, _, label)| label);
            let stats = animated.player_stats();
            if stats.level > self.last_level {
                self.last_level = stats.level;
                log_event(&mut self.event_log, format!("LEVEL UP! L{}", stats.level));
            }
            if buff_label != self.last_buff_label {
                self.last_buff_label = buff_label;
                if let Some(label) = buff_label {
                    log_event(&mut self.event_log, format!("BUFF {label}"));
                }
            }
            let i0 = animated.inhibitor_count(0);
            if i0.0 < self.last_inhib0.0 {
                log_event(&mut self.event_log, "INHIBITOR DOWN".to_string());
            } else if i0.0 > self.last_inhib0.0 {
                log_event(&mut self.event_log, "INHIBITOR RESPAWN".to_string());
            }
            self.last_inhib0 = i0;
            let i1 = animated.inhibitor_count(1);
            if i1.0 < self.last_inhib1.0 {
                log_event(&mut self.event_log, "ENEMY INHIBITOR DOWN".to_string());
            } else if i1.0 > self.last_inhib1.0 {
                log_event(&mut self.event_log, "ENEMY INHIBITOR UP".to_string());
            }
            self.last_inhib1 = i1;
            let (hud_w, hud_h) = (renderer.size.0 as f32, renderer.size.1 as f32);
            let shop_hover = if self.shop.open {
                shop_card_at(hud_w, hud_h, self.mouse_pos, &self.shop)
            } else {
                None
            };
            let towers = {
                let healths = animated.tower_healths(0);
                let alive = healths.iter().filter(|(hp, _)| *hp > 0.0).count();
                (alive, healths.len())
            };
            self.shop.sync_visit();
            for index in animated.take_delivered() {
                self.shop.add_owned(index);
                // Undo now takes it back out of the bag, not the chest.
                if let Some(purchase) = self
                    .shop
                    .undo
                    .iter_mut()
                    .find(|p| p.index == index && p.stored)
                {
                    purchase.stored = false;
                }
                if let Some(item) = self.shop.items.get(index) {
                    log_event(&mut self.event_log, format!("DELIVERED {}", item.name));
                    tracing::info!("delivery: handed over {}", item.name);
                }
            }
            let scores = animated.team_scores();
            let heroes: Vec<HeroMarker> = animated
                .units
                .iter()
                .enumerate()
                // You, your bot allies, and enemy heroes your team can see.
                .filter(|(_, unit)| unit.is_champion && (unit.team == 0 || unit.visible))
                .map(|(index, unit)| HeroMarker {
                    name: if unit.kind == UnitKind::Player {
                        animated.champion_name().to_string()
                    } else {
                        animated
                            .bot_of(index)
                            .map(|bot| bot.name.clone())
                            .unwrap_or_default()
                    },
                    team: unit.team,
                    pos: (unit.pos.x, unit.pos.z),
                    alive: unit.health > 0.0,
                    health: unit.health,
                    max_health: unit.max_health,
                    mana: unit.mana,
                    max_mana: unit.max_mana,
                    level: unit.level,
                    you: unit.kind == UnitKind::Player,
                })
                .collect();
            let hover = {
                let mut hover = None;
                if let Some((origin, dir)) = renderer.screen_ray(self.mouse_pos.0, self.mouse_pos.1)
                {
                    if let Some(index) = animated.pick_unit(origin, dir) {
                        if let Some((name, level, description)) = animated.unit_info(index) {
                            let (health, max_health, team, kind) = animated.unit_status(index);
                            hover = Some(HoverInfo {
                                name,
                                level,
                                health,
                                max_health,
                                team,
                                kind,
                                description,
                            });
                        }
                    }
                }
                hover
            };
            let roster = animated
                .units
                .iter()
                .enumerate()
                .filter(|(_, unit)| unit.is_champion)
                .map(|(index, unit)| HeroMarker {
                    name: if unit.kind == UnitKind::Player {
                        animated.champion_name().to_string()
                    } else {
                        animated
                            .bot_of(index)
                            .map(|bot| bot.name.clone())
                            .unwrap_or_else(|| "RIVAL CHAMPION".to_string())
                    },
                    team: unit.team,
                    pos: (unit.pos.x, unit.pos.z),
                    alive: unit.health > 0.0,
                    health: unit.health,
                    max_health: unit.max_health,
                    mana: unit.mana,
                    max_mana: unit.max_mana,
                    level: unit.level,
                    you: unit.kind == UnitKind::Player,
                })
                .collect();
            // Project the world's floating numbers into screen space so the
            // HUD can draw them over the units that were hit.
            let aspect = hud_w / hud_h.max(1.0);
            let view_proj = renderer.camera.view_proj(aspect);
            let mut damage_numbers = Vec::new();
            let mut overhead = Vec::new();
            let mut hitsplats = Vec::new();
            for float in &animated.floats {
                let clip = view_proj * glam::Vec4::new(float.pos.x, float.pos.y, float.pos.z, 1.0);
                if clip.w <= 0.0 {
                    continue;
                }
                let ndc_x = clip.x / clip.w;
                let ndc_y = clip.y / clip.w;
                if ndc_x.abs() > 1.2 || ndc_y.abs() > 1.2 {
                    continue;
                }
                let screen_x = (ndc_x * 0.5 + 0.5) * hud_w;
                let screen_y = (1.0 - (ndc_y * 0.5 + 0.5)) * hud_h;
                match float.style {
                    FloatStyle::Hitsplat(kind) => hitsplats.push(crate::hud::Hitsplat {
                        x: screen_x,
                        y: screen_y,
                        text: float.text.clone(),
                        kind,
                        alpha: (float.life / 0.3).clamp(0.0, 1.0),
                        stack: float.stack,
                        tint: float.tint,
                    }),
                    FloatStyle::Label(color) => damage_numbers.push((
                        screen_x,
                        screen_y,
                        float.text.clone(),
                        color,
                        float.scale,
                    )),
                    FloatStyle::Say => {
                        // A speaker's line sits above their plate: over the
                        // 2x name, and over the skull and tag of an enemy
                        // hero. A line with no speaker stays where it is.
                        let lift = match float.unit.and_then(|index| animated.units.get(index)) {
                            Some(unit) if unit.kind == UnitKind::BotChampion && unit.team == 1 => {
                                108.0
                            }
                            Some(unit) if unit.is_champion => 46.0,
                            Some(_) => 28.0,
                            None => 0.0,
                        };
                        overhead.push((
                            screen_x,
                            screen_y - lift,
                            float.text.clone(),
                            (float.life / 0.3).clamp(0.0, 1.0),
                        ))
                    }
                }
            }
            // Screen-space name/health plates. Minions keep their cheap
            // world-space bars; champions, structures and camps get the
            // reference client's readable frame.
            let mut unit_frames = Vec::new();
            for (index, unit) in animated.units.iter().enumerate() {
                if !unit.visible || unit.health <= 0.0 || unit.spawn_delay > 0.0 {
                    continue;
                }
                let plate = matches!(
                    unit.kind,
                    UnitKind::Player
                        | UnitKind::BotChampion
                        | UnitKind::Tower
                        | UnitKind::Inhibitor
                        | UnitKind::Nexus
                        | UnitKind::Watchtower
                        | UnitKind::Camp
                        | UnitKind::Dummy
                );
                if !plate {
                    continue;
                }
                let clip = view_proj
                    * glam::Vec4::new(
                        unit.pos.x,
                        unit.pos.y + unit.bar_height + 12.0,
                        unit.pos.z,
                        1.0,
                    );
                if clip.w <= 0.0 {
                    continue;
                }
                let ndc_x = clip.x / clip.w;
                let ndc_y = clip.y / clip.w;
                if ndc_x.abs() > 1.15 || ndc_y.abs() > 1.15 {
                    continue;
                }
                let Some((name, level, _)) = animated.unit_info(index) else {
                    continue;
                };
                let hero = matches!(unit.kind, UnitKind::Player | UnitKind::BotChampion);
                let structure = matches!(
                    unit.kind,
                    UnitKind::Tower | UnitKind::Inhibitor | UnitKind::Nexus | UnitKind::Watchtower
                );
                let camp = unit.kind == UnitKind::Camp;
                let pk = unit.kind == UnitKind::BotChampion && unit.team == 1;
                unit_frames.push(crate::hud::UnitFrame {
                    x: (ndc_x * 0.5 + 0.5) * hud_w,
                    y: (1.0 - (ndc_y * 0.5 + 0.5)) * hud_h,
                    name,
                    level,
                    health: unit.health,
                    max_health: unit.max_health,
                    mana: unit.mana,
                    max_mana: unit.max_mana,
                    team: unit.team,
                    hero,
                    camp,
                    structure,
                    name_color: crate::hud::unit_name_color(unit.team, hero, camp, unit.camp_buff),
                    name_scale: if hero { 2.0 } else { 1.0 },
                    pk,
                    tag: pk.then(|| "NO SAFE".to_string()),
                    width: if hero {
                        184.0
                    } else if structure {
                        170.0
                    } else {
                        116.0
                    },
                });
            }
            let player_unit = &animated.units[animated.player_index];
            let cast_progress = if player_unit.attack_pending {
                Some(
                    (1.0 - player_unit.attack_windup_timer / player_unit.attack_windup.max(0.001))
                        .clamp(0.0, 1.0),
                )
            } else {
                None
            };
            let player_hud = PlayerHud {
                health: stats.health,
                max_health: stats.max_health,
                mana: stats.mana,
                max_mana: stats.max_mana,
                xp: stats.xp,
                xp_next: stats.xp_next,
                cooldowns: stats.cooldowns,
                cooldown_maxima: stats.cooldown_maxima,
                armor: stats.armor,
                ranged: stats.ranged,
                attack_interval: stats.attack_interval,
                ability_power: stats.ability_power,
                attack_damage: stats.attack_damage,
                shield: stats.shield,
                shield_timer: stats.shield_timer,
                buff,
                buff_label,
                level: stats.level,
                gold: stats.gold,
                shop_open: self.shop.open,
                shop_hover,
                shop_selected: self.shop.selected,
                champion_name: animated.champion_name().to_string(),
                minions: animated.minion_count(),
                visible_enemies: animated.visible_enemy_count(),
                towers,
                inhibitors: animated.inhibitor_count(0),
                items_owned: self.shop.item_count(),
                events: self.event_log.clone(),
                kills: (scores.0.0, scores.1.0),
                deaths: (scores.0.1, scores.1.1),
                player_kills: stats.kills,
                player_deaths: stats.deaths,
                player_assists: stats.assists,
                player_cs: stats.cs,
                debug_hud: self.debug_hud,
                match_time: self.app_time,
                hover,
                biome: animated.current_biome(),
                heroes: heroes.clone(),
                roster,
                damage_numbers,
                overhead,
                hitsplats,
                unit_frames,
                cast_progress,
                damage_flash: self.damage_flash,
                spectator: self.spectator.and_then(|index| {
                    let unit = animated.units.get(index)?;
                    let (name, level, _) = animated.unit_info(index)?;
                    Some(crate::hud::SpectatorInfo {
                        name,
                        level,
                        health: unit.health,
                        max_health: unit.max_health,
                        mana: unit.mana,
                        max_mana: unit.max_mana,
                        team: unit.team,
                    })
                }),
                respawn_remaining: {
                    let player = &animated.units[animated.player_index];
                    (player.health <= 0.0).then_some(player.dead_timer.max(0.0))
                },
                announcement: animated.announcement.as_ref().map(|(text, _)| text.clone()),
                kill_feed: animated
                    .kill_feed
                    .iter()
                    .map(|(text, _)| text.clone())
                    .collect(),
                delivery_pending: animated.delivery_count(),
                delivery_state: match animated.delivery_state {
                    crate::animated::DeliveryState::Idle => 0,
                    crate::animated::DeliveryState::FlyingToHero => 1,
                    crate::animated::DeliveryState::Returning => 2,
                },
                pending_ids: animated
                    .delivery_pending
                    .iter()
                    .map(|(_, item)| item.id.clone())
                    .collect(),
            };
            let hud_start = Instant::now();
            // The assembled interface owns the screen under --ui-live. Do
            // not build the legacy HUD, which would duplicate the same
            // panels and bars underneath it.
            if !self.ui_live {
                self.hud.build(
                    renderer.size.0 as f32,
                    renderer.size.1 as f32,
                    &player_hud,
                    &self.shop,
                    nexus0,
                    nexus1,
                    animated.wave_timer,
                    animated.wave_interval,
                    animated.match_over,
                    self.mouse_pos,
                );
                self.hud.build_shop(hud_w, hud_h, &player_hud, &self.shop);
                self.hud.build_minimap(
                    renderer.size.0 as f32,
                    renderer.size.1 as f32,
                    animated,
                    (renderer.camera.target.x, renderer.camera.target.z),
                    renderer.camera.distance,
                    renderer.camera.pitch,
                    &heroes,
                );
            } else {
                // Nothing legacy is drawn in a live match: drop whatever the
                // menu or hero select left in the mesh, or its last frame
                // stays on screen over the arena.
                self.hud.vertices.clear();
                self.hud.indices.clear();
            }
            renderer.set_hud_mesh(&self.hud.vertices, &self.hud.indices);
            phase_hud = hud_start.elapsed().as_secs_f32() * 1000.0;
            let ui_camera = (
                renderer.camera.target.x,
                renderer.camera.target.z,
                renderer.camera.distance,
                renderer.camera.pitch,
            );
            let ui_start = Instant::now();
            let placed = draw_ui_overlay(
                renderer,
                self.ui_theme.as_ref(),
                self.ui_theme_sheet,
                self.ui_layout,
                self.ui_live,
                self.ui_scale,
                &mut self.ui_shown,
                Some(&player_hud),
                Some(animated),
                self.scene.as_ref(),
                Some(&self.shop),
                ui_camera,
                (self.mouse_pos.0 as i32, self.mouse_pos.1 as i32),
                self.selected,
            );
            if self.ui_live {
                self.ui_placed = placed;
            }
            phase_ui = ui_start.elapsed().as_secs_f32() * 1000.0;
            if let Some(index) = self.spectator {
                if let Some(unit) = animated.units.get(index) {
                    renderer.camera.target = unit.pos + Vec3::Y * 140.0;
                    renderer.update_camera();
                }
            } else if self.follow {
                renderer.camera.target = animated.player_pos() + Vec3::Y * 140.0;
                renderer.update_camera();
            }

            self.title_timer += dt;
            self.title_frames += 1;
            if self.title_timer >= 0.5 {
                let fps = self.title_frames as f32 / self.title_timer;
                let pos = animated.player_pos();
                let stats = animated.player_stats();
                let (hp, max_hp) = (stats.health, stats.max_health);
                let nexus0 = animated.nexus_health(0).unwrap_or((0.0, 1.0));
                let nexus1 = animated.nexus_health(1).unwrap_or((0.0, 1.0));
                let result = match animated.match_over {
                    Some(0) => " | VICTORY",
                    Some(_) => " | DEFEAT",
                    None => "",
                };
                let inhib0 = animated.inhibitor_count(0);
                let inhib1 = animated.inhibitor_count(1);
                let shield0 = if animated.nexus_vulnerable(0) {
                    "open"
                } else {
                    "shielded"
                };
                let shield1 = if animated.nexus_vulnerable(1) {
                    "open"
                } else {
                    "shielded"
                };
                let buff_text = animated
                    .team_buff(0)
                    .map(|(remaining, _, kind)| format!("{:?} {remaining:.0}s", kind))
                    .unwrap_or_else(|| "-".to_string());
                let shop_note = if self.shop.open {
                    match shop_card_at(
                        renderer.size.0 as f32,
                        renderer.size.1 as f32,
                        self.mouse_pos,
                        &self.shop,
                    ) {
                        Some(i) => format!(
                            " | SHOP {} {}g",
                            self.shop.items[i].name, self.shop.items[i].cost
                        ),
                        None => " | SHOP open (B)".to_string(),
                    }
                } else {
                    String::new()
                };
                if let Some(window) = self.window.as_ref() {
                    window.set_title(&format!(
                        "Rift - Karamja | {} | tile {:.0},{:.0} | {:.0} fps | L{} {:.0}/{:.0} XP | HP {:.0}/{:.0} | MP {:.0}/{:.0} | gold {} | dmg {:.0} spd {:.0} | items {}/6{} | Q/W/E/R {:.1}/{:.1}/{:.1}/{:.1}s | nexus {:.0}/{:.0} ({}) - {:.0}/{:.0} ({}) | inhib {}/{} - {}/{} | wave {:.0}s | minions {} | buff {}{}",
                        animated.champion_name(),
                        pos.x / 128.0,
                        pos.z / 128.0,
                        fps,
                        stats.level,
                        stats.xp,
                        stats.xp_next,
                        hp,
                        max_hp,
                        stats.mana,
                        stats.max_mana,
                        stats.gold,
                        stats.attack_damage,
                        stats.speed,
                        self.shop.item_count(),
                        shop_note,
                        stats.cooldowns[0],
                        stats.cooldowns[1],
                        stats.cooldowns[2],
                        stats.cooldowns[3],
                        nexus0.0,
                        nexus0.1,
                        shield0,
                        nexus1.0,
                        nexus1.1,
                        shield1,
                        inhib0.0,
                        inhib0.1,
                        inhib1.0,
                        inhib1.1,
                        animated.wave_timer,
                        animated.minion_count(),
                        buff_text,
                        result
                    ));
                }
                self.title_timer = 0.0;
                self.title_frames = 0;
            }
        }

        if profiling {
            self.benchmark_sim_ms += phase_sim;
            self.benchmark_mesh_ms += phase_mesh;
            self.benchmark_hud_ms += phase_hud;
            self.benchmark_ui_ms += phase_ui;
            self.benchmark_phase_frames += 1;
        }
        self.update_camera(dt);
    }

    /// Keep the hero's hand in step with the inventory: the first weapon
    /// owned, else the champion's cosmetic weapon. Cheap when nothing changed.
    fn sync_weapon(&mut self) {
        let Some(animated) = self.animated.as_ref() else {
            return;
        };
        if !animated.player_can_wield {
            return;
        }
        let default = self
            .hero_select
            .selected_champion()
            .and_then(|champion| champion.weapon.as_deref())
            .and_then(|id| self.shop.items.iter().find(|item| item.id == id))
            .and_then(|item| item.obj);
        let wanted = wield::chosen_weapon(&self.shop.owned, &self.shop.items, default);
        if wanted == animated.player_weapon() {
            return;
        }
        let parts = match wanted {
            Some(obj) => self.weapon_parts(obj),
            None => Vec::new(),
        };
        // An item without a wield model leaves the hand empty rather than
        // retrying every frame.
        if let Some(animated) = self.animated.as_mut() {
            animated.equip_weapon(wanted, &parts);
        }
    }

    fn weapon_parts(&mut self, obj: u32) -> Vec<rift_cache::Model> {
        if let Some(parts) = self.weapon_models.get(&obj) {
            return parts.clone();
        }
        let parts = wield::load_weapon(&self.cache, obj).unwrap_or_else(|err| {
            tracing::warn!("wield: {err:#}");
            Vec::new()
        });
        self.weapon_models.insert(obj, parts.clone());
        parts
    }

    /// Render the paused world behind the champion picker, then, once
    /// everyone is locked in, count down on the same screen and load the
    /// match (the reference has no lobby between the two).
    fn tick_hero_select(&mut self, dt: f32) {
        // The previous frame showed "Loading the arena...": load now.
        if self.hero_select.ready_to_load() {
            self.start_match();
            return;
        }
        self.hero_select.time += dt;
        let (w, h) = self
            .renderer
            .as_ref()
            .map(|r| (r.size.0 as f32, r.size.1 as f32))
            .unwrap_or((1280.0, 720.0));
        self.hero_select.hover = if self.hero_select.is_locked() {
            None
        } else {
            hero_select::card_at(w, h, self.mouse_pos, &self.hero_select.visible_classes())
        };
        if self.hero_select.is_locked() {
            self.hero_select.tick_locked(dt);
        } else if self.hero_select.tick_countdown(dt) {
            tracing::info!("hero select: time is up");
            self.lock_in();
        }
        let second = self
            .hero_select
            .starting_in()
            .unwrap_or_else(|| self.hero_select.seconds_left());
        if second <= 5 && self.last_tick_second != Some(second) {
            self.play_ui_sound("skill_point");
        }
        self.last_tick_second = Some(second);
        self.update_spotlight(dt);

        if let (Some(renderer), Some(animated), Some(height), Some(collision)) = (
            self.renderer.as_mut(),
            self.animated.as_mut(),
            self.height.as_ref(),
            self.collision.as_ref(),
        ) {
            let eye = renderer.camera.eye();
            let forward = (eye - renderer.camera.target).normalize_or_zero();
            let right = forward.cross(Vec3::Y).normalize_or_zero();
            let up = right.cross(forward).normalize_or_zero();
            animated.refresh_visibility(dt.min(0.05), collision);
            animated.prepare_render(
                height,
                renderer.camera.target,
                camera_cull_radius(renderer.size, &renderer.camera),
                right,
                up,
                forward,
            );
            renderer.set_dynamic_mesh(&animated.vertices, &animated.indices);
            renderer.set_fx_mesh(&animated.fx.vertices, &animated.fx.indices);
            renderer.set_shadow_mesh(&animated.shadow_vertices, &animated.shadow_indices);
            renderer.set_caster_mesh(&animated.caster_vertices, &animated.caster_indices);
            renderer.set_lights(&animated.lights());
            renderer.set_time(self.app_time);
            renderer.set_vision(animated.player_pos(), 2600.0);
            if animated.visibility_dirty {
                renderer.set_vision_grid(
                    animated.vision_origin,
                    animated.visibility.width,
                    animated.visibility.height,
                    &animated.visibility.tiles,
                );
            }
            animated.visibility_dirty = false;
            if self.follow {
                renderer.camera.target = animated.player_pos() + Vec3::Y * 140.0;
                renderer.update_camera();
            }
        }

        self.hud
            .build_hero_select(w, h, &self.hero_select, &self.shop, self.mouse_pos);
        if let Some(renderer) = self.renderer.as_mut() {
            renderer.set_hud_mesh(&self.hud.vertices, &self.hud.indices);
        }
    }

    /// The main menu: the arena plays itself behind a slow orbiting camera.
    fn tick_menu(&mut self, dt: f32) {
        self.menu.time += dt;
        let (w, h) = self
            .renderer
            .as_ref()
            .map(|r| (r.size.0 as f32, r.size.1 as f32))
            .unwrap_or((1280.0, 720.0));
        self.menu.hover = menu::button_at(w, h, self.mouse_pos);
        if let Some(index) = self.menu.hover {
            self.menu.focus = index;
        }
        if let (Some(renderer), Some(animated), Some(height), Some(collision)) = (
            self.renderer.as_mut(),
            self.animated.as_mut(),
            self.height.as_ref(),
            self.collision.as_ref(),
        ) {
            // Attract mode: waves march and towers fire behind the menu.
            animated.update(dt.min(0.05), height, collision);
            animated.sound_cues.clear();
            let (lo, hi) = animated.minimap_bounds;
            let centre = (lo + hi) * 0.5;
            renderer.camera.target = Vec3::new(
                centre.x,
                height.height_at(centre.x, centre.y) + 160.0,
                centre.y,
            );
            renderer.camera.yaw = 0.6 + self.menu.time * 0.03;
            renderer.camera.pitch = 0.80;
            renderer.camera.distance = 7400.0;
            renderer.update_camera();
            let eye = renderer.camera.eye();
            let forward = (eye - renderer.camera.target).normalize_or_zero();
            let right = forward.cross(Vec3::Y).normalize_or_zero();
            let up = right.cross(forward).normalize_or_zero();
            animated.refresh_visibility(dt.min(0.05), collision);
            animated.prepare_render(
                height,
                renderer.camera.target,
                camera_cull_radius(renderer.size, &renderer.camera),
                right,
                up,
                forward,
            );
            renderer.set_dynamic_mesh(&animated.vertices, &animated.indices);
            renderer.set_fx_mesh(&animated.fx.vertices, &animated.fx.indices);
            renderer.set_shadow_mesh(&animated.shadow_vertices, &animated.shadow_indices);
            renderer.set_caster_mesh(&animated.caster_vertices, &animated.caster_indices);
            renderer.set_lights(&animated.lights());
            renderer.set_time(self.app_time);
            renderer.set_vision(renderer.camera.target, 40_000.0);
            renderer.clear_vision_grid();
            animated.visibility_dirty = false;
        }
        let version = concat!("V", env!("CARGO_PKG_VERSION"));
        self.hud.build_main_menu(w, h, &self.menu, version);
        if let Some(renderer) = self.renderer.as_mut() {
            renderer.set_hud_mesh(&self.hud.vertices, &self.hud.indices);
        }
    }

    /// Run a main-menu button.
    fn menu_activate(&mut self, action: menu::MenuAction, event_loop: &ActiveEventLoop) {
        match action {
            menu::MenuAction::Quit => event_loop.exit(),
            menu::MenuAction::Play(mode) => {
                tracing::info!("menu: {}", mode.label());
                self.play_ui_sound("skill_point");
                self.menu.open = false;
                self.hero_select.begin(mode);
                self.last_tick_second = None;
                // The attract-mode match is thrown away at lock in; keep the
                // world still behind the picker.
                if let Some(window) = self.window.as_ref() {
                    window.set_title(&format!("MobaScape - {} - Choose Your Hero", mode.label()));
                }
            }
        }
    }

    /// Leave hero select for the main menu (Esc).
    fn back_to_menu(&mut self) {
        self.hero_select.open = false;
        self.menu.open = true;
        if let Some(window) = self.window.as_ref() {
            window.set_title("MobaScape");
        }
    }

    /// Play a shared interface sound at the listener.
    fn play_ui_sound(&mut self, name: &str) {
        let Some(spec) = self.ui_sounds.get(name).copied() else {
            return;
        };
        let listener = audio::Listener {
            pos: Vec3::ZERO,
            right: Vec3::X,
        };
        let cue = audio::SoundCue {
            id: spec.id,
            pos: Vec3::ZERO,
            volume: spec.volume,
            speed: spec.speed,
        };
        self.audio.play_cues(&self.cache, &listener, [cue]);
    }

    /// Keep the spotlight on the highlighted hero and stream its frames into
    /// the HUD atlas.
    fn update_spotlight(&mut self, dt: f32) {
        let Some(champion) = self.hero_select.selected_champion().cloned() else {
            return;
        };
        if self.spotlight.showing() != Some(champion.id.as_str()) {
            let weapon = champion
                .weapon
                .as_deref()
                .and_then(|id| self.shop.items.iter().find(|item| item.id == id))
                .and_then(|item| item.obj);
            let parts = weapon.map(|obj| self.weapon_parts(obj)).unwrap_or_default();
            let started = Instant::now();
            if let Err(err) = self.spotlight.show(&self.cache, &champion, &parts) {
                tracing::warn!("spotlight {}: {err:#}", champion.id);
            }
            tracing::debug!(
                "spotlight: {} loaded in {:.1} ms",
                champion.id,
                started.elapsed().as_secs_f32() * 1000.0
            );
        }
        if let Some(pixels) = self.spotlight.update(dt)
            && let Some(renderer) = self.renderer.as_ref()
        {
            let (x, y, w, h) = ui_assets::SPOTLIGHT_RECT;
            renderer.write_hud_atlas(x, y, w, h, pixels);
        }
    }

    /// Rebuild the `--ui-live` theme with the picked hero's portrait.
    fn refresh_live_portrait(&mut self, champion: &champion::ChampionConfig) {
        let Some(inputs) = self.live_theme_inputs.as_mut() else {
            return;
        };
        if inputs.portrait_for.as_deref() == Some(champion.id.as_str()) {
            return;
        }
        let portrait = match portraits::bake_portrait(
            &self.cache,
            champion.model,
            &champion.attachments,
            champion.idle,
        ) {
            Ok(image) => image,
            Err(err) => {
                tracing::warn!("team portrait {}: {err:#}", champion.id);
                return;
            }
        };
        match theme_file::load_with_items_minimap_and_portrait(
            &self.cache,
            &inputs.path,
            &inputs.item_icons,
            Some(&inputs.minimap),
            Some(&portrait),
            &inputs.sprites,
        ) {
            Ok(theme) => {
                inputs.portrait_for = Some(champion.id.clone());
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.set_ui_atlas(&theme.atlas);
                }
                self.ui_theme = Some(theme);
                tracing::info!("team portrait: {} baked for the live frames", champion.id);
            }
            Err(err) => tracing::warn!("live theme rebuild: {err:#}"),
        }
    }

    /// Lock the pick in: the bots still choosing lock in too and hero select
    /// counts down to the match.
    fn lock_in(&mut self) {
        let index = self.hero_select.selected;
        let Some(champion) = self.hero_select.champions.get(index) else {
            return;
        };
        tracing::info!("hero select: locked in {} ({})", champion.name, champion.id);
        self.hero_select.lock_in();
        let (blue, red) = self.hero_select.lineups();
        let names = |team: &[usize]| {
            team.iter()
                .filter_map(|&i| self.hero_select.champions.get(i))
                .map(|c| c.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        };
        tracing::info!(
            "hero select: blue [{}] vs red [{}]",
            names(&blue),
            names(&red)
        );
        if let Some(window) = self.window.as_ref() {
            window.set_title("Rift - Game starting");
        }
    }

    /// Lock in the highlighted champion and start a clean match.
    fn start_match(&mut self) {
        let index = self.hero_select.selected;
        let Some(champion) = self.hero_select.champions.get(index).cloned() else {
            return;
        };
        self.refresh_live_portrait(&champion);
        let Some(scene) = self.scene.as_ref() else {
            return;
        };
        let mut animated = match AnimatedScene::new(&self.cache, scene, &champion) {
            Ok(animated) => animated,
            Err(err) => {
                tracing::error!("failed to start match as {}: {err:#}", champion.name);
                return;
            }
        };
        // The draft's other nine picks play as bots.
        let (blue, red) = self.hero_select.lineups();
        let (blue_lanes, red_lanes) = self.hero_select.lanes();
        let lineup = bot_lineup(
            &self.hero_select.champions,
            &blue,
            &red,
            (&blue_lanes, &red_lanes),
        );
        let builds = bot_builds(&self.shop, &lineup);
        let count = animated.spawn_bots(&self.cache, &lineup, &builds, &scene.height);
        tracing::info!("bots: {count} bot heroes in the match");

        self.audio.preload(&self.cache, animated.sound_ids());
        self.animated = Some(animated);
        self.hero_select.open = false;
        self.hero_select.hover = None;
        self.shop
            .recommend_for(&champion.id, &champion.role, &champion.class);
        self.shop.reset();
        self.event_log = vec![
            format!("{} READY", champion.name.to_uppercase()),
            "PRESS B FOR SHOP".to_string(),
            "WAVE INBOUND".to_string(),
        ];
        self.last_level = 1;
        self.last_buff_label = None;
        self.last_inhib0 = (3, 3);
        self.last_inhib1 = (3, 3);
        self.lane_log_timer = 0.0;
        self.title_timer = 0.0;
        self.title_frames = 0;
        self.app_time = 0.0;
        self.lmb_down = false;
        self.rmb_down = false;
        self.lmb_press = None;
        self.selected = None;
        self.held.clear();
        self.follow = true;
        self.last_frame = Instant::now();

        if let (Some(renderer), Some(animated)) = (self.renderer.as_mut(), self.animated.as_ref()) {
            renderer.camera.target = animated.player_pos() + Vec3::Y * 140.0;
            renderer.camera.distance = if self.demo_anim {
                780.0
            } else {
                self.initial_distance
            };
            if self.demo_anim {
                renderer.camera.pitch = 0.52;
            }
            renderer.update_camera();
        }
        if let Some(window) = self.window.as_ref() {
            window.set_title(&format!("Rift - Karamja | {}", champion.name));
        }
        tracing::info!(
            "hero select: locked in {} ({}) at tile {:.0},{:.0}",
            champion.name,
            champion.id,
            self.animated
                .as_ref()
                .map(|animated| animated.player_pos().x / 128.0)
                .unwrap_or(0.0),
            self.animated
                .as_ref()
                .map(|animated| animated.player_pos().z / 128.0)
                .unwrap_or(0.0)
        );
    }

    fn click_minimap(&mut self) -> bool {
        // The old HUD's minimap is not drawn under the live interface; its
        // rectangle must not catch clicks there.
        if self.ui_live {
            return false;
        }
        let Some(renderer) = self.renderer.as_mut() else {
            return false;
        };
        let Some(animated) = self.animated.as_ref() else {
            return false;
        };
        let Some(height) = self.height.as_ref() else {
            return false;
        };
        let Some((wx, wz)) = minimap_world_at(
            renderer.size.0 as f32,
            renderer.size.1 as f32,
            self.mouse_pos,
            animated.minimap_bounds,
        ) else {
            return false;
        };
        let wy = height.height_at(wx, wz) + 140.0;
        renderer.camera.target = Vec3::new(wx, wy, wz);
        self.follow = false;
        renderer.update_camera();
        tracing::info!("minimap pan to tile {:.0},{:.0}", wx / 128.0, wz / 128.0);
        true
    }

    fn purchase_item(&mut self, index: usize) {
        let Some(item) = self.shop.items.get(index).cloned() else {
            return;
        };
        let plan = self.shop.upgrade(index);
        let pending = self
            .animated
            .as_ref()
            .map(|animated| animated.delivery_count())
            .unwrap_or(0);
        // An upgrade frees the slots of the parts it uses up.
        if self.shop.item_count() + pending >= 6 + plan.consumed.len() {
            tracing::info!("shop: inventory full (including items in transit)");
            log_event(&mut self.event_log, "INVENTORY FULL".to_string());
            return;
        }
        let Some(animated) = self.animated.as_mut() else {
            return;
        };
        let stats = animated.player_stats();
        if stats.health <= 0.0 || stats.gold < plan.price {
            tracing::info!("shop: {} failed: dead or not enough gold", item.name);
            return;
        }
        // The parts leave the bag (and their stats come off) first.
        for &part in &plan.consumed {
            if let Some(config) = self.shop.items.get(part).cloned()
                && animated.refund_item(&config, 0).is_ok()
            {
                self.shop.remove_owned(part);
            }
        }
        let mut priced = item.clone();
        priced.cost = plan.price;
        let verb = if plan.consumed.is_empty() {
            "BOUGHT"
        } else {
            "UPGRADED"
        };
        match animated.buy_item(index, &priced) {
            Ok(outcome) => {
                let stored = outcome == BuyOutcome::Stored;
                if !stored {
                    self.shop.add_owned(index);
                }
                self.shop.undo.push(items::Purchase {
                    index,
                    stored,
                    paid: plan.price,
                    consumed: plan.consumed.clone(),
                });
                tracing::info!(
                    "shop: {} {} ({}) for {}g of {}g{} ({} slots used)",
                    verb.to_lowercase(),
                    item.name,
                    item.id,
                    plan.price,
                    item.cost,
                    if stored { ", stored in the chest" } else { "" },
                    self.shop.item_count()
                );
                let what = if stored { "STORED" } else { verb };
                log_event(&mut self.event_log, format!("{what} {}", item.name));
            }
            Err(err) => tracing::info!("shop: {} failed: {err}", item.name),
        }
    }

    fn sell_item(&mut self, index: usize) {
        let Some(item) = self.shop.items.get(index).cloned() else {
            return;
        };
        if !self.shop.is_owned(index) {
            return;
        }
        let Some(animated) = self.animated.as_mut() else {
            return;
        };
        match animated.sell_item(&item) {
            Ok(()) => {
                self.shop.remove_owned(index);
                self.shop.forget_purchase(index);
                tracing::info!(
                    "shop: sold {} for {}g ({} slots used)",
                    item.name,
                    items::sell_price(item.cost),
                    self.shop.item_count()
                );
                log_event(&mut self.event_log, format!("SOLD {}", item.name));
            }
            Err(err) => tracing::info!("shop: sell {} failed: {err}", item.name),
        }
    }

    /// Take back this visit's newest purchase for its full price.
    fn undo_purchase(&mut self) {
        let Some(purchase) = self.shop.undo.pop() else {
            return;
        };
        let Some(item) = self.shop.items.get(purchase.index).cloned() else {
            return;
        };
        let Some(animated) = self.animated.as_mut() else {
            return;
        };
        let undone = if purchase.stored {
            animated.cancel_stored(purchase.index)
        } else if self.shop.is_owned(purchase.index) {
            match animated.refund_item(&item, purchase.paid) {
                Ok(()) => self.shop.remove_owned(purchase.index),
                Err(err) => {
                    tracing::info!("shop: undo {} failed: {err}", item.name);
                    false
                }
            }
        } else {
            false
        };
        if undone {
            // An upgrade's parts come back into the bag (or as gold if the
            // bag has filled since).
            for &part in &purchase.consumed {
                let Some(config) = self.shop.items.get(part).cloned() else {
                    continue;
                };
                if self.shop.add_owned(part) {
                    animated.grant_item(&config);
                } else {
                    animated.debug_grant_gold(config.cost);
                }
            }
            tracing::info!("shop: undid {} (+{}g)", item.name, purchase.paid);
            log_event(&mut self.event_log, format!("UNDID {}", item.name));
        } else {
            tracing::info!("shop: nothing to undo for {}", item.name);
        }
    }

    fn champion_indices(&self) -> Vec<usize> {
        self.animated
            .as_ref()
            .map(|animated| {
                animated
                    .units
                    .iter()
                    .enumerate()
                    .filter(|(_, unit)| unit.is_champion)
                    .map(|(index, _)| index)
                    .collect()
            })
            .unwrap_or_default()
    }

    fn toggle_spectator(&mut self) {
        if self.spectator.is_some() {
            self.spectator = None;
            self.follow = true;
            return;
        }
        let champions = self.champion_indices();
        if let Some(&first) = champions.first() {
            self.spectator = Some(first);
            self.follow = false;
        }
    }

    fn cycle_spectator(&mut self, delta: i32) {
        let champions = self.champion_indices();
        if champions.is_empty() {
            return;
        }
        let current = self
            .spectator
            .and_then(|index| champions.iter().position(|&candidate| candidate == index));
        let next = match current {
            Some(position) => (position as i32 + delta).rem_euclid(champions.len() as i32) as usize,
            None => 0,
        };
        self.spectator = Some(champions[next]);
        self.follow = false;
    }

    /// Handle a click on the assembled interface when `--ui-live` is drawing
    /// it. Returns true when the click belonged to a control or panel, so the
    /// old HUD and the world do not also act on it.
    fn live_interface_click(&mut self, width: f32, height: f32) -> bool {
        if !self.ui_live {
            return false;
        }
        let window = (width.max(1.0) as u32, height.max(1.0) as u32);
        let placed = match self.ui_placed.as_ref() {
            Some(placed) if placed.window == window => placed.clone(),
            _ => {
                let state = rift_ui::hud::sample::whole_for(window);
                rift_ui::hud::place(&state, window, self.ui_scale)
            }
        };
        let scale = placed.scale as i32;
        let mouse = (self.mouse_pos.0 as i32, self.mouse_pos.1 as i32);

        if self.shop.open {
            // Any click ends typing; a click on the box starts it again.
            self.shop.typing = false;
            let area = rift_ui::hud::shop::rect(
                rift_ui::draw::Rect::new(0, 0, window.0 as i32, window.1 as i32),
                scale,
            );
            let mut live = crate::ui_live::shop(&self.shop);
            live.delivery = self.animated.as_ref().and_then(|animated| {
                let state = match animated.delivery_state {
                    crate::animated::DeliveryState::Idle => 0,
                    crate::animated::DeliveryState::FlyingToHero => 1,
                    crate::animated::DeliveryState::Returning => 2,
                };
                crate::ui_live::delivery_label(animated.delivery_count(), state)
            });
            if let Some(hit) = rift_ui::hud::shop::hit(area, scale, &live, mouse) {
                match hit {
                    rift_ui::hud::shop::ShopHit::Category(index) => {
                        if index < crate::items::CATEGORY_LABELS.len() {
                            self.shop.category = index;
                            self.shop.selected = self.shop.filtered_items().first().copied();
                            self.shop.scroll = 0;
                        }
                    }
                    rift_ui::hud::shop::ShopHit::Row(index) => {
                        if let Some(&item) = self.shop.filtered_items().get(index) {
                            self.shop.selected = Some(item);
                        }
                    }
                    rift_ui::hud::shop::ShopHit::Buy => {
                        if let Some(item) = self.shop.shown() {
                            self.purchase_item(item);
                        }
                    }
                    rift_ui::hud::shop::ShopHit::Search => self.shop.typing = true,
                    rift_ui::hud::shop::ShopHit::Sell => {
                        if let Some(item) = self.shop.shown() {
                            self.sell_item(item);
                        }
                    }
                    rift_ui::hud::shop::ShopHit::Undo => self.undo_purchase(),
                    rift_ui::hud::shop::ShopHit::Deliver => {
                        if let Some(animated) = self.animated.as_mut()
                            && animated.call_delivery()
                        {
                            log_event(
                                &mut self.event_log,
                                "ARMADYLEAN GUARD DISPATCHED".to_string(),
                            );
                            tracing::info!("delivery: guard dispatched from the live shop");
                        }
                    }
                    rift_ui::hud::shop::ShopHit::Close => self.shop.open = false,
                    rift_ui::hud::shop::ShopHit::Bag(slot) => {
                        if let Some(item) = self.shop.owned.get(slot).copied().flatten() {
                            // Show it, switching to All when the current
                            // category or search hides it, so Sell is one
                            // click away.
                            if !self.shop.filtered_items().contains(&item) {
                                self.shop.category = 1;
                                self.shop.query.clear();
                                self.shop.scroll = 0;
                            }
                            self.shop.selected = Some(item);
                        }
                    }
                }
                return true;
            }
            let (mx, my) = mouse;
            return mx >= area.x && mx < area.right() && my >= area.y && my < area.bottom();
        }

        if let Some(area) = placed.get(rift_ui::hud::Piece::Inventory)
            && rift_ui::hud::inventory::shop_button_at(area, scale, mouse)
        {
            self.shop.open = true;
            self.shop.selected = self.shop.filtered_items().first().copied();
            self.shop.scroll = 0;
            tracing::info!("shop: opened from the assembled inventory");
            return true;
        }
        // The minimap pans the camera to the clicked point, like the old HUD.
        if let Some(area) = placed.get(rift_ui::hud::Piece::Minimap)
            && let Some((u, v)) = rift_ui::hud::minimap::picture_at(area, scale, mouse)
        {
            self.pan_camera_to_minimap(u, v);
            return true;
        }
        // Ability slots cast like their hotkeys.
        if let Some(area) = placed.get(rift_ui::hud::Piece::Ability)
            && let Some(slot) = rift_ui::hud::ability_bar::ability_at(area, scale, mouse)
        {
            tracing::info!("ability bar: slot {slot} clicked");
            // With a skill point to spend, a click ranks the ability up
            // instead of casting it.
            if let Some(animated) = self.animated.as_mut()
                && animated.upgrade_ability(slot)
            {
                return true;
            }
            match slot {
                0 => self.cast_q(),
                1 => self.cast_w(),
                2 => self.cast_e(),
                _ => self.cast_r(),
            }
            return true;
        }
        // Every other click on a panel belongs to the interface: it must not
        // walk the hero to the ground hidden behind the panel.
        placed.pieces.values().any(|area| {
            mouse.0 >= area.x
                && mouse.0 < area.right()
                && mouse.1 >= area.y
                && mouse.1 < area.bottom()
        })
    }

    /// Centre the camera on the world point under a minimap fraction. The
    /// inverse of `ui_live::map_point`, over the same minimap bounds.
    fn pan_camera_to_minimap(&mut self, u: f32, v: f32) {
        let (Some(renderer), Some(animated), Some(height)) = (
            self.renderer.as_mut(),
            self.animated.as_ref(),
            self.height.as_ref(),
        ) else {
            return;
        };
        let (min, max) = animated.minimap_bounds;
        let (wx, wz) = crate::ui_live::world_from_map(u, v, min, max);
        renderer.camera.target = Vec3::new(wx, height.height_at(wx, wz) + 140.0, wz);
        self.follow = false;
        renderer.update_camera();
        tracing::info!(
            "live minimap pan to tile {:.0},{:.0}",
            wx / 128.0,
            wz / 128.0
        );
    }

    fn pick_and_move(&mut self) {
        if self.spectator.is_some() {
            return;
        }
        let picked = {
            let (Some(renderer), Some(height), Some(collision), Some(animated)) = (
                self.renderer.as_ref(),
                self.height.as_ref(),
                self.collision.as_ref(),
                self.animated.as_mut(),
            ) else {
                return;
            };
            let mut picked = None;
            if let Some((origin, dir)) = renderer.screen_ray(self.mouse_pos.0, self.mouse_pos.1)
                && let Some(index) = animated.pick_unit(origin, dir)
            {
                let ok = animated.set_attack_target(index, collision, height);
                println!("ATTACK target={index} ok={ok}");
                picked = Some(index);
            } else if let Some(point) =
                renderer.pick_ground(self.mouse_pos.0, self.mouse_pos.1, height)
            {
                let waypoints = animated.set_player_target(point, collision, height);
                println!(
                    "MOVE target={:.0},{:.0},{:.0} path={} waypoints",
                    point.x, point.y, point.z, waypoints
                );
            }
            picked
        };
        self.selected = picked;
    }

    fn cast_q(&mut self) {
        let (Some(renderer), Some(height), Some(animated)) = (
            self.renderer.as_ref(),
            self.height.as_ref(),
            self.animated.as_mut(),
        ) else {
            return;
        };
        let fallback = animated.player_pos();
        let point = renderer
            .pick_ground(self.mouse_pos.0, self.mouse_pos.1, height)
            .unwrap_or(fallback);
        let cast = animated.cast_q(point);
        println!("Q cast={cast}");
    }

    fn cast_w(&mut self) {
        let (Some(renderer), Some(height), Some(collision), Some(animated)) = (
            self.renderer.as_ref(),
            self.height.as_ref(),
            self.collision.as_ref(),
            self.animated.as_mut(),
        ) else {
            return;
        };
        if let Some(point) = renderer.pick_ground(self.mouse_pos.0, self.mouse_pos.1, height) {
            let cast = animated.cast_w(point, collision, height);
            println!("W cast={cast}");
        }
    }

    fn cast_e(&mut self) {
        let (Some(renderer), Some(height), Some(collision), Some(animated)) = (
            self.renderer.as_ref(),
            self.height.as_ref(),
            self.collision.as_ref(),
            self.animated.as_mut(),
        ) else {
            return;
        };
        let fallback = animated.player_pos();
        animated.kit_aim = renderer
            .pick_ground(self.mouse_pos.0, self.mouse_pos.1, height)
            .unwrap_or(fallback);
        let cast = animated.cast_e(collision, height);
        println!("E cast={cast}");
    }

    fn cast_r(&mut self) {
        let (Some(renderer), Some(height), Some(animated)) = (
            self.renderer.as_ref(),
            self.height.as_ref(),
            self.animated.as_mut(),
        ) else {
            return;
        };
        if let Some((origin, dir)) = renderer.screen_ray(self.mouse_pos.0, self.mouse_pos.1) {
            let fallback = animated.player_pos();
            let point = renderer
                .pick_ground(self.mouse_pos.0, self.mouse_pos.1, height)
                .unwrap_or_else(|| {
                    animated
                        .pick_enemy_champion(origin, dir)
                        .and_then(|index| animated.unit_pos(index))
                        .unwrap_or(fallback)
                });
            let cast = animated.cast_r(point);
            println!("R cast={cast}");
        }
    }

    fn update_camera(&mut self, dt: f32) {
        if self.demo_base {
            self.demo_base_timer += dt;
            if self.demo_base_stage == 0 && self.demo_base_timer >= 0.5 {
                if let Some(animated) = self.animated.as_mut() {
                    animated.debug_damage_player(300.0);
                    let s = animated.player_stats();
                    tracing::info!(
                        "demo base: damaged to {:.0}/{:.0} hp, {:.0}/{:.0} mp, gold {}",
                        s.health,
                        s.max_health,
                        s.mana,
                        s.max_mana,
                        s.gold
                    );
                }
                self.demo_base_stage = 1;
            }
            if self.demo_base_stage == 1 && self.demo_base_timer >= 4.0 {
                if let Some(animated) = self.animated.as_ref() {
                    let s = animated.player_stats();
                    tracing::info!(
                        "demo base: fountain healed to {:.0}/{:.0} hp, {:.0}/{:.0} mp, passive gold {}",
                        s.health,
                        s.max_health,
                        s.mana,
                        s.max_mana,
                        s.gold
                    );
                }
                self.demo_base = false;
            }
        }

        if self.demo_inhibitor {
            self.demo_inhibitor_timer += dt;
            if self.demo_inhibitor_stage == 0 && self.demo_inhibitor_timer >= 0.5 {
                if let Some(animated) = self.animated.as_mut() {
                    let ok = animated.debug_destroy_inhibitor(1, 0, 30.0);
                    tracing::info!(
                        "demo inhibitor: destroyed={ok} state={:?} inhib1={:?} vulnerable1={} supers={}",
                        animated.debug_inhibitor_state(1, 0),
                        animated.inhibitor_count(1),
                        animated.nexus_vulnerable(1),
                        animated.super_minion_count()
                    );
                }
                self.demo_inhibitor_stage = 1;
            }
            if self.demo_inhibitor_stage == 1 && self.demo_inhibitor_timer >= 2.0 {
                if let Some(animated) = self.animated.as_ref() {
                    tracing::info!(
                        "demo inhibitor: after wave state={:?} inhib1={:?} vulnerable1={} supers={}",
                        animated.debug_inhibitor_state(1, 0),
                        animated.inhibitor_count(1),
                        animated.nexus_vulnerable(1),
                        animated.super_minion_count()
                    );
                }
                self.demo_inhibitor_stage = 2;
            }
            if self.demo_inhibitor_stage == 2 && self.demo_inhibitor_timer >= 5.5 {
                if let Some(animated) = self.animated.as_ref() {
                    tracing::info!(
                        "demo inhibitor: after respawn state={:?} inhib1={:?} vulnerable1={} supers={}",
                        animated.debug_inhibitor_state(1, 0),
                        animated.inhibitor_count(1),
                        animated.nexus_vulnerable(1),
                        animated.super_minion_count()
                    );
                }
                self.demo_inhibitor = false;
            }
        }

        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };

        let yaw = renderer.camera.yaw;
        let forward = Vec3::new(-yaw.sin(), 0.0, -yaw.cos());
        let right = Vec3::new(yaw.cos(), 0.0, -yaw.sin());

        let mut movement = Vec3::ZERO;
        if self.held.contains(&KeyCode::ArrowUp) {
            movement += forward;
        }
        if self.held.contains(&KeyCode::ArrowDown) {
            movement -= forward;
        }
        if self.held.contains(&KeyCode::ArrowRight) {
            movement += right;
        }
        if self.held.contains(&KeyCode::ArrowLeft) {
            movement -= right;
        }

        let mut changed = false;
        if movement.length_squared() > 0.0 {
            self.follow = false;
            let speed = 2600.0 * dt;
            renderer.camera.target += movement.normalize() * speed;
            changed = true;
        }
        if self.held.contains(&KeyCode::KeyZ) {
            renderer.camera.yaw += 1.2 * dt;
            changed = true;
        }
        if self.held.contains(&KeyCode::KeyX) {
            renderer.camera.yaw -= 1.2 * dt;
            changed = true;
        }

        if changed {
            renderer.update_camera();
        }
    }
}

/// Where the screen's four corners meet the ground plane under the camera
/// focus (top left, top right, bottom right, bottom left). Corners looking
/// at or above the horizon are cut off at a far distance.
fn screen_footprint(renderer: &Renderer) -> Option<[Vec3; 4]> {
    let (w, h) = (renderer.size.0 as f64, renderer.size.1 as f64);
    let ground = renderer.camera.target.y;
    let corner = |x: f64, y: f64| -> Option<Vec3> {
        let (origin, dir) = renderer.screen_ray(x, y)?;
        let far = 60_000.0;
        if dir.y > -1e-3 {
            return Some(origin + dir * far);
        }
        let t = ((ground - origin.y) / dir.y).min(far);
        Some(origin + dir * t)
    };
    Some([
        corner(0.0, 0.0)?,
        corner(w, 0.0)?,
        corner(w, h)?,
        corner(0.0, h)?,
    ])
}

/// Build the stage 3 interface for this window and hand it to the renderer's
/// interface pass, when its theme was asked for.
#[allow(clippy::too_many_arguments)]
fn draw_ui_overlay(
    renderer: &mut Renderer,
    theme: Option<&rift_ui::Theme>,
    theme_sheet: bool,
    layout: bool,
    live: bool,
    scale: Option<u32>,
    shown_cache: &mut (u32, u32, u32),
    player: Option<&PlayerHud>,
    animated: Option<&AnimatedScene>,
    scene: Option<&SceneData>,
    shop: Option<&ItemShop>,
    camera: (f32, f32, f32, f32),
    mouse: (i32, i32),
    selected: Option<usize>,
) -> Option<rift_ui::hud::Layout> {
    let theme = theme?;
    let mut placed_out = None;
    let (list, what, detail) = if layout || live {
        let mut state = rift_ui::hud::sample::whole_for(renderer.size);
        if live {
            state.tooltip = None;
            state.target = None;
            if let Some(player) = player {
                state.score = crate::ui_live::score(player);
                state.ability = crate::ui_live::ability(player);
                state.plates = crate::ui_live::plates(player);
                state.hitmarks = crate::ui_live::hitmarks(player);
                state.overhead = crate::ui_live::overhead(player);
                state.chat = crate::ui_live::chat(player);
                state.feed = crate::ui_live::feed(player);
                state.announcement = crate::ui_live::announcement(player);
                state.gold = player.gold;
            }
            if let Some(animated) = animated {
                crate::ui_live::ability_ranks(&mut state.ability, animated);
            }
            if let (Some(player), Some(animated)) = (player, animated) {
                state.death = crate::ui_live::death(player, animated);
                state.allies = crate::ui_live::allies(animated);
                state.target = crate::ui_live::target(animated, selected);
            }
            if let (Some(animated), Some(scene)) = (animated, scene) {
                let footprint = screen_footprint(renderer);
                state.minimap =
                    crate::ui_live::minimap(scene, animated, camera, renderer.size, footprint);
                state.objectives = crate::ui_live::objectives(scene, animated);
            }
            if let Some(shop) = shop {
                state.shop = crate::ui_live::shop(shop);
                state.shop.delivery = player.and_then(|player| {
                    crate::ui_live::delivery_label(player.delivery_pending, player.delivery_state)
                });
                state.tooltip = crate::ui_live::tooltip(shop, mouse);
                state.inventory = crate::ui_live::inventory(shop);
            }
        }
        let placed = rift_ui::hud::place(&state, renderer.size, scale);
        placed_out = Some(placed.clone());
        // Hovering Q, W, E or R explains the ability and its ranks.
        if live
            && state.death.is_none()
            && let (Some(animated), Some(area)) =
                (animated, placed.get(rift_ui::hud::Piece::Ability))
            && let Some(slot) =
                rift_ui::hud::ability_bar::ability_at(area, placed.scale as i32, mouse)
        {
            // Anchor above the bar, centred on the slot, so it never covers
            // the slots themselves.
            let scale = placed.scale as i32;
            let anchor = (area.x + (92 + slot as i32 * 42 + 20) * scale, area.y);
            state.ability_tooltip = crate::ui_live::ability_tooltip(animated, slot, anchor);
        }
        let detail = format!(
            "requested {:?}, drawn {}, usual {:.1}%, steps {:?}, conflicts {:?}",
            placed.requested,
            placed.scale,
            placed.usual_coverage * 100.0,
            placed.steps,
            placed.conflicts
        );
        (
            rift_ui::hud::stage3::build(theme, &placed, &state),
            if live {
                "interface live"
            } else {
                "interface layout"
            },
            detail,
        )
    } else if theme_sheet {
        (
            rift_ui::theme_sheet::build(theme, renderer.size, scale),
            "interface theme sheet",
            String::new(),
        )
    } else {
        (
            rift_ui::test_screen::build(theme, renderer.size, scale),
            "interface test",
            String::new(),
        )
    };
    let shown = (renderer.size.0, renderer.size.1, list.scale() as u32);
    if shown != *shown_cache {
        tracing::info!(
            "{}: window {} x {}, scale {}, {} quads {}",
            what,
            shown.0,
            shown.1,
            shown.2,
            list.len(),
            detail
        );
        *shown_cache = shown;
    }
    let (vertices, indices) = list.finish();
    renderer.set_ui_mesh(&vertices, &indices);
    placed_out
}

/// Dev demos, dev screens and explicit spawn requests boot straight into the
/// match instead of stopping at the main menu. `--ui-live` is not one: it
/// only picks the in-match interface, and the AppImage passes it on every
/// launch.
fn has_dev_demo_flags(cli: &Cli) -> bool {
    cli.ui_test
        || cli.ui_theme_sheet
        || cli.ui_layout
        || cli.demo_target.is_some()
        || cli.spawn.is_some()
        || cli.shots.is_some()
        || cli.demo_combat
        || cli.demo_fx
        || cli.demo_hitsplats
        || cli.demo_anim
        || cli.demo_duel
        || cli.demo_awowogei
        || cli.demo_camp
        || cli.demo_ranks.is_some()
        || cli.demo_watchtower
        || cli.demo_delivery
        || cli.demo_kit
        || cli.demo_shop
        || cli.demo_base
        || cli.demo_inhibitor
}

fn log_event(events: &mut Vec<String>, message: String) {
    events.insert(0, message);
    events.truncate(5);
}

fn parse_window_size(text: &str) -> Option<(u32, u32)> {
    let lower = text.to_ascii_lowercase();
    let (width, height) = lower.split_once('x')?;
    let width = width.parse().ok()?;
    let height = height.parse().ok()?;
    (width > 0 && height > 0).then_some((width, height))
}

/// Radius around the camera focus that can contain visible dynamic units at
/// this camera distance and pitch, plus a margin for tall models and shadow
/// casters beyond the ground view.
fn camera_cull_radius(size: (u32, u32), camera: &Camera) -> f32 {
    let aspect = size.0 as f32 / size.1.max(1) as f32;
    let pitch = camera.pitch.max(0.2);
    let half_w = camera.distance * 0.466 * aspect;
    let half_h = camera.distance * 0.466 / pitch.sin();
    (half_w.hypot(half_h) + 2000.0).clamp(5000.0, 14_000.0)
}

fn resolve_render_scale(text: &str, window: (u32, u32)) -> Option<f32> {
    if text.eq_ignore_ascii_case("auto") {
        let pixels = (window.0 as f32 * window.1 as f32).max(1.0);
        Some((1_600_000.0 / pixels).sqrt().clamp(0.62, 1.0))
    } else {
        let scale: f32 = text.parse().ok()?;
        (0.5..=1.0).contains(&scale).then_some(scale)
    }
}

fn parse_present_mode(name: &str) -> wgpu::PresentMode {
    match name.to_ascii_lowercase().as_str() {
        "immediate" => wgpu::PresentMode::Immediate,
        "mailbox" => wgpu::PresentMode::Mailbox,
        _ => wgpu::PresentMode::Fifo,
    }
}

fn parse_shadow_mode(name: &str) -> u32 {
    match name.to_ascii_lowercase().as_str() {
        "off" => 0,
        "blob" => 1,
        "map" => 2,
        _ => 2,
    }
}

fn parse_debug_view(name: &str) -> u32 {
    devtools::debug_view_index(name)
}

fn digit_index(code: KeyCode) -> Option<usize> {
    match code {
        KeyCode::Digit1 => Some(0),
        KeyCode::Digit2 => Some(1),
        KeyCode::Digit3 => Some(2),
        KeyCode::Digit4 => Some(3),
        KeyCode::Digit5 => Some(4),
        KeyCode::Digit6 => Some(5),
        _ => None,
    }
}

fn print_camera(renderer: &Renderer) {
    let target = renderer.camera.target;
    println!(
        "CAM target={:.0},{:.0},{:.0} yaw={:.2} pitch={:.2} dist={:.0}",
        target.x,
        target.y,
        target.z,
        renderer.camera.yaw,
        renderer.camera.pitch,
        renderer.camera.distance
    );
}

/// The draft as bots: blue's picks after the player's own, then red's.
/// Each bot plays the lane it called on hero select (`lanes` runs in the
/// same order as the picks, the player first on blue); a row without a call
/// falls back to the draft-order role.
fn bot_lineup(
    champions: &[ChampionConfig],
    blue: &[usize],
    red: &[usize],
    lanes: (&[Option<hero_select::Lane>], &[Option<hero_select::Lane>]),
) -> Vec<(ChampionConfig, u8, animated::bots::BotRole)> {
    let mut out = Vec::new();
    let blue_calls = lanes.0.get(1..).unwrap_or(&[]);
    for (team, picks, calls) in [
        (0u8, blue.get(1..).unwrap_or(&[]), blue_calls),
        (1u8, red, lanes.1),
    ] {
        let roles = animated::bots::roles_for(team, picks.len());
        for (row, (&pick, fallback)) in picks.iter().zip(roles).enumerate() {
            let role = calls
                .get(row)
                .copied()
                .flatten()
                .map(hero_select::Lane::bot_role)
                .unwrap_or(fallback);
            if let Some(champion) = champions.get(pick) {
                out.push((champion.clone(), team, role));
            }
        }
    }
    out
}

/// Each bot's item build, from the shop's recommended lists.
fn bot_builds(
    shop: &ItemShop,
    lineup: &[(ChampionConfig, u8, animated::bots::BotRole)],
) -> Vec<Vec<items::ItemConfig>> {
    lineup
        .iter()
        .map(|(c, _, _)| shop.build_for(&c.id, &c.role, &c.class))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_sizes_parse_only_positive_width_and_height() {
        assert_eq!(parse_window_size("1280x720"), Some((1280, 720)));
        assert_eq!(parse_window_size("1920X1080"), Some((1920, 1080)));
        assert_eq!(parse_window_size("0x720"), None);
        assert_eq!(parse_window_size("1280x0"), None);
        assert_eq!(parse_window_size("1280"), None);
        assert_eq!(parse_window_size("wide x high"), None);

        assert_eq!(resolve_render_scale("auto", (1280, 720)), Some(1.0));
        let scale = resolve_render_scale("auto", (1920, 1080)).unwrap();
        assert!((scale - 0.878).abs() < 0.01, "{scale}");
        assert_eq!(resolve_render_scale("0.85", (1920, 1080)), Some(0.85));
        assert_eq!(resolve_render_scale("0.4", (1920, 1080)), None);
        assert_eq!(resolve_render_scale("1.2", (1920, 1080)), None);
    }

    #[test]
    fn bots_play_the_lanes_they_called() {
        use animated::bots::BotRole;
        use hero_select::Lane;
        let champions: Vec<ChampionConfig> = (0..10)
            .map(|i| hero_select::tests::champion(&format!("h{i}")))
            .collect();
        let blue: Vec<usize> = (0..5).collect();
        let red: Vec<usize> = (5..10).collect();
        // You call jungle: blue's bots take top, mid and bottom.
        let blue_calls: Vec<Option<Lane>> = std::iter::once(Some(Lane::Jungle))
            .chain(hero_select::bot_lanes(Some(Lane::Jungle)).map(Some))
            .collect();
        let red_calls = hero_select::RED_LANES.map(Some);
        let lineup = bot_lineup(&champions, &blue, &red, (&blue_calls, &red_calls));
        let roles: Vec<(u8, BotRole)> = lineup.iter().map(|(_, t, r)| (*t, *r)).collect();
        assert_eq!(
            &roles[..4],
            &[
                (0, BotRole::Lane(0)),
                (0, BotRole::Lane(1)),
                (0, BotRole::Lane(2)),
                (0, BotRole::Lane(2)),
            ]
        );
        assert!(
            !roles[..4].contains(&(0, BotRole::Jungle)),
            "the jungle is yours"
        );
        // A row with no call falls back to its draft-order role.
        let lineup = bot_lineup(&champions, &blue, &red, (&[], &[]));
        let fallback: Vec<BotRole> = lineup.iter().skip(4).map(|(_, _, r)| *r).collect();
        assert_eq!(fallback, animated::bots::roles_for(1, 5));
    }
}
