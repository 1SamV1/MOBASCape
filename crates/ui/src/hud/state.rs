//! The plain data the interface shows (docs/INTERFACE_STAGE3_PLAN.md,
//! section 3). In stage 3 every value here is made up by `sample.rs`; in
//! stage 4 the client fills the same fields from the match, and nothing in
//! a component has to change.

/// One jungle camp's line in the objectives panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CampState {
    pub name: String,
    /// "Might" or "Swift".
    pub grant: String,
    /// "Alive", "Taken", or a countdown.
    pub state: String,
    /// A quieter second line, shown only when there is one.
    pub detail: Option<String>,
}

/// One lane's structure pips. Two towers, then the inhibitor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaneState {
    pub name: String,
    pub blue_towers: [bool; 2],
    pub blue_inhibitor: bool,
    pub red_towers: [bool; 2],
    pub red_inhibitor: bool,
}

/// A nexus line: shielded, open, or its health as text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NexusState {
    pub name: String,
    pub blue: bool,
    pub state: String,
}

/// The lanes table and the objectives list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectivesState {
    pub lanes: Vec<LaneState>,
    pub nexuses: Vec<NexusState>,
    pub camps: Vec<CampState>,
    /// How many of the objective lines have a second line today.
    pub second_lines: usize,
    /// How many second lines the usual state shows. The budget is measured
    /// on this state; the longest list is what the scale must fit.
    pub usual_second_lines: usize,
}

impl ObjectivesState {
    /// The panel is the reference's head and rule, plus 24 for every line
    /// and second line. Nothing here is a fixed 302 or 398: add a line and
    /// the height follows.
    pub const fn height(lines: usize, second_lines: usize) -> i32 {
        158 + 24 * (lines + second_lines) as i32
    }

    /// The objective lines are the nexuses and the camps. There is no
    /// separate count to fall out of step with the content.
    pub fn lines(&self) -> usize {
        self.nexuses.len() + self.camps.len()
    }

    pub fn usual_height(&self) -> i32 {
        Self::height(self.lines(), self.usual_second_lines)
    }

    pub fn longest_height(&self) -> i32 {
        Self::height(self.lines(), self.second_lines)
    }

    /// What the sample wants on screen: the full list when it is live.
    pub fn wanted_height(&self) -> i32 {
        self.longest_height()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllyState {
    pub name: String,
    pub health: u32,
    pub max_health: u32,
    pub resource: u32,
    pub max_resource: u32,
    pub level: u8,
    pub dead: bool,
    /// Seconds until respawn, when `dead`.
    pub respawn_seconds: Option<u32>,
    /// Theme sprite role for this row's portrait, when one was baked from
    /// the cache. The component falls back to the team-coloured initial.
    pub portrait: Option<String>,
}

/// One figure in the target panel's two columns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetStat {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetState {
    pub name: String,
    /// The tag at the panel's right: "Structure", "Hero", "Minion".
    pub kind: String,
    /// Which team the target belongs to; the name takes its colour.
    pub blue: bool,
    /// The orange line under the health bar. Stage 3 samples it; stage 4
    /// fills it from the selected unit.
    pub description: String,
    pub health: u32,
    pub max_health: u32,
    /// Up to eight figures, drawn two columns of four.
    pub stats: Vec<TargetStat>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KillFeedEntry {
    pub killer: String,
    pub victim: String,
    pub gold: u32,
    pub killer_blue: bool,
    pub victim_blue: bool,
}

/// One item in the six-slot inventory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventoryItem {
    pub name: String,
    /// The item's colour, used when its cache icon is absent.
    pub colour: [u8; 3],
    /// Theme role of the cache-baked inventory icon, when one was made.
    pub icon: Option<String>,
}

/// One item in the shop's list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShopItem {
    pub name: String,
    pub price: u32,
    pub colour: [u8; 3],
    /// Theme role of the cache-baked inventory icon, when one was made.
    pub icon: Option<String>,
    pub stats: Vec<String>,
    /// What this item builds into, when the item system has upgrades.
    pub builds_into: Option<String>,
    /// What it upgrades from (names), owned or not.
    pub builds_from: Vec<String>,
    /// The full cost; `price` is lower when owned parts make it an upgrade.
    pub full_price: u32,
    /// Shop section: 0 starter, 1 early, 2 core, 3 late (see
    /// `hud::shop::TIERS`).
    pub tier: u8,
}

/// The shop overlay. Stage 3 holds one category and our six items; stage 4
/// adds buying and a live gold count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShopState {
    pub open: bool,
    /// The category rows drawn down the left side.
    pub categories: Vec<String>,
    /// The selected category's index into `categories`.
    pub category: usize,
    pub search: String,
    /// True while the search box has the keyboard (draws a caret and
    /// shows `search` as typed text, not a placeholder).
    pub searching: bool,
    pub items: Vec<ShopItem>,
    pub selected: usize,
    /// How far the item list is scrolled, in interface pixels.
    pub scroll: i32,
    pub hint: String,
    /// The delivery control's label, when purchases are waiting or in flight.
    pub delivery: Option<String>,
    /// What selling the selected item pays, when one is in the bag.
    pub sell_price: Option<u32>,
    /// How many of the selected item are in the bag.
    pub owned: u32,
    /// The item Undo would take back (this visit's newest purchase).
    pub undo: Option<String>,
}

/// One source of damage on the death panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DamageSource {
    pub name: String,
    pub amount: u32,
}

/// The death overlay. `total` is taken from the sources, so the bars and the
/// figure cannot disagree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeathState {
    pub respawn_seconds: u32,
    pub killer: String,
    pub killer_blue: bool,
    pub damage: Vec<DamageSource>,
}

impl DeathState {
    pub fn total(&self) -> u32 {
        self.damage.iter().map(|source| source.amount).sum()
    }
}

/// The item tooltip: the title and price, green stat lines, and a passive
/// whose name is orange and whose text wraps in white. `at` is the cursor in
/// window pixels; the tooltip is placed beside it and kept in the window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TooltipState {
    pub title: String,
    pub price: u32,
    pub stats: Vec<String>,
    pub passive: Option<String>,
    pub text: String,
    pub at: (i32, i32),
}

/// Who a chat line comes from, which decides its colours: a system line is
/// dark on the parchment, a match event is blue, and a player's name takes
/// their team's colour with their hero in brackets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatKind {
    System,
    Event,
    Player {
        speaker: String,
        hero: String,
        blue: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatLine {
    pub kind: ChatKind,
    pub text: String,
}

/// What kind of unit a plate belongs to. The three kinds draw at different
/// sizes: heroes carry a name, level and bar, minions a thin bar, structures
/// a wider one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlateKind {
    Hero,
    Camp,
    Minion,
    Structure,
}

/// Which OSRS hitmark art a combat number uses. `Max` is the gold-trimmed
/// maximum-hit splat; `Miss` is the blue zero/blocked splat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitmarkKind {
    Damage,
    Poison,
    Venom,
    Disease,
    Heal,
    Miss,
    Max,
}

impl HitmarkKind {
    /// Theme sprite role suffix.
    pub fn role(self) -> &'static str {
        match self {
            Self::Damage => "damage",
            Self::Poison => "poison",
            Self::Venom => "venom",
            Self::Disease => "disease",
            Self::Heal => "heal",
            Self::Miss => "miss",
            Self::Max => "max",
        }
    }
}

/// One projected hitmark, in window pixels. `stack` offsets hits on the same
/// unit down the body; `tint` cools hits that did not come from the local
/// player, the way OSRS tints other players' splats.
#[derive(Debug, Clone, PartialEq)]
pub struct HitmarkState {
    pub at: (i32, i32),
    pub text: String,
    pub kind: HitmarkKind,
    /// 0..1, used for the fade at the end of the splat's life.
    pub alpha: f32,
    pub stack: u8,
    /// Multiplied over the cache sprite; `None` keeps its original colour.
    pub tint: Option<[u8; 3]>,
}

/// One unit plate, already placed in window pixels by the client's
/// projection. `at` is the unit's head; the plate is drawn above it. The
/// interface draws a plate where it is told and never places one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitPlate {
    pub at: (i32, i32),
    pub kind: PlateKind,
    pub name: String,
    pub level: u8,
    pub health: u32,
    pub max_health: u32,
    pub blue: bool,
    /// Solid overhead-name colour. `None` falls back to the team colour.
    pub name_colour: Option<[u8; 3]>,
    /// Player-killer skull above the name.
    pub pk: bool,
    /// A short warning label such as "NO SAFE", above the name.
    pub tag: Option<String>,
}

/// Everything the interface shows, as plain data. The component slices are
/// kept together here so the client has one thing to fill, not a dozen.
#[derive(Debug, Clone, PartialEq)]
pub struct HudState {
    pub score: ScoreState,
    pub ability: AbilityState,
    pub minimap: MinimapState,
    pub objectives: ObjectivesState,
    pub allies: Vec<AllyState>,
    pub plates: Vec<UnitPlate>,
    pub hitmarks: Vec<HitmarkState>,
    pub target: Option<TargetState>,
    pub inventory: Vec<Option<InventoryItem>>,
    pub gold: u32,
    pub chat: Vec<ChatLine>,
    pub feed: Vec<KillFeedEntry>,
    pub announcement: Option<String>,
    pub tooltip: Option<TooltipState>,
    /// The hovered ability's tooltip, when the mouse is over Q, W, E or R.
    pub ability_tooltip: Option<AbilityTooltipState>,
    pub shop: ShopState,
    pub death: Option<DeathState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScoreState {
    pub clock: String,
    pub blue_kills: u32,
    pub red_kills: u32,
    pub kills: u32,
    pub deaths: u32,
    pub assists: u32,
    pub creep_score: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AbilityState {
    pub health: u32,
    pub max_health: u32,
    pub resource: u32,
    pub max_resource: u32,
    pub level: u8,
    pub experience: u32,
    pub cooldowns: [f32; 4],
    /// Theme sprite role for the champion portrait, when one is baked.
    pub portrait: Option<String>,
    /// Theme sprite roles for the Q, W, E and R icons, when baked.
    pub icons: [Option<String>; 4],
    /// Current rank of each ability, drawn as pips under its slot.
    pub ranks: [u8; 4],
    /// Ranks each ability has (pips drawn per slot).
    pub max_ranks: [u8; 4],
    /// Slots that would take a skill point if clicked: their hotkey glows
    /// gold and the slot shimmers.
    pub upgradable: [bool; 4],
    /// Seconds, for the shimmer animation.
    pub time: f32,
}

/// The ability tooltip shown over a hovered Q, W, E or R slot:
/// "Name: summary", the rank, the numbers now, and every upgrade left.
#[derive(Debug, Clone, PartialEq)]
pub struct AbilityTooltipState {
    /// Hotkey: Q, W, E or R.
    pub key: String,
    pub name: String,
    pub summary: String,
    pub rank: u8,
    pub max_rank: u8,
    pub current: AbilityNumbers,
    /// Ranks still to come, next one first.
    pub upgrades: Vec<AbilityUpgrade>,
    /// What an ability without a damage number does, e.g. "Heal: 25% of
    /// missing health".
    pub note: Option<String>,
    /// "Click to rank up" style hint when a point can be spent here.
    pub hint: Option<String>,
    /// Window pixel the tooltip sits above, centred (the top middle of the
    /// hovered slot's bar).
    pub at: (i32, i32),
}

/// Damage, range and cooldown, already formatted for display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbilityNumbers {
    /// What the first number is: "Damage", "Shield", "Heal" ...
    pub damage_label: String,
    pub damage: String,
    pub range: String,
    pub cooldown: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbilityUpgrade {
    pub rank: u8,
    pub numbers: AbilityNumbers,
    /// Champion level this rank still waits for, when it is locked.
    pub needs_level: Option<u32>,
}

/// A point in the minimap's own 254 by 252 picture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapPoint {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StructureMark {
    pub at: MapPoint,
    pub blue: bool,
}

/// A hero on the minimap: a ring in the team colour, gold for the player.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeroMark {
    pub at: MapPoint,
    pub blue: bool,
    pub player: bool,
    /// Theme role of a small disc portrait drawn inside the ring.
    pub portrait: Option<&'static str>,
}

/// The team's live vision over the map picture: `cols` by `rows` cells,
/// row-major, true where an ally (hero, minion, tower) has line of sight.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VisionMask {
    pub cols: usize,
    pub rows: usize,
    pub seen: Vec<bool>,
}

/// The minimap's picture and marks. Stage 3 fills this with a sample map;
/// stage 4 feeds the client's visible units and camera box.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MinimapState {
    pub lanes: Vec<Vec<MapPoint>>,
    pub river: Vec<MapPoint>,
    pub bases: Vec<StructureMark>,
    pub allies: Vec<MapPoint>,
    pub enemies: Vec<MapPoint>,
    pub camps: Vec<MapPoint>,
    pub structures: Vec<StructureMark>,
    /// Heroes (allies, and enemies the team can see).
    pub heroes: Vec<HeroMark>,
    /// The team's vision; `None` draws the whole map lit.
    pub vision: Option<VisionMask>,
    /// Left, top, width and height in the map picture.
    pub camera: (i32, i32, i32, i32),
    /// The screen's footprint on the ground (top left, top right, bottom
    /// right, bottom left), unclamped so it can run off the picture. When
    /// set it is drawn instead of the `camera` rectangle.
    pub view: Option<[MapPoint; 4]>,
}
