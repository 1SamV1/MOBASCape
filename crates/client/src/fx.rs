//! Additive combat effects: projectile looks, glows, ribbons and ground
//! indicators.
//!
//! Everything here is cosmetic. Effects are written into an [`FxMesh`] that
//! the renderer draws after the opaque scene with additive blending and no
//! depth writes, so glows stack into bloom instead of hiding each other.
//! The vertex colour's alpha channel carries intensity (HDR, may exceed 1)
//! and the material picks the shape the fragment shader cuts:
//!
//! | material | shape |
//! |---|---|
//! | [`MAT_GLOW`] | soft round sprite with a hot core, `uv` in -1..1 |
//! | [`MAT_RIBBON`] | camera-facing strip, soft across `uv.y` |
//! | [`MAT_FLAT`] | flat colour (translucent cache model faces) |
//! | [`MAT_BAND`] | ground band, soft across `uv.y` |
//! | [`MAT_AREA`] | ground fill with a bright rim, `uv.y` 0 inside to 1 at the edge |

use glam::Vec3;

use crate::scene::{HeightGrid, SceneVertex};

pub const MAT_GLOW: u32 = 20;
pub const MAT_RIBBON: u32 = 21;
pub const MAT_FLAT: u32 = 22;
pub const MAT_BAND: u32 = 23;
pub const MAT_AREA: u32 = 24;

/// Hard cap on additive vertices per frame so a teamfight cannot grow the
/// buffer without bound.
const MAX_FX_VERTICES: usize = 120_000;

/// How a projectile and its impact look. Spot animation ids are OSRS cache
/// graphics; the renderer falls back to the glow alone when one is missing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProjectileLook {
    pub name: &'static str,
    /// Flying model (cache spot animation), if any.
    pub spotanim: Option<u32>,
    /// One-shot model played where the projectile lands.
    pub hit_spotanim: Option<u32>,
    /// Multiplier on the spot animation's own resize.
    pub model_scale: f32,
    /// Core glow colour and sprite size in world units.
    pub glow: [u8; 3],
    pub glow_size: f32,
    pub glow_intensity: f32,
    /// Trail colour, head width and how many history points it keeps.
    pub trail: [u8; 3],
    pub trail_width: f32,
    pub trail_points: usize,
    /// Peak height of a lobbed basic attack, in world units.
    pub arc: f32,
    /// Roll speed of the flying model, radians per second.
    pub spin: f32,
    /// Embers shed per second while flying.
    pub embers: f32,
    pub ember_color: [u8; 3],
    /// Impact flash radius and spark count.
    pub impact_radius: f32,
    pub impact_sparks: usize,
    pub impact_color: [u8; 3],
    /// Point light carried by the projectile (linear RGB, radius).
    pub light: [f32; 3],
    pub light_radius: f32,
}

impl ProjectileLook {
    const fn base(name: &'static str) -> Self {
        Self {
            name,
            spotanim: None,
            hit_spotanim: None,
            model_scale: 1.0,
            glow: [255, 170, 80],
            glow_size: 60.0,
            glow_intensity: 1.4,
            trail: [255, 140, 60],
            trail_width: 26.0,
            trail_points: 10,
            arc: 0.0,
            spin: 0.0,
            embers: 0.0,
            ember_color: [255, 170, 80],
            impact_radius: 150.0,
            impact_sparks: 10,
            impact_color: [255, 150, 60],
            light: [1.0, 0.45, 0.12],
            light_radius: 460.0,
        }
    }
}

impl ProjectileLook {
    /// This look recoloured to `tint` (an ability colour), if given.
    pub fn tinted(mut self, tint: Option<[u8; 3]>) -> Self {
        if let Some(color) = tint {
            let bright = color.map(|c| (c as f32 * 0.6 + 102.0).min(255.0) as u8);
            self.glow = color;
            self.trail = color;
            self.impact_color = color;
            self.ember_color = bright;
            self.light = color.map(|c| c as f32 / 255.0);
        }
        self
    }
}

/// Look used when nothing names one: a warm arcane orb.
pub const DEFAULT_LOOK: usize = 0;

/// The preset table. Order is stable; [`look_by_name`] resolves names from
/// champion files.
pub const LOOKS: &[ProjectileLook] = &[
    ProjectileLook {
        glow: [255, 175, 90],
        trail: [255, 120, 50],
        embers: 30.0,
        ..ProjectileLook::base("arcane")
    },
    // Blowpipe dart: small, fast, a venom-green streak.
    ProjectileLook {
        spotanim: Some(228),
        model_scale: 1.6,
        glow: [170, 255, 170],
        glow_size: 26.0,
        glow_intensity: 0.9,
        trail: [90, 230, 110],
        trail_width: 9.0,
        trail_points: 7,
        arc: 36.0,
        impact_radius: 90.0,
        impact_sparks: 7,
        impact_color: [120, 255, 140],
        light: [0.3, 1.0, 0.4],
        light_radius: 260.0,
        ..ProjectileLook::base("dart")
    },
    // Rune arrow with a pale streak.
    ProjectileLook {
        spotanim: Some(15),
        model_scale: 1.8,
        glow: [235, 240, 255],
        glow_size: 22.0,
        glow_intensity: 0.7,
        trail: [200, 215, 255],
        trail_width: 8.0,
        trail_points: 6,
        arc: 70.0,
        impact_radius: 80.0,
        impact_sparks: 6,
        impact_color: [230, 235, 255],
        light: [0.8, 0.85, 1.0],
        light_radius: 200.0,
        ..ProjectileLook::base("arrow")
    },
    ProjectileLook {
        spotanim: Some(130),
        hit_spotanim: Some(131),
        model_scale: 1.5,
        glow: [255, 150, 50],
        glow_size: 84.0,
        trail: [255, 90, 20],
        trail_width: 34.0,
        embers: 70.0,
        ember_color: [255, 190, 90],
        impact_radius: 190.0,
        impact_sparks: 16,
        impact_color: [255, 140, 40],
        ..ProjectileLook::base("fire_blast")
    },
    ProjectileLook {
        spotanim: Some(156),
        hit_spotanim: Some(157),
        model_scale: 1.4,
        glow: [255, 130, 40],
        glow_size: 110.0,
        glow_intensity: 1.6,
        trail: [255, 80, 20],
        trail_width: 48.0,
        trail_points: 14,
        spin: 6.0,
        embers: 90.0,
        ember_color: [255, 200, 90],
        impact_radius: 240.0,
        impact_sparks: 22,
        impact_color: [255, 120, 30],
        light_radius: 620.0,
        ..ProjectileLook::base("fire_wave")
    },
    ProjectileLook {
        spotanim: Some(136),
        hit_spotanim: Some(137),
        model_scale: 1.5,
        glow: [90, 170, 255],
        glow_size: 84.0,
        trail: [40, 110, 255],
        trail_width: 34.0,
        embers: 50.0,
        ember_color: [150, 210, 255],
        impact_radius: 190.0,
        impact_sparks: 16,
        impact_color: [90, 170, 255],
        light: [0.25, 0.55, 1.0],
        ..ProjectileLook::base("water_blast")
    },
    ProjectileLook {
        spotanim: Some(162),
        hit_spotanim: Some(163),
        model_scale: 1.4,
        glow: [80, 160, 255],
        glow_size: 110.0,
        glow_intensity: 1.6,
        trail: [40, 100, 255],
        trail_width: 48.0,
        trail_points: 14,
        spin: 6.0,
        embers: 70.0,
        ember_color: [160, 220, 255],
        impact_radius: 240.0,
        impact_sparks: 22,
        impact_color: [90, 170, 255],
        light: [0.25, 0.55, 1.0],
        light_radius: 620.0,
        ..ProjectileLook::base("water_wave")
    },
    ProjectileLook {
        spotanim: Some(165),
        hit_spotanim: Some(166),
        model_scale: 1.5,
        glow: [130, 255, 110],
        glow_size: 110.0,
        glow_intensity: 1.3,
        trail: [70, 200, 60],
        trail_width: 60.0,
        trail_points: 14,
        spin: 5.0,
        embers: 60.0,
        ember_color: [190, 255, 150],
        impact_radius: 260.0,
        impact_sparks: 22,
        impact_color: [140, 255, 110],
        light: [0.4, 1.0, 0.35],
        light_radius: 620.0,
        ..ProjectileLook::base("earth_wave")
    },
    ProjectileLook {
        spotanim: Some(133),
        hit_spotanim: Some(134),
        model_scale: 1.5,
        glow: [235, 240, 255],
        glow_size: 80.0,
        glow_intensity: 1.2,
        trail: [200, 210, 240],
        trail_width: 32.0,
        embers: 40.0,
        ember_color: [255, 255, 255],
        impact_radius: 180.0,
        impact_sparks: 14,
        impact_color: [235, 240, 255],
        light: [0.85, 0.9, 1.0],
        ..ProjectileLook::base("wind_blast")
    },
    // King Black Dragon style breaths, used by the dragons.
    ProjectileLook {
        spotanim: Some(393),
        model_scale: 1.1,
        glow: [255, 140, 40],
        glow_size: 130.0,
        glow_intensity: 1.7,
        trail: [255, 70, 10],
        trail_width: 70.0,
        trail_points: 14,
        embers: 120.0,
        ember_color: [255, 190, 80],
        impact_radius: 230.0,
        impact_sparks: 24,
        impact_color: [255, 120, 30],
        light_radius: 680.0,
        ..ProjectileLook::base("dragonfire")
    },
    ProjectileLook {
        spotanim: Some(395),
        model_scale: 1.1,
        glow: [170, 225, 255],
        glow_size: 120.0,
        glow_intensity: 1.5,
        trail: [110, 180, 255],
        trail_width: 64.0,
        trail_points: 14,
        embers: 90.0,
        ember_color: [220, 245, 255],
        impact_radius: 220.0,
        impact_sparks: 22,
        impact_color: [170, 225, 255],
        light: [0.5, 0.75, 1.0],
        light_radius: 620.0,
        ..ProjectileLook::base("icefire")
    },
    ProjectileLook {
        spotanim: Some(394),
        model_scale: 1.0,
        glow: [150, 255, 90],
        glow_size: 120.0,
        glow_intensity: 1.4,
        trail: [80, 200, 40],
        trail_width: 64.0,
        trail_points: 14,
        embers: 80.0,
        ember_color: [190, 255, 120],
        impact_radius: 220.0,
        impact_sparks: 20,
        impact_color: [150, 255, 90],
        light: [0.45, 1.0, 0.25],
        light_radius: 600.0,
        ..ProjectileLook::base("venomfire")
    },
    // JalTok-Jad's fireball.
    ProjectileLook {
        spotanim: Some(448),
        hit_spotanim: Some(157),
        model_scale: 1.1,
        glow: [255, 120, 40],
        glow_size: 130.0,
        glow_intensity: 1.8,
        trail: [255, 60, 20],
        trail_width: 66.0,
        trail_points: 14,
        spin: 3.0,
        embers: 110.0,
        ember_color: [255, 200, 90],
        impact_radius: 250.0,
        impact_sparks: 24,
        impact_color: [255, 120, 40],
        light_radius: 700.0,
        ..ProjectileLook::base("jad_fire")
    },
    // Trident-style blue star.
    ProjectileLook {
        spotanim: Some(1252),
        hit_spotanim: Some(1253),
        model_scale: 1.3,
        glow: [110, 170, 255],
        glow_size: 90.0,
        glow_intensity: 1.6,
        trail: [70, 120, 255],
        trail_width: 30.0,
        spin: 9.0,
        embers: 60.0,
        ember_color: [180, 220, 255],
        impact_radius: 200.0,
        impact_sparks: 18,
        impact_color: [120, 180, 255],
        light: [0.35, 0.55, 1.0],
        ..ProjectileLook::base("trident")
    },
    // Armadyl crystal shard.
    ProjectileLook {
        spotanim: Some(1200),
        model_scale: 0.7,
        glow: [150, 200, 255],
        glow_size: 70.0,
        glow_intensity: 1.3,
        trail: [110, 160, 255],
        trail_width: 22.0,
        trail_points: 9,
        embers: 40.0,
        ember_color: [210, 235, 255],
        impact_radius: 160.0,
        impact_sparks: 14,
        impact_color: [160, 210, 255],
        light: [0.45, 0.65, 1.0],
        light_radius: 380.0,
        ..ProjectileLook::base("crystal")
    },
    // Purple telekinetic rings.
    ProjectileLook {
        spotanim: Some(143),
        hit_spotanim: Some(144),
        model_scale: 1.2,
        glow: [210, 120, 255],
        glow_size: 80.0,
        glow_intensity: 1.4,
        trail: [170, 70, 255],
        trail_width: 30.0,
        spin: 7.0,
        embers: 40.0,
        ember_color: [240, 190, 255],
        impact_radius: 180.0,
        impact_sparks: 14,
        impact_color: [210, 130, 255],
        light: [0.7, 0.35, 1.0],
        ..ProjectileLook::base("telegrab")
    },
    // Pale ghostly soul: glow only, long wavering trail.
    ProjectileLook {
        glow: [170, 210, 255],
        glow_size: 70.0,
        glow_intensity: 1.3,
        trail: [110, 150, 255],
        trail_width: 34.0,
        trail_points: 16,
        embers: 50.0,
        ember_color: [200, 225, 255],
        impact_radius: 170.0,
        impact_sparks: 14,
        impact_color: [170, 210, 255],
        light: [0.5, 0.65, 1.0],
        light_radius: 380.0,
        ..ProjectileLook::base("soul")
    },
    // Wide ground shockwave for big line skills.
    ProjectileLook {
        glow: [240, 220, 180],
        glow_size: 150.0,
        glow_intensity: 1.0,
        trail: [200, 170, 120],
        trail_width: 120.0,
        trail_points: 12,
        embers: 120.0,
        ember_color: [230, 200, 150],
        impact_radius: 220.0,
        impact_sparks: 18,
        impact_color: [240, 220, 180],
        light: [1.0, 0.85, 0.6],
        light_radius: 520.0,
        ..ProjectileLook::base("shockwave")
    },
    // Lane fairy bolt: small and cheap, tinted per team at spawn.
    ProjectileLook {
        glow: [150, 200, 255],
        glow_size: 38.0,
        glow_intensity: 1.1,
        trail: [90, 140, 255],
        trail_width: 14.0,
        trail_points: 6,
        impact_radius: 90.0,
        impact_sparks: 6,
        impact_color: [150, 200, 255],
        light: [0.4, 0.6, 1.0],
        light_radius: 240.0,
        ..ProjectileLook::base("minion_blue")
    },
    ProjectileLook {
        glow: [255, 150, 130],
        glow_size: 38.0,
        glow_intensity: 1.1,
        trail: [255, 90, 70],
        trail_width: 14.0,
        trail_points: 6,
        impact_radius: 90.0,
        impact_sparks: 6,
        impact_color: [255, 150, 130],
        light: [1.0, 0.45, 0.35],
        light_radius: 240.0,
        ..ProjectileLook::base("minion_red")
    },
    // Tower shots: a heavy orb with a thick trail.
    ProjectileLook {
        spotanim: Some(1526),
        model_scale: 3.2,
        glow: [110, 180, 255],
        glow_size: 120.0,
        glow_intensity: 1.8,
        trail: [60, 130, 255],
        trail_width: 46.0,
        trail_points: 12,
        spin: 4.0,
        embers: 60.0,
        ember_color: [170, 215, 255],
        impact_radius: 230.0,
        impact_sparks: 20,
        impact_color: [120, 190, 255],
        light: [0.35, 0.6, 1.0],
        light_radius: 640.0,
        ..ProjectileLook::base("tower_blue")
    },
    ProjectileLook {
        spotanim: Some(1528),
        model_scale: 3.2,
        glow: [255, 120, 80],
        glow_size: 120.0,
        glow_intensity: 1.8,
        trail: [255, 70, 40],
        trail_width: 46.0,
        trail_points: 12,
        spin: 4.0,
        embers: 60.0,
        ember_color: [255, 190, 150],
        impact_radius: 230.0,
        impact_sparks: 20,
        impact_color: [255, 120, 80],
        light: [1.0, 0.4, 0.25],
        light_radius: 640.0,
        ..ProjectileLook::base("tower_red")
    },
    ProjectileLook {
        spotanim: Some(1527),
        model_scale: 3.6,
        glow: [255, 220, 120],
        glow_size: 140.0,
        glow_intensity: 1.6,
        trail: [255, 180, 60],
        trail_width: 52.0,
        trail_points: 16,
        spin: 2.0,
        embers: 50.0,
        ember_color: [255, 235, 170],
        impact_radius: 240.0,
        impact_sparks: 20,
        impact_color: [255, 220, 120],
        light: [1.0, 0.8, 0.4],
        light_radius: 640.0,
        ..ProjectileLook::base("watchtower")
    },
    ProjectileLook {
        spotanim: Some(139),
        hit_spotanim: Some(140),
        model_scale: 1.4,
        glow: [190, 230, 110],
        glow_size: 80.0,
        trail: [120, 170, 60],
        trail_width: 30.0,
        embers: 40.0,
        ember_color: [210, 240, 150],
        impact_radius: 180.0,
        impact_sparks: 14,
        impact_color: [180, 230, 110],
        light: [0.6, 0.9, 0.3],
        ..ProjectileLook::base("earth_blast")
    },
    ProjectileLook {
        spotanim: Some(1539),
        model_scale: 1.3,
        glow: [255, 70, 70],
        glow_size: 80.0,
        glow_intensity: 1.6,
        trail: [200, 20, 30],
        trail_width: 30.0,
        embers: 60.0,
        ember_color: [255, 120, 110],
        impact_radius: 190.0,
        impact_sparks: 16,
        impact_color: [255, 70, 70],
        light: [1.0, 0.2, 0.2],
        ..ProjectileLook::base("blood")
    },
    ProjectileLook {
        spotanim: Some(1483),
        model_scale: 1.4,
        glow: [140, 255, 90],
        glow_size: 80.0,
        glow_intensity: 1.2,
        trail: [70, 190, 40],
        trail_width: 40.0,
        trail_points: 12,
        arc: 120.0,
        embers: 50.0,
        ember_color: [180, 255, 130],
        impact_radius: 200.0,
        impact_sparks: 18,
        impact_color: [140, 255, 90],
        light: [0.4, 1.0, 0.25],
        ..ProjectileLook::base("acid")
    },
    // Big Frog's tongue: a wet pink lash with a short glossy trail.
    ProjectileLook {
        glow: [255, 140, 170],
        glow_size: 44.0,
        glow_intensity: 1.1,
        trail: [230, 80, 120],
        trail_width: 18.0,
        trail_points: 12,
        impact_radius: 130.0,
        impact_sparks: 10,
        impact_color: [255, 150, 180],
        light: [1.0, 0.45, 0.6],
        light_radius: 300.0,
        ..ProjectileLook::base("tongue")
    },
    // Ice Barrage battle spell: the Ice Blitz flecks (368) in a pale
    // frost glow; the target's ice block (369) is played by the spell.
    ProjectileLook {
        spotanim: Some(368),
        model_scale: 1.2,
        glow: [190, 235, 255],
        glow_size: 70.0,
        glow_intensity: 1.2,
        trail: [140, 200, 255],
        trail_width: 26.0,
        trail_points: 10,
        embers: 40.0,
        ember_color: [230, 248, 255],
        impact_radius: 150.0,
        impact_sparks: 14,
        impact_color: [190, 235, 255],
        light: [0.6, 0.85, 1.0],
        light_radius: 380.0,
        ..ProjectileLook::base("ice_barrage")
    },
    // Tele Block battle spell: the dark streaks (344) landing in purple
    // flecks (345); the target holds the purple bars (342).
    ProjectileLook {
        spotanim: Some(344),
        hit_spotanim: Some(345),
        glow: [170, 110, 255],
        glow_size: 56.0,
        glow_intensity: 1.0,
        trail: [110, 60, 200],
        trail_width: 20.0,
        trail_points: 10,
        impact_radius: 140.0,
        impact_sparks: 12,
        impact_color: [190, 140, 255],
        light: [0.6, 0.4, 1.0],
        light_radius: 320.0,
        ..ProjectileLook::base("tele_block")
    },
    // Chaos Elemental Discord: the three-colour swirl (557) landing in its
    // ring (558), with a trail that shifts through red, green and blue.
    ProjectileLook {
        spotanim: Some(557),
        hit_spotanim: Some(558),
        model_scale: 1.1,
        glow: [220, 140, 255],
        glow_size: 80.0,
        glow_intensity: 1.3,
        trail: [120, 255, 140],
        trail_width: 34.0,
        trail_points: 12,
        embers: 60.0,
        ember_color: [255, 120, 120],
        impact_radius: 180.0,
        impact_sparks: 18,
        impact_color: [140, 200, 255],
        light: [0.8, 0.55, 1.0],
        light_radius: 420.0,
        ..ProjectileLook::base("chaos")
    },
    // Chaos Elemental Confusion: the green orb (551) and ring (552).
    ProjectileLook {
        spotanim: Some(551),
        hit_spotanim: Some(552),
        glow: [140, 255, 140],
        glow_size: 64.0,
        trail: [70, 220, 90],
        trail_width: 24.0,
        trail_points: 10,
        impact_radius: 150.0,
        impact_sparks: 12,
        impact_color: [150, 255, 150],
        light: [0.4, 1.0, 0.45],
        light_radius: 340.0,
        ..ProjectileLook::base("confusion")
    },
    // Chaos Elemental Madness: the red recolour (554) and ring (555).
    ProjectileLook {
        spotanim: Some(554),
        hit_spotanim: Some(555),
        glow: [255, 110, 100],
        glow_size: 64.0,
        trail: [230, 60, 50],
        trail_width: 24.0,
        trail_points: 10,
        impact_radius: 150.0,
        impact_sparks: 12,
        impact_color: [255, 140, 120],
        light: [1.0, 0.4, 0.35],
        light_radius: 340.0,
        ..ProjectileLook::base("madness")
    },
    // Pestilent Bloat's Flesh Hook: a thrown limb (1574) that lands in a
    // blood splat (1576), trailing gore.
    ProjectileLook {
        spotanim: Some(1574),
        hit_spotanim: Some(1576),
        model_scale: 1.6,
        glow: [150, 40, 30],
        glow_size: 40.0,
        glow_intensity: 0.6,
        trail: [120, 20, 15],
        trail_width: 14.0,
        trail_points: 16,
        embers: 30.0,
        ember_color: [140, 20, 10],
        impact_radius: 140.0,
        impact_sparks: 14,
        impact_color: [170, 30, 20],
        light: [0.6, 0.15, 0.1],
        light_radius: 200.0,
        ..ProjectileLook::base("bloat_hook")
    },
];

/// Resolve a look by name. Unknown names fall back to the default orb so a
/// typo in a champion file never breaks a cast.
pub fn look_by_name(name: &str) -> usize {
    LOOKS
        .iter()
        .position(|look| look.name.eq_ignore_ascii_case(name))
        .unwrap_or_else(|| {
            tracing::warn!("fx: unknown projectile look {name:?}, using the default");
            DEFAULT_LOOK
        })
}

pub fn look(index: usize) -> &'static ProjectileLook {
    LOOKS.get(index).unwrap_or(&LOOKS[DEFAULT_LOOK])
}

/// Every spot animation the preset table references.
pub fn spotanim_ids() -> Vec<u32> {
    let mut ids: Vec<u32> = LOOKS
        .iter()
        .flat_map(|look| [look.spotanim, look.hit_spotanim])
        .flatten()
        .collect();
    ids.sort_unstable();
    ids.dedup();
    ids
}

/// Height of a lobbed projectile above its straight path at progress `t`.
pub fn arc_offset(arc: f32, t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    arc * 4.0 * t * (1.0 - t)
}

/// Additive geometry for one frame.
#[derive(Default)]
pub struct FxMesh {
    pub vertices: Vec<SceneVertex>,
    pub indices: Vec<u32>,
}

fn fx_vertex(
    pos: Vec3,
    color: [u8; 3],
    intensity: f32,
    uv: [f32; 2],
    material: u32,
) -> SceneVertex {
    let mut vertex = SceneVertex::new(pos, Vec3::Y, color);
    vertex.color[3] = intensity.max(0.0);
    vertex.uv = uv;
    vertex.material = material;
    vertex
}

impl FxMesh {
    pub fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
    }

    fn room(&self, vertices: usize) -> bool {
        self.vertices.len() + vertices <= MAX_FX_VERTICES
    }

    fn quad(&mut self, corners: [SceneVertex; 4]) {
        let base = self.vertices.len() as u32;
        self.vertices.extend_from_slice(&corners);
        self.indices
            .extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 1, base + 3]);
    }

    /// A camera-facing soft sprite.
    pub fn glow(
        &mut self,
        center: Vec3,
        size: f32,
        color: [u8; 3],
        intensity: f32,
        right: Vec3,
        up: Vec3,
    ) {
        if size <= 0.0 || intensity <= 0.0 || !self.room(4) {
            return;
        }
        let r = right * size;
        let u = up * size;
        self.quad([
            fx_vertex(center - r - u, color, intensity, [-1.0, -1.0], MAT_GLOW),
            fx_vertex(center + r - u, color, intensity, [1.0, -1.0], MAT_GLOW),
            fx_vertex(center - r + u, color, intensity, [-1.0, 1.0], MAT_GLOW),
            fx_vertex(center + r + u, color, intensity, [1.0, 1.0], MAT_GLOW),
        ]);
    }

    /// A camera-facing strip through `points` (head first). Width and
    /// intensity taper from head to tail.
    pub fn ribbon(
        &mut self,
        points: &[Vec3],
        head_width: f32,
        color: [u8; 3],
        intensity: f32,
        camera_forward: Vec3,
    ) {
        if points.len() < 2 || !self.room(points.len() * 2) {
            return;
        }
        let count = points.len();
        let base = self.vertices.len() as u32;
        for (i, &p) in points.iter().enumerate() {
            let along = if i + 1 < count {
                points[i] - points[i + 1]
            } else {
                points[i - 1] - points[i]
            };
            let side = along.cross(camera_forward).normalize_or_zero();
            let t = i as f32 / (count - 1) as f32;
            let width = head_width * (1.0 - t * 0.85);
            let fade = intensity * (1.0 - t).powf(1.4);
            self.vertices.push(fx_vertex(
                p + side * width,
                color,
                fade,
                [t, -1.0],
                MAT_RIBBON,
            ));
            self.vertices.push(fx_vertex(
                p - side * width,
                color,
                fade,
                [t, 1.0],
                MAT_RIBBON,
            ));
        }
        for i in 0..(count as u32 - 1) {
            let o = base + i * 2;
            self.indices
                .extend_from_slice(&[o, o + 1, o + 2, o + 2, o + 1, o + 3]);
        }
    }

    /// One flat additive triangle (a translucent face of a cache model).
    pub fn triangle(&mut self, corners: [Vec3; 3], color: [u8; 3], intensity: f32) {
        if !self.room(3) {
            return;
        }
        let base = self.vertices.len() as u32;
        for p in corners {
            self.vertices
                .push(fx_vertex(p, color, intensity, [0.0, 0.0], MAT_FLAT));
        }
        self.indices.extend_from_slice(&[base, base + 1, base + 2]);
    }

    /// A soft ring on the terrain.
    pub fn ground_ring(
        &mut self,
        height: &HeightGrid,
        center: Vec3,
        radius: f32,
        width: f32,
        color: [u8; 3],
        intensity: f32,
    ) {
        let segments = ring_segments(radius);
        if radius <= 0.0 || !self.room(segments * 2 + 2) {
            return;
        }
        let base = self.vertices.len() as u32;
        let inner = (radius - width).max(0.0);
        let outer = radius + width;
        for i in 0..=segments {
            let a = i as f32 / segments as f32 * std::f32::consts::TAU;
            let (s, c) = a.sin_cos();
            for (r, v) in [(outer, -1.0), (inner, 1.0)] {
                let x = center.x + c * r;
                let z = center.z + s * r;
                let p = Vec3::new(x, height.height_at(x, z) + 8.0, z);
                self.vertices
                    .push(fx_vertex(p, color, intensity, [0.0, v], MAT_BAND));
            }
        }
        for i in 0..segments as u32 {
            let o = base + i * 2;
            self.indices
                .extend_from_slice(&[o, o + 1, o + 2, o + 2, o + 1, o + 3]);
        }
    }

    /// A filled ground circle or cone (`arc` degrees around `facing`) with a
    /// bright rim. Used by ability indicators.
    #[allow(clippy::too_many_arguments)]
    pub fn ground_area(
        &mut self,
        height: &HeightGrid,
        center: Vec3,
        radius: f32,
        arc: f32,
        facing: Vec3,
        color: [u8; 3],
        intensity: f32,
    ) {
        let full = arc >= 359.0;
        let span = arc.clamp(1.0, 360.0).to_radians();
        let segments =
            ((ring_segments(radius) as f32 * span / std::f32::consts::TAU).ceil() as usize).max(6);
        let rings = 6usize;
        if radius <= 0.0 || !self.room((segments + 1) * (rings + 1) + 2 * (rings + 1) * 2) {
            return;
        }
        let start = facing.z.atan2(facing.x) - span * 0.5;
        let base = self.vertices.len() as u32;
        for ring in 0..=rings {
            let f = ring as f32 / rings as f32;
            for i in 0..=segments {
                let a = start + span * i as f32 / segments as f32;
                let (s, c) = a.sin_cos();
                let x = center.x + c * radius * f;
                let z = center.z + s * radius * f;
                let p = Vec3::new(x, height.height_at(x, z) + 7.0, z);
                self.vertices
                    .push(fx_vertex(p, color, intensity, [0.0, f], MAT_AREA));
            }
        }
        let row = segments as u32 + 1;
        for ring in 0..rings as u32 {
            for i in 0..segments as u32 {
                let o = base + ring * row + i;
                self.indices
                    .extend_from_slice(&[o, o + 1, o + row, o + row, o + 1, o + row + 1]);
            }
        }
        if !full {
            // Bright straight edges on a cone.
            for edge in [start, start + span] {
                let dir = Vec3::new(edge.cos(), 0.0, edge.sin());
                self.ground_strip(height, center, dir, radius, 10.0, color, intensity * 1.6);
            }
        }
    }

    /// A filled ground rectangle from `from` along `dir` with bright long
    /// edges. Used by skillshot and dash indicators.
    #[allow(clippy::too_many_arguments)]
    pub fn ground_rect(
        &mut self,
        height: &HeightGrid,
        from: Vec3,
        dir: Vec3,
        length: f32,
        half_width: f32,
        color: [u8; 3],
        intensity: f32,
    ) {
        let dir = Vec3::new(dir.x, 0.0, dir.z).normalize_or_zero();
        if dir == Vec3::ZERO || length <= 0.0 {
            return;
        }
        let steps = ((length / 96.0).ceil() as usize).clamp(2, 64);
        if !self.room((steps + 1) * 3) {
            return;
        }
        let side = Vec3::new(-dir.z, 0.0, dir.x);
        let base = self.vertices.len() as u32;
        for i in 0..=steps {
            let along = from + dir * (length * i as f32 / steps as f32);
            for (offset, v) in [(-1.0f32, 1.0f32), (0.0, 0.0), (1.0, 1.0)] {
                let p = along + side * (half_width * offset);
                let p = Vec3::new(p.x, height.height_at(p.x, p.z) + 7.0, p.z);
                self.vertices
                    .push(fx_vertex(p, color, intensity, [0.0, v], MAT_AREA));
            }
        }
        for i in 0..steps as u32 {
            let o = base + i * 3;
            self.indices.extend_from_slice(&[
                o,
                o + 1,
                o + 3,
                o + 3,
                o + 1,
                o + 4,
                o + 1,
                o + 2,
                o + 4,
                o + 4,
                o + 2,
                o + 5,
            ]);
        }
    }

    /// A thin soft line on the terrain.
    #[allow(clippy::too_many_arguments)]
    pub fn ground_strip(
        &mut self,
        height: &HeightGrid,
        from: Vec3,
        dir: Vec3,
        length: f32,
        half_width: f32,
        color: [u8; 3],
        intensity: f32,
    ) {
        let dir = Vec3::new(dir.x, 0.0, dir.z).normalize_or_zero();
        let steps = ((length / 96.0).ceil() as usize).clamp(1, 64);
        if dir == Vec3::ZERO || !self.room((steps + 1) * 2) {
            return;
        }
        let side = Vec3::new(-dir.z, 0.0, dir.x);
        let base = self.vertices.len() as u32;
        for i in 0..=steps {
            let along = from + dir * (length * i as f32 / steps as f32);
            for (offset, v) in [(-1.0f32, -1.0f32), (1.0, 1.0)] {
                let p = along + side * (half_width * offset);
                let p = Vec3::new(p.x, height.height_at(p.x, p.z) + 9.0, p.z);
                self.vertices
                    .push(fx_vertex(p, color, intensity, [0.0, v], MAT_BAND));
            }
        }
        for i in 0..steps as u32 {
            let o = base + i * 2;
            self.indices
                .extend_from_slice(&[o, o + 1, o + 2, o + 2, o + 1, o + 3]);
        }
    }

    /// An arrowhead on the ground at `tip`, pointing along `dir`.
    pub fn ground_arrow(
        &mut self,
        height: &HeightGrid,
        tip: Vec3,
        dir: Vec3,
        size: f32,
        color: [u8; 3],
        intensity: f32,
    ) {
        let dir = Vec3::new(dir.x, 0.0, dir.z).normalize_or_zero();
        if dir == Vec3::ZERO {
            return;
        }
        let side = Vec3::new(-dir.z, 0.0, dir.x);
        for wing in [side, -side] {
            let back = (-dir * 0.8 + wing * 0.6).normalize();
            self.ground_strip(height, tip, back, size, 12.0, color, intensity);
        }
    }
}

fn ring_segments(radius: f32) -> usize {
    ((radius / 40.0) as usize).clamp(24, 128)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_look_name_is_unique_and_resolves_to_itself() {
        for (index, look) in LOOKS.iter().enumerate() {
            assert_eq!(look_by_name(look.name), index, "{}", look.name);
        }
    }

    #[test]
    fn unknown_names_fall_back_to_the_default_look() {
        assert_eq!(look_by_name("no_such_look"), DEFAULT_LOOK);
        assert_eq!(look(9_999).name, LOOKS[DEFAULT_LOOK].name);
    }

    #[test]
    fn lookup_ignores_case() {
        assert_eq!(look_by_name("FIRE_WAVE"), look_by_name("fire_wave"));
    }

    #[test]
    fn spotanim_ids_are_sorted_and_unique() {
        let ids = spotanim_ids();
        assert!(!ids.is_empty());
        assert!(ids.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn a_lob_starts_and_ends_on_its_line_and_peaks_in_the_middle() {
        assert_eq!(arc_offset(100.0, 0.0), 0.0);
        assert_eq!(arc_offset(100.0, 1.0), 0.0);
        assert!((arc_offset(100.0, 0.5) - 100.0).abs() < 1e-4);
        assert_eq!(arc_offset(100.0, 2.0), 0.0);
    }

    #[test]
    fn a_ribbon_has_two_vertices_per_point_and_tapers() {
        let mut mesh = FxMesh::default();
        let points = [Vec3::ZERO, Vec3::X * -10.0, Vec3::X * -20.0];
        mesh.ribbon(&points, 10.0, [255, 0, 0], 1.0, Vec3::Z);
        assert_eq!(mesh.vertices.len(), 6);
        assert_eq!(mesh.indices.len(), 12);
        assert!(mesh.vertices[0].color[3] > mesh.vertices[4].color[3]);
    }

    #[test]
    fn glows_carry_intensity_in_alpha_and_skip_when_dark() {
        let mut mesh = FxMesh::default();
        mesh.glow(Vec3::ZERO, 10.0, [255, 255, 255], 2.5, Vec3::X, Vec3::Y);
        assert_eq!(mesh.vertices.len(), 4);
        assert!((mesh.vertices[0].color[3] - 2.5).abs() < 1e-6);
        assert_eq!(mesh.vertices[0].material, MAT_GLOW);
        mesh.glow(Vec3::ZERO, 10.0, [255, 255, 255], 0.0, Vec3::X, Vec3::Y);
        assert_eq!(mesh.vertices.len(), 4);
    }
}
