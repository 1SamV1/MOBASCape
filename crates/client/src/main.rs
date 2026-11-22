mod animated;
mod champion;
mod font;
mod grass_textures;
mod hero_select;
mod hud;
mod item_icons;
mod items;
mod minimap;
mod model_textures;
mod motion;
mod portraits;
mod renderer;
mod scene;
mod terrain_textures;
mod theme_file;
mod ui_assets;
mod ui_live;
mod ui_pass;

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use clap::Parser;
use glam::Vec3;
use rift_cache::Cache;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

use crate::animated::{AnimatedScene, BuyOutcome, UnitKind};
use crate::champion::ChampionConfig;
use crate::hero_select::HeroSelect;
use crate::hud::{HeroMarker, HoverInfo, Hud, PlayerHud, minimap_world_at};
use crate::items::{
    ItemShop, delivery_button_at, point_in_rect, shop_button_at, shop_buy_at, shop_card_at,
    shop_card_rect, shop_category_at, shop_close_at, shop_sell_at, shop_window_rect,
};
use crate::renderer::{Camera, Renderer};
use crate::scene::{CampBuff, HeightGrid, SceneData, WorldCollision};

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
    /// Scene render scale: `auto` targets 1.2M scene pixels, or a factor
    /// from 0.5 to 1.0. Post and HUD stay at native window resolution.
    #[arg(long, default_value = "auto")]
    render_scale: String,
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
    /// Demo: spawn a sparring enemy champion beside the hero for Trial by Combat.
    #[arg(long)]
    demo_duel: bool,
    /// Demo: spawn a rival champion and automatically cast Trial by Combat.
    #[arg(long)]
    demo_trial: bool,
    /// Demo: pop floating damage numbers, a kill shout and a minimap ping.
    #[arg(long)]
    demo_fx: bool,
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
    /// Dev: write a PNG of the presented frame, then exit.
    #[arg(long)]
    screenshot: Option<PathBuf>,
    /// Dev: frames to draw before `--screenshot` captures (default 90).
    #[arg(long, default_value_t = 90)]
    screenshot_after: u32,
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
    let window_size = parse_window_size(&cli.window_size)
        .with_context(|| format!("window size {:?} (expected WIDTHxHEIGHT)", cli.window_size))?;
    let render_scale = resolve_render_scale(&cli.render_scale, window_size).with_context(|| {
        format!(
            "render scale {:?} (expected auto or 0.5..1.0)",
            cli.render_scale
        )
    })?;
    let render_scale_auto = cli.render_scale.eq_ignore_ascii_case("auto");
    let cache = Cache::open_with_keys(&cli.cache, None::<&PathBuf>)
        .with_context(|| format!("opening cache at {}", cli.cache.display()))?;

    tracing::info!(
        "building {0}x{0} region block around {1},{2}",
        cli.radius * 2 + 1,
        cli.region_x,
        cli.region_y
    );
    let mut champions = ChampionConfig::load_all()?;
    let requested_champion = if cli.demo_anim {
        Some("monkey")
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
    let hero_select_open = cli.champion.is_none() && !cli.skip_hero_select && !has_demo_flags(&cli);
    let shop = ItemShop::load()?;
    let scene = scene::build(&cache, cli.region_x, cli.region_y, cli.radius, &cli.map)?;
    let animated = AnimatedScene::new(&cache, &scene, &champion)?;
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
    if cli.demo_duel || cli.demo_trial {
        let spawn = animated.player_pos() + Vec3::new(3.0 * 128.0, 0.0, 0.0);
        let spawn = Vec3::new(spawn.x, height.height_at(spawn.x, spawn.z), spawn.z);
        match animated.spawn_enemy_champion(&cache, &champion, spawn) {
            Ok(index) => {
                if cli.demo_trial {
                    // Leave the rival wounded so the automated demo can
                    // actually resolve the duel and prove the win bonus.
                    let unit = &mut animated.units[index];
                    unit.health = (unit.max_health * 0.25).max(1.0);
                    tracing::info!("demo trial: rival starts at {:.0} HP", unit.health);
                }
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

    let mut ui_atlas = ui_assets::UiAtlas::load()?;
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
    }
    // Bake a real terrain map for the minimap from the scene colours.
    let minimap_image = minimap::render(&scene, animated.minimap_bounds, minimap::MINIMAP_SIZE);
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
    let ui_theme = if cli.ui_test || cli.ui_theme_sheet || cli.ui_layout || cli.ui_live {
        let theme = theme_file::load_with_items(&cache, &cli.ui_theme, &item_icons)
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
    let mut app = App {
        cache,
        scene: Some(scene),
        animated: Some(animated),
        height: Some(height),
        collision: Some(collision),
        hero_select: HeroSelect::new(champions, selected_champion, hero_select_open),
        lobby_open: false,
        lobby_deadline: None,
        window: None,
        window_size,
        render_scale,
        render_scale_auto,
        renderer: None,
        held: HashSet::new(),
        focused: false,
        follow: true,
        mouse_pos: (640.0, 360.0),
        lmb_down: false,
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
        demo_trial: cli.demo_trial,
        demo_trial_timer: 0.0,
        demo_trial_stage: 0,
        demo_fx: cli.demo_fx,
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
        screenshot_request: cli
            .screenshot
            .clone()
            .map(|path| (path, cli.screenshot_after)),
        app_time: 0.0,
        lane_log_timer: 0.0,
        last_frame: Instant::now(),
        title_timer: 0.0,
        title_frames: 0,
    };
    event_loop.run_app(&mut app)?;
    Ok(())
}

struct App {
    cache: Cache,
    scene: Option<SceneData>,
    animated: Option<AnimatedScene>,
    height: Option<HeightGrid>,
    collision: Option<WorldCollision>,
    hero_select: HeroSelect,
    /// The 5v5 lobby shown between the hero pick and the match.
    lobby_open: bool,
    /// Wall-clock deadline for the lobby auto-start, so the countdown keeps
    /// real time even when the frame rate drops.
    lobby_deadline: Option<Instant>,
    window: Option<Arc<Window>>,
    window_size: (u32, u32),
    render_scale: f32,
    render_scale_auto: bool,
    renderer: Option<Renderer>,
    held: HashSet<KeyCode>,
    focused: bool,
    follow: bool,
    mouse_pos: (f64, f64),
    lmb_down: bool,
    lmb_press: Option<Instant>,
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
    demo_trial: bool,
    demo_trial_timer: f32,
    demo_trial_stage: u8,
    demo_fx: bool,
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
    screenshot_request: Option<(std::path::PathBuf, u32)>,
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
        let model_textures = match crate::model_textures::ModelTextures::load(&self.cache) {
            Ok(textures) => textures,
            Err(err) => {
                tracing::error!("failed to load model textures: {err:#}");
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
            model_textures,
            self.hud.atlas.clone(),
        )) {
            Ok(renderer) => renderer,
            Err(err) => {
                tracing::error!("failed to create renderer: {err:#}");
                event_loop.exit();
                return;
            }
        };

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
        }
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
                if button != MouseButton::Left {
                    return;
                }
                let (w, h) = self
                    .renderer
                    .as_ref()
                    .map(|r| (r.size.0 as f32, r.size.1 as f32))
                    .unwrap_or((1280.0, 720.0));
                if self.lobby_open {
                    if state == ElementState::Pressed
                        && hero_select::lobby_button_at(w, h, self.mouse_pos)
                    {
                        self.start_match();
                    }
                    return;
                }
                if self.hero_select.open {
                    if state == ElementState::Pressed {
                        if let Some(filter) = hero_select::filter_at(w, h, self.mouse_pos) {
                            self.hero_select.set_filter(filter);
                        } else if let Some(slot) = hero_select::card_at(
                            w,
                            h,
                            self.mouse_pos,
                            self.hero_select.visible.len(),
                        ) {
                            self.hero_select.select_slot(slot);
                        } else if hero_select::confirm_at(w, h, self.mouse_pos) {
                            self.open_lobby();
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
                        if let Some(item) = self.shop.selected {
                            self.purchase_item(item);
                        }
                        return;
                    }
                    if shop_sell_at(w, h, self.mouse_pos) {
                        if let Some(item) = self.shop.selected {
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

                if self.lobby_open {
                    if event.state == ElementState::Pressed {
                        match code {
                            KeyCode::Escape => event_loop.exit(),
                            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
                                self.start_match()
                            }
                            _ => {}
                        }
                    }
                    return;
                }
                if self.hero_select.open {
                    if event.state == ElementState::Pressed {
                        match code {
                            KeyCode::Escape => event_loop.exit(),
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
                            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
                                self.open_lobby()
                            }
                            _ => {}
                        }
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
                    let rows = self.shop.filtered_items().len() as i32;
                    let max = (rows * 44 - 398).max(0);
                    let step = (amount * 44.0).round() as i32;
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
    fn tick(&mut self) {
        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f32().min(0.1);
        self.last_frame = now;
        self.app_time += dt;
        let profiling = self.benchmark_remaining.is_some() && self.app_time > 1.0;
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
                self.benchmark_remaining = None;
                self.benchmark_done = true;
            }
        }

        if self.hero_select.open {
            if self.renderer.is_some() {
                self.tick_hero_select(dt);
            }
            return;
        }
        if self.lobby_open {
            if self.renderer.is_some() {
                self.tick_lobby(dt);
            }
            return;
        }

        if self.demo_shop {
            self.demo_shop_timer += dt;
            if self.demo_shop_stage == 0 && self.demo_shop_timer >= 0.5 {
                if let Some(animated) = self.animated.as_mut() {
                    animated.debug_grant_gold(4000);
                }
                self.shop.open = true;
                self.shop.scroll = 0;
                tracing::info!("demo shop: granted 4000 gold, shop open");
                self.demo_shop_stage = 1;
            }
            if (1..=6).contains(&self.demo_shop_stage) {
                let due = self.demo_shop_stage as f32 * 0.5;
                if self.demo_shop_timer >= due {
                    let index = (self.demo_shop_stage - 1) as usize;
                    self.purchase_item(index);
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
            if self.demo_shop_stage == 7 && self.demo_shop_timer >= 4.5 {
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
                    animated.debug_grant_gold(4000);
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
                        let ok = animated.cast_r(pos, Some(index), height);
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

            if self.demo_trial {
                self.demo_trial_timer += dt;
                if self.demo_trial_stage == 0 && self.demo_trial_timer >= 1.0 {
                    if let Some(index) = animated
                        .units
                        .iter()
                        .position(|unit| unit.is_champion && unit.team == 1 && unit.health > 0.0)
                    {
                        let pos = animated.unit_pos(index).unwrap_or(animated.player_pos());
                        let ok = animated.cast_r(pos, Some(index), height);
                        tracing::info!("demo trial: cast={ok} target={index}");
                    } else {
                        tracing::warn!("demo trial: no enemy champion");
                    }
                    self.demo_trial_stage = 1;
                }
                if self.demo_trial_stage == 1 && self.demo_trial_timer >= 3.0 {
                    let target = animated
                        .units
                        .iter()
                        .position(|unit| unit.is_champion && unit.team == 1 && unit.health > 0.0)
                        .and_then(|index| animated.unit_pos(index));
                    let pos = target.unwrap_or(animated.player_pos());
                    let ok = animated.cast_q(pos);
                    tracing::info!("demo trial: bananarang cast={ok}");
                    self.demo_trial_stage = 2;
                }
                if self.demo_trial_stage >= 1 && self.demo_trial_timer >= 13.0 {
                    let stats = animated.player_stats();
                    let bonus = animated.units[animated.player_index].min_base_damage_bonus;
                    let target = animated
                        .units
                        .iter()
                        .find(|unit| unit.is_champion && unit.team == 1)
                        .map(|unit| (unit.health, unit.max_health));
                    tracing::info!(
                        "demo trial: wins={} bonus={:.0} attack={:.0} hp={:.0}/{:.0} target={target:?}",
                        animated.duel_wins,
                        bonus,
                        stats.attack_damage,
                        stats.health,
                        stats.max_health
                    );
                    self.demo_trial = false;
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
            let steps = self.timescale.round().clamp(1.0, 20.0) as u32;
            let sim_start = Instant::now();
            for _ in 0..steps {
                animated.update(dt, height, collision);
                if self.demo_fx {
                    // Re-pin the demo numbers so they do not drift off screen.
                    animated.demo_fx();
                }
            }
            phase_sim = sim_start.elapsed().as_secs_f32() * 1000.0;
            let current_health = animated.player_stats().health;
            if self.last_health > current_health + 0.5 {
                self.damage_flash = (self.damage_flash + 0.55).min(0.9);
            }
            self.last_health = current_health;
            self.damage_flash = (self.damage_flash - dt * 2.4).max(0.0);
            animated.refresh_visibility(dt, collision);
            let mesh_start = Instant::now();
            animated.prepare_render(
                height,
                renderer.camera.target,
                camera_right,
                camera_up,
                camera_forward,
            );
            renderer.set_dynamic_mesh(&animated.vertices, &animated.indices);
            renderer.set_shadow_mesh(&animated.shadow_vertices, &animated.shadow_indices);
            renderer.set_caster_mesh(&animated.caster_vertices, &animated.caster_indices);
            renderer.set_lights(&animated.lights());
            renderer.set_time(self.app_time);
            // Fog of war is a shader-side circle around the hero, so it has
            // no geometry to fight with the terrain's z levels.
            renderer.set_vision(animated.player_pos(), 2600.0);
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
            for index in animated.take_delivered() {
                self.shop.add_owned(index);
                if let Some(item) = self.shop.items.get(index) {
                    log_event(&mut self.event_log, format!("DELIVERED {}", item.name));
                    tracing::info!("delivery: handed over {}", item.name);
                }
            }
            let scores = animated.team_scores();
            let heroes: Vec<HeroMarker> = animated
                .units
                .iter()
                .filter(|unit| unit.kind == UnitKind::Player && unit.team == 0)
                .map(|unit| HeroMarker {
                    name: animated.champion_name().to_string(),
                    team: unit.team,
                    pos: (unit.pos.x, unit.pos.z),
                    alive: unit.health > 0.0,
                    health: unit.health,
                    max_health: unit.max_health,
                    mana: unit.mana,
                    max_mana: unit.max_mana,
                    level: unit.level,
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
                .filter(|unit| unit.is_champion)
                .map(|unit| HeroMarker {
                    name: if unit.kind == UnitKind::Player {
                        animated.champion_name().to_string()
                    } else {
                        "RIVAL CHAMPION".to_string()
                    },
                    team: unit.team,
                    pos: (unit.pos.x, unit.pos.z),
                    alive: unit.health > 0.0,
                    health: unit.health,
                    max_health: unit.max_health,
                    mana: unit.mana,
                    max_mana: unit.max_mana,
                    level: unit.level,
                })
                .collect();
            // Project the world's floating numbers into screen space so the
            // HUD can draw them over the units that were hit.
            let aspect = hud_w / hud_h.max(1.0);
            let view_proj = renderer.camera.view_proj(aspect);
            let mut damage_numbers = Vec::new();
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
                damage_numbers.push((
                    screen_x,
                    screen_y,
                    float.text.clone(),
                    float.color,
                    float.scale,
                ));
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
                        | UnitKind::EnemyChampion
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
                let hero = matches!(unit.kind, UnitKind::Player | UnitKind::EnemyChampion);
                let structure = matches!(
                    unit.kind,
                    UnitKind::Tower | UnitKind::Inhibitor | UnitKind::Nexus | UnitKind::Watchtower
                );
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
                    structure,
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
            if !self.ui_live {
                self.hud.build_shop(hud_w, hud_h, &player_hud, &self.shop);
            }
            self.hud.build_minimap(
                renderer.size.0 as f32,
                renderer.size.1 as f32,
                animated,
                (renderer.camera.target.x, renderer.camera.target.z),
                renderer.camera.distance,
                renderer.camera.pitch,
                &heroes,
            );
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

    /// Render the paused world behind the champion picker.
    fn tick_hero_select(&mut self, dt: f32) {
        self.hero_select.time += dt;
        let (w, h) = self
            .renderer
            .as_ref()
            .map(|r| (r.size.0 as f32, r.size.1 as f32))
            .unwrap_or((1280.0, 720.0));
        self.hero_select.hover =
            hero_select::card_at(w, h, self.mouse_pos, self.hero_select.visible.len());

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
            animated.prepare_render(height, renderer.camera.target, right, up, forward);
            renderer.set_dynamic_mesh(&animated.vertices, &animated.indices);
            renderer.set_shadow_mesh(&animated.shadow_vertices, &animated.shadow_indices);
            renderer.set_caster_mesh(&animated.caster_vertices, &animated.caster_indices);
            renderer.set_lights(&animated.lights());
            renderer.set_time(self.app_time);
            renderer.set_vision(animated.player_pos(), 2600.0);
            animated.visibility_dirty = false;
            if self.follow {
                renderer.camera.target = animated.player_pos() + Vec3::Y * 140.0;
                renderer.update_camera();
            }
        }

        self.hud
            .build_hero_select(w, h, &self.hero_select, self.mouse_pos);
        if let Some(renderer) = self.renderer.as_mut() {
            renderer.set_hud_mesh(&self.hud.vertices, &self.hud.indices);
        }
    }

    /// Lock the pick in and move to the 5v5 lobby.
    fn open_lobby(&mut self) {
        let index = self.hero_select.selected;
        let Some(champion) = self.hero_select.champions.get(index) else {
            return;
        };
        tracing::info!("hero select: locked in {} ({})", champion.name, champion.id);
        self.hero_select.open = false;
        self.lobby_open = true;
        self.lobby_deadline = Some(Instant::now() + Duration::from_secs(8));
        if let Some(window) = self.window.as_ref() {
            window.set_title("Rift - Lobby");
        }
    }

    /// Render the paused world behind the 5v5 lobby.
    fn tick_lobby(&mut self, dt: f32) {
        let (w, h) = self
            .renderer
            .as_ref()
            .map(|r| (r.size.0 as f32, r.size.1 as f32))
            .unwrap_or((1280.0, 720.0));

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
            animated.prepare_render(height, renderer.camera.target, right, up, forward);
            renderer.set_dynamic_mesh(&animated.vertices, &animated.indices);
            renderer.set_shadow_mesh(&animated.shadow_vertices, &animated.shadow_indices);
            renderer.set_caster_mesh(&animated.caster_vertices, &animated.caster_indices);
            renderer.set_lights(&animated.lights());
            renderer.set_time(self.app_time);
            renderer.set_vision(animated.player_pos(), 2600.0);
            animated.visibility_dirty = false;
        }

        self.hud
            .build_lobby(w, h, &self.hero_select, self.mouse_pos);
        if let Some(renderer) = self.renderer.as_mut() {
            renderer.set_hud_mesh(&self.hud.vertices, &self.hud.indices);
        }
        // The lobby auto-starts like the reference client, but the player can
        // click the button or press Enter to drop in immediately.
        if let Some(deadline) = self.lobby_deadline {
            if Instant::now() >= deadline {
                self.start_match();
            }
        }
    }

    /// Lock in the highlighted champion and start a clean match.
    fn start_match(&mut self) {
        self.lobby_open = false;
        self.lobby_deadline = None;
        let index = self.hero_select.selected;
        let Some(champion) = self.hero_select.champions.get(index).cloned() else {
            return;
        };
        let Some(scene) = self.scene.as_ref() else {
            return;
        };
        let animated = match AnimatedScene::new(&self.cache, scene, &champion) {
            Ok(animated) => animated,
            Err(err) => {
                tracing::error!("failed to start match as {}: {err:#}", champion.name);
                return;
            }
        };

        self.animated = Some(animated);
        self.hero_select.open = false;
        self.hero_select.hover = None;
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
        let pending = self
            .animated
            .as_ref()
            .map(|animated| animated.delivery_count())
            .unwrap_or(0);
        if !self.shop.has_free_slot() || self.shop.item_count() + pending >= 6 {
            tracing::info!("shop: inventory full (including items in transit)");
            log_event(&mut self.event_log, "INVENTORY FULL".to_string());
            return;
        }
        let Some(animated) = self.animated.as_mut() else {
            return;
        };
        match animated.buy_item(index, &item) {
            Ok(BuyOutcome::Delivered) => {
                self.shop.add_owned(index);
                tracing::info!(
                    "shop: bought {} ({}) for {}g ({} slots used)",
                    item.name,
                    item.id,
                    item.cost,
                    self.shop.item_count()
                );
                log_event(&mut self.event_log, format!("BOUGHT {}", item.name));
            }
            Ok(BuyOutcome::Stored) => {
                tracing::info!("shop: {} stored in the chest for delivery", item.name);
                log_event(&mut self.event_log, format!("STORED {}", item.name));
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
                tracing::info!(
                    "shop: sold {} for {}g ({} slots used)",
                    item.name,
                    item.cost / 2,
                    self.shop.item_count()
                );
                log_event(&mut self.event_log, format!("SOLD {}", item.name));
            }
            Err(err) => tracing::info!("shop: sell {} failed: {err}", item.name),
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
            let area = rift_ui::hud::shop::rect(
                rift_ui::draw::Rect::new(0, 0, window.0 as i32, window.1 as i32),
                scale,
            );
            let live = crate::ui_live::shop(&self.shop);
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
                        if let Some(item) = self.shop.selected {
                            self.purchase_item(item);
                        }
                    }
                    rift_ui::hud::shop::ShopHit::Close => self.shop.open = false,
                }
                return true;
            }
            let (mx, my) = mouse;
            return mx >= area.x && mx < area.right() && my >= area.y && my < area.bottom();
        }

        let Some(area) = placed.get(rift_ui::hud::Piece::Inventory) else {
            return false;
        };
        if rift_ui::hud::inventory::shop_button_at(area, scale, mouse) {
            self.shop.open = true;
            self.shop.selected = self.shop.filtered_items().first().copied();
            self.shop.scroll = 0;
            tracing::info!("shop: opened from the assembled inventory");
            return true;
        }
        let (mx, my) = mouse;
        mx >= area.x && mx < area.right() && my >= area.y && my < area.bottom()
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
        let (Some(height), Some(collision), Some(animated)) = (
            self.height.as_ref(),
            self.collision.as_ref(),
            self.animated.as_mut(),
        ) else {
            return;
        };
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
            let target = animated.pick_enemy_champion(origin, dir);
            let fallback = animated.player_pos();
            let point = renderer
                .pick_ground(self.mouse_pos.0, self.mouse_pos.1, height)
                .unwrap_or(fallback);
            let cast = animated.cast_r(point, target, height);
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
                state.chat = crate::ui_live::chat(player);
                state.feed = crate::ui_live::feed(player);
                state.announcement = crate::ui_live::announcement(player);
                state.gold = player.gold;
            }
            if let (Some(player), Some(animated)) = (player, animated) {
                state.death = crate::ui_live::death(player, animated);
                state.allies = crate::ui_live::allies(animated);
                state.target = crate::ui_live::target(animated, selected);
            }
            if let (Some(animated), Some(scene)) = (animated, scene) {
                state.minimap = crate::ui_live::minimap(scene, animated, camera, renderer.size);
                state.objectives = crate::ui_live::objectives(scene, animated);
            }
            if let Some(shop) = shop {
                state.shop = crate::ui_live::shop(shop);
                state.tooltip = crate::ui_live::tooltip(shop, mouse);
                state.inventory = crate::ui_live::inventory(shop);
            }
        }
        let placed = rift_ui::hud::place(&state, renderer.size, scale);
        placed_out = Some(placed.clone());
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

/// Dev demos and explicit spawn requests boot straight into the match instead
/// of stopping at the champion picker.
fn has_demo_flags(cli: &Cli) -> bool {
    cli.ui_test
        || cli.ui_theme_sheet
        || cli.ui_layout
        || cli.ui_live
        || cli.demo_target.is_some()
        || cli.spawn.is_some()
        || cli.demo_combat
        || cli.demo_anim
        || cli.demo_duel
        || cli.demo_trial
        || cli.demo_camp
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

fn resolve_render_scale(text: &str, window: (u32, u32)) -> Option<f32> {
    if text.eq_ignore_ascii_case("auto") {
        let pixels = (window.0 as f32 * window.1 as f32).max(1.0);
        Some((1_200_000.0 / pixels).sqrt().clamp(0.62, 1.0))
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
    match name.to_ascii_lowercase().as_str() {
        "albedo" => 1,
        "normal" => 2,
        "lighting" => 3,
        "fog" => 4,
        "lights" => 5,
        "shadow" => 6,
        "ao" => 7,
        _ => 0,
    }
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
        assert!((scale - 0.762).abs() < 0.01, "{scale}");
        assert_eq!(resolve_render_scale("0.85", (1920, 1080)), Some(0.85));
        assert_eq!(resolve_render_scale("0.4", (1920, 1080)), None);
        assert_eq!(resolve_render_scale("1.2", (1920, 1080)), None);
    }
}
