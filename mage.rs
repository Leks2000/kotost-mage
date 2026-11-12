// Kotost Mage: a Horizon native mod (horizon-mixins module, put into the game's `game` module, so it reaches the
// game's own helpers). Track "Mage", stage M1:
// - mana (server side, regenerates), a bar and the spells' cooldowns in the HUD;
// - fireball (R): a flying ball of fire with a trail and light, a small blast that breaks nothing;
// - shadow step (G): a jump 5-12 blocks where the player looks, smoke at both ends;
// - meteor (Z): a red ring on the ground, a burning rock from the sky, a crater that takes a town with it,
//   flying debris, a shockwave, a smoke column, ash and a dark sky for a while;
// - the mage's outfit drawn on the player model: a hat with a wide brim and a bent tip, a dress with a bell
//   skirt that swings on a spring while walking, a staff with a glowing crystal (first person too, with
//   kotost_body's own body). They are items of the content mod `kotost_mage` (content/): the hat is drawn while
//   it is worn on the head, the dress while it is worn on the chest, the staff while it is held; spells
//   are cast only with the staff in a hand, and the mage's things give mana and faster cooldowns (less to
//   another class).
// Every number is in `config/kotost-mage.txt` in the game's folder (the server's numbers in the server's), read
// again when it changes: no rebuild.

use super::Game;
use horizon_client::mod_api::NativeServerPlayer;
use horizon_client::particles::{ParticleOptions, ParticleType};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime};

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

// ===================================================================================================
// Settings
// ===================================================================================================

/// The mage's things: items of the content mod `kotost_mage` (content/data/kotost_mage/horizon/item).
pub const ITEM_HAT: &str = "kotost_mage_items:mage_hat";
pub const ITEM_DRESS: &str = "kotost_mage_items:mage_dress";
pub const ITEM_STAFF: &str = "kotost_mage_items:mage_staff";
/// The magic items: one of them in a hand shows the spells and the mana bar and lets spells be cast
/// (`need_staff`). A new magic item: add its id here.
pub const MAGIC_ITEMS: &[&str] = &[ITEM_STAFF];
/// The class the mage's things belong to (`class` in the server's config).
const CLASS_MAGE: &str = "mage";

#[derive(Clone, Copy, Debug)]
pub struct Tune {
    pub debug: bool,
    pub mana_max: f32,
    pub mana_regen: f32,
    pub need_staff: bool,
    pub hat_mana: f32,
    pub hat_haste: f32,
    pub dress_mana: f32,
    pub dress_haste: f32,
    pub staff_mana: f32,
    pub staff_haste: f32,
    pub gear_foreign: f32,
    pub fire_cost: f32,
    pub fire_cooldown: f32,
    pub fire_speed: f32,
    pub fire_gravity: f32,
    pub fire_radius: f32,
    pub fire_life: f32,
    pub fire_ignite: bool,
    pub step_cost: f32,
    pub step_cooldown: f32,
    pub step_min: f32,
    pub step_max: f32,
    pub meteor_cost: f32,
    pub meteor_cooldown: f32,
    pub meteor_range: f32,
    pub meteor_fall: f32,
    pub meteor_radius: f32,
    pub meteor_depth: f32,
    pub meteor_clear: f32,
    pub meteor_rim: f32,
    pub meteor_blocks: bool,
    pub meteor_debris: f32,
    pub meteor_throw: f32,
    pub meteor_fire: f32,
    pub meteor_lava: f32,
    pub meteor_damage: f32,
    pub meteor_push: f32,
    pub meteor_smoke: f32,
    pub meteor_size: f32,
    pub frost_cost: f32,
    pub frost_cooldown: f32,
    pub frost_radius: f32,
    pub frost_seconds: f32,
    pub frost_damage: f32,
    pub chain_cost: f32,
    pub chain_cooldown: f32,
    pub chain_range: f32,
    pub chain_jump: f32,
    pub chain_count: f32,
    pub chain_damage: f32,
    pub chain_bolt: bool,
    pub nova_cost: f32,
    pub nova_cooldown: f32,
    pub nova_rise: f32,
    pub nova_charge: f32,
    pub nova_radius: f32,
    pub nova_damage: f32,
    pub nova_blocks: bool,
    pub nova_fire: f32,
    pub shake: f32,
    pub flash: f32,
    pub dust: f32,
    pub dust_seconds: f32,
    pub dust_fog: f32,
    pub hud: bool,
    pub skirt_k: f32,
    pub skirt_c: f32,
    pub skirt_kick: f32,
    pub skirt_trail: f32,
    pub skirt_turn: f32,
    pub skirt_jump: f32,
    pub skirt_max: f32,
    pub skirt_len: f32,
    pub skirt_width: f32,
    pub slit: f32,
    pub stockings: bool,
    pub cast_fx: bool,
    pub cast_kick: f32,
    pub recoil: f32,
    pub legs_push: f32,
    pub stance: f32,
    pub sparks: f32,
    pub pillar: bool,
    /// Seconds each spell gathers before it goes off (by spell id - 1), all scaled by `windup_scale`; the
    /// worn things and the spell's mastery shorten them as they do the cooldowns.
    pub windup: [f32; SPELLS],
    pub windup_scale: f32,
    /// Mastery: each spell levels up by being cast (`mastery_casts` casts for level 2, twice that more for
    /// level 3...), up to `mastery_max`; each level past the first casts and recharges `mastery_speed` %
    /// faster.
    pub mastery_max: f32,
    pub mastery_casts: f32,
    pub mastery_speed: f32,
    /// The fireball locks on a mob within this many degrees of the aim, and turns toward it this many
    /// degrees a tick.
    pub fire_assist: f32,
    pub fire_homing: f32,
    /// Aim previews while a spell gathers (the fireball's path, the meteor's ring, the step's landing).
    pub aim: bool,
}

impl Default for Tune {
    fn default() -> Self {
        Tune {
            debug: false,
            mana_max: 100.0,
            mana_regen: 4.0,
            need_staff: true,
            hat_mana: 25.0,
            hat_haste: 10.0,
            dress_mana: 50.0,
            dress_haste: 5.0,
            staff_mana: 0.0,
            staff_haste: 15.0,
            gear_foreign: 0.5,
            fire_cost: 12.0,
            fire_cooldown: 0.6,
            fire_speed: 1.3,
            fire_gravity: 0.01,
            fire_radius: 2.0,
            fire_life: 3.0,
            fire_ignite: false,
            step_cost: 20.0,
            step_cooldown: 1.5,
            step_min: 5.0,
            step_max: 12.0,
            meteor_cost: 90.0,
            meteor_cooldown: 30.0,
            meteor_range: 160.0,
            meteor_fall: 1.5,
            meteor_radius: 36.0,
            meteor_depth: 14.0,
            meteor_clear: 56.0,
            meteor_rim: 4.0,
            meteor_blocks: true,
            meteor_debris: 600.0,
            meteor_throw: 1.0,
            meteor_fire: 0.06,
            meteor_lava: 0.35,
            meteor_damage: 60.0,
            meteor_push: 2.5,
            meteor_smoke: 14.0,
            meteor_size: 9.0,
            frost_cost: 40.0,
            frost_cooldown: 12.0,
            frost_radius: 12.0,
            frost_seconds: 4.0,
            frost_damage: 8.0,
            chain_cost: 30.0,
            chain_cooldown: 4.0,
            chain_range: 24.0,
            chain_jump: 10.0,
            chain_count: 5.0,
            chain_damage: 7.0,
            chain_bolt: true,
            nova_cost: 70.0,
            nova_cooldown: 25.0,
            nova_rise: 3.0,
            nova_charge: 2.0,
            nova_radius: 10.0,
            nova_damage: 30.0,
            nova_blocks: true,
            nova_fire: 0.45,
            shake: 1.0,
            flash: 1.0,
            dust: 1.0,
            dust_seconds: 45.0,
            dust_fog: 40.0,
            hud: true,
            skirt_k: 0.9,
            skirt_c: 0.35,
            skirt_kick: 1.6,
            skirt_trail: 9.0,
            skirt_turn: 0.25,
            skirt_jump: 1.0,
            skirt_max: 28.0,
            skirt_len: 12.6,
            skirt_width: 1.0,
            slit: 1.0,
            stockings: true,
            cast_fx: true,
            cast_kick: 1.0,
            recoil: 1.0,
            legs_push: 1.0,
            stance: 1.0,
            sparks: 1.0,
            pillar: true,
            windup: [0.45, 0.3, 1.3, 0.9, 0.7, 0.5],
            windup_scale: 1.0,
            mastery_max: 10.0,
            mastery_casts: 5.0,
            mastery_speed: 4.0,
            fire_assist: 6.0,
            fire_homing: 3.0,
            aim: true,
        }
    }
}

const DEFAULT_FILE: &str = r#"# Kotost Mage settings. The game re-reads this file by itself (twice a second): no restart needed.
# key = value; anything after # is a comment; delete a line to get its default back.
# The spells' numbers (mana, damage, radii) are the server's: on your own server, edit the file in the server's folder.
mana_max = 100
debug = 0              # 1: write kotost-mage-debug.log (what is cast and why not)

# --- keys ---
key_fireball = r       # fireball
key_step = g           # shadow step
key_meteor = z         # meteor
key_frost = x          # absolute zero
key_chain = c          # chain lightning
key_nova = n           # supernova (in Horizon B is the emote wheel)

# --- mana ---

mana_regen = 4         # mana per second

# --- the mage's things (the server counts them) ---
# Items: kotost_mage_items:mage_hat (hat, on the head), kotost_mage_items:mage_dress (dress, on the chest),
# kotost_mage_items:mage_staff (staff, in a hand). The clothes show only while worn, the staff while held.
need_staff = 1         # spells only with a magic item (the staff) in a hand (0: cast without it too)
hat_mana = 25          # hat: + to the mana maximum
hat_haste = 10         # hat: faster cooldowns, %
dress_mana = 50        # dress: + to the mana maximum
dress_haste = 5        # dress: faster cooldowns, %
staff_mana = 0         # staff: + to the mana maximum
staff_haste = 15       # staff: faster cooldowns, %
gear_foreign = 0.5     # share of the mage items' bonus on a player of another class
class = mage           # players' class by default: mage | knight | archer | merchant
# class.Name = knight  # one player's class (a line for each)

# --- wind-up and mastery (the server counts them) ---
# A spell gathers first (the mage winds up, the circle grows), then goes off. The mage's things and the
# spell's mastery shorten both the wind-up and the cooldown.
fire_windup = 0.45     # fireball: seconds of wind-up
step_windup = 0.3      # shadow step
meteor_windup = 1.3    # meteor
frost_windup = 0.9     # absolute zero
chain_windup = 0.7     # chain lightning
nova_windup = 0.5      # supernova (then its charge, nova_charge)
windup_scale = 1.0     # all wind-ups at once (0: they go off at once, 2: twice as long)
mastery_max = 10       # levels of each spell
mastery_casts = 5      # casts to level 2; each next level needs that many more
mastery_speed = 4      # each level after the first: wind-up and cooldown faster, %
fire_assist = 6        # the fireball locks on a mob within this many degrees of the crosshair
fire_homing = 3        # and turns to it, degrees per tick (0: flies straight)
aim = 1                # aim preview while winding up (the ball's path, the meteor's circle, the step's spot), per player

# --- fireball ---
fire_cost = 12
fire_cooldown = 0.6    # seconds
fire_speed = 1.3       # blocks per tick
fire_gravity = 0.01    # how much the ball drops
fire_radius = 2        # blast radius (breaks no blocks)
fire_life = 3          # seconds of flight before it fades
fire_ignite = 0        # 1: set fire around the blast

# --- shadow step ---
step_cost = 20
step_cooldown = 1.5
step_min = 5           # closer: no jump
step_max = 12          # further: no jump

# --- meteor ---
meteor_cost = 90
meteor_cooldown = 30
meteor_range = 160     # how far you can aim
meteor_fall = 1.5      # seconds from the circle to the impact
meteor_radius = 36     # crater radius (up to 96)
meteor_depth = 14      # depth in the middle
meteor_clear = 56      # how many blocks above ground it sweeps away (houses, trees)
meteor_rim = 4         # height of the rim around it
meteor_blocks = 1      # 0: no crater, only the blast and the effects
meteor_debris = 600    # flying blocks (up to 1000)
meteor_throw = 1.0     # how far they are thrown
meteor_fire = 0.06     # share of fire around the crater
meteor_lava = 0.35     # share of lava at the bottom in the middle
meteor_damage = 60     # damage to mobs in the middle (less towards the edge)
meteor_push = 2.5      # shock wave: knockback strength
meteor_smoke = 14      # seconds of smoke and ash
meteor_size = 9        # size of the rock in the sky (blocks)

# --- absolute zero ---
frost_cost = 40
frost_cooldown = 12
frost_radius = 12      # ring of ice: water freezes, grass under snow
frost_seconds = 4      # how long mobs stay in the ice
frost_damage = 8       # damage when the ice shatters

# --- chain lightning ---
chain_cost = 30
chain_cooldown = 4
chain_range = 24       # to the first target
chain_jump = 10        # jump to the next one
chain_count = 5        # how many targets
chain_damage = 7       # damage to each
chain_bolt = 1         # a real lightning bolt from the sky into the last one

# --- supernova ---
nova_cost = 70
nova_cooldown = 25
nova_rise = 3          # how many blocks the mage rises
nova_charge = 2        # seconds of charge
nova_radius = 10       # blast radius
nova_damage = 30       # damage in the middle
nova_blocks = 1        # 0: no crater
nova_fire = 0.45       # share of fire in the ring

# --- screen (each player's own) ---
shake = 1.0            # shake (0: none)
flash = 1.0            # flash (0: none)
dust = 1.0             # brown sky after a meteor (0: none)
dust_seconds = 45
dust_fog = 40          # how far you see in the dust, blocks
hud = 1                # mana bar

# --- the mage's clothes (drawn by the worn items, see "the mage's things") ---
skirt_k = 0.9          # skirt spring stiffness
skirt_c = 0.35         # damping (lower: swings longer)
skirt_kick = 1.6       # kick of each step
skirt_trail = 9        # how far it trails back while running, degrees
skirt_turn = 0.25      # swing on turns
skirt_jump = 1.0       # the hem flying up on a jump
skirt_max = 28         # swing limit, degrees
skirt_len = 12.6       # skirt length from the waist, model pixels (12.6: to the ankles)
skirt_width = 1.0      # skirt width at the hem
slit = 1.0             # side slit up to the thigh (0: none, 1.5: wider)
stockings = 1          # stockings with garters, and boots
legs_push = 1.0        # the legs push the hem (0: they pass through)
stance = 1.0           # standing, the leg under the slit steps forward (0: stands straight)

# --- casting (each player's own) ---
cast_fx = 1            # circle of runes at the feet, ring, sparks (0: none)
cast_kick = 1.0        # the hem and the hat fly up on a cast
recoil = 1.0           # camera recoil on your own cast (0: none)
sparks = 1.0           # how many sparks on a cast (0: none, 2: twice as thick)
pillar = 1             # pillar of light and ring of runes at the staff on a cast
"#;

/// The players' classes as the server's config names them: everyone's (`class`), then by player
/// (`class.<name>`, names in lower case).
#[derive(Clone, Debug)]
struct Classes {
    default: String,
    by_name: Vec<(String, String)>,
}

impl Default for Classes {
    fn default() -> Self {
        Classes {
            default: CLASS_MAGE.to_owned(),
            by_name: Vec::new(),
        }
    }
}

impl Classes {
    fn of(&self, name: &str) -> &str {
        let name = name.to_lowercase();
        self.by_name
            .iter()
            .find(|(n, _)| *n == name)
            .map_or(self.default.as_str(), |(_, class)| class.as_str())
    }
}

struct Settings {
    tune: Tune,
    keys: [String; 6],
    classes: Classes,
    stamp: Option<SystemTime>,
    checked: Instant,
}

static SETTINGS: Mutex<Option<Settings>> = Mutex::new(None);

fn config_path() -> std::path::PathBuf {
    horizon_client::logging::game_home().join("config").join("kotost-mage.txt")
}

fn parse(text: &str) -> (Tune, [String; 6], Classes) {
    let mut t = Tune::default();
    let mut keys = ["r", "g", "z", "x", "c", "n"].map(str::to_owned);
    let mut classes = Classes::default();
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or("");
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let (key, value) = (key.trim(), value.trim());
        match key {
            "key_fireball" => keys[0] = value.to_lowercase(),
            "key_step" => keys[1] = value.to_lowercase(),
            "key_meteor" => keys[2] = value.to_lowercase(),
            "key_frost" => keys[3] = value.to_lowercase(),
            "key_chain" => keys[4] = value.to_lowercase(),
            "key_nova" => keys[5] = value.to_lowercase(),
            "class" if !value.is_empty() => classes.default = value.to_lowercase(),
            _ => {
                if let Some(name) = key.strip_prefix("class.")
                    && !name.is_empty()
                    && !value.is_empty()
                    && classes.by_name.len() < 1024
                {
                    classes.by_name.push((name.trim().to_lowercase(), value.to_lowercase()));
                }
            }
        }
        let on = matches!(value, "1" | "on" | "true" | "yes");
        let n = value.parse::<f32>().ok().filter(|n| n.is_finite());
        match key {
            "debug" => t.debug = on,
            "need_staff" => t.need_staff = on,
            "fire_ignite" => t.fire_ignite = on,
            "meteor_blocks" => t.meteor_blocks = on,
            "hud" => t.hud = on,
            "stockings" => t.stockings = on,
            "cast_fx" => t.cast_fx = on,
            "pillar" => t.pillar = on,
            "chain_bolt" => t.chain_bolt = on,
            "nova_blocks" => t.nova_blocks = on,
            "aim" => t.aim = on,
            _ => {}
        }
        let Some(n) = n else { continue };
        match key {
            "mana_max" => t.mana_max = n.clamp(1.0, 10_000.0),
            "mana_regen" => t.mana_regen = n.clamp(0.0, 1000.0),
            "hat_mana" => t.hat_mana = n.clamp(0.0, 10_000.0),
            "hat_haste" => t.hat_haste = n.clamp(0.0, 90.0),
            "dress_mana" => t.dress_mana = n.clamp(0.0, 10_000.0),
            "dress_haste" => t.dress_haste = n.clamp(0.0, 90.0),
            "staff_mana" => t.staff_mana = n.clamp(0.0, 10_000.0),
            "staff_haste" => t.staff_haste = n.clamp(0.0, 90.0),
            "gear_foreign" => t.gear_foreign = n.clamp(0.0, 1.0),
            "fire_cost" => t.fire_cost = n.clamp(0.0, 10_000.0),
            "fire_cooldown" => t.fire_cooldown = n.clamp(0.0, 600.0),
            "fire_speed" => t.fire_speed = n.clamp(0.1, 4.0),
            "fire_gravity" => t.fire_gravity = n.clamp(0.0, 0.2),
            "fire_radius" => t.fire_radius = n.clamp(0.5, 8.0),
            "fire_life" => t.fire_life = n.clamp(0.5, 20.0),
            "step_cost" => t.step_cost = n.clamp(0.0, 10_000.0),
            "step_cooldown" => t.step_cooldown = n.clamp(0.0, 600.0),
            "step_min" => t.step_min = n.clamp(0.0, 64.0),
            "step_max" => t.step_max = n.clamp(1.0, 64.0),
            "meteor_cost" => t.meteor_cost = n.clamp(0.0, 10_000.0),
            "meteor_cooldown" => t.meteor_cooldown = n.clamp(0.0, 3600.0),
            "meteor_range" => t.meteor_range = n.clamp(8.0, 400.0),
            "meteor_fall" => t.meteor_fall = n.clamp(0.3, 10.0),
            "meteor_radius" => t.meteor_radius = n.clamp(3.0, 96.0),
            "meteor_depth" => t.meteor_depth = n.clamp(1.0, 64.0),
            "meteor_clear" => t.meteor_clear = n.clamp(0.0, 200.0),
            "meteor_rim" => t.meteor_rim = n.clamp(0.0, 16.0),
            "meteor_debris" => t.meteor_debris = n.clamp(0.0, 1000.0),
            "meteor_throw" => t.meteor_throw = n.clamp(0.0, 3.0),
            "meteor_fire" => t.meteor_fire = n.clamp(0.0, 1.0),
            "meteor_lava" => t.meteor_lava = n.clamp(0.0, 1.0),
            "meteor_damage" => t.meteor_damage = n.clamp(0.0, 10_000.0),
            "meteor_push" => t.meteor_push = n.clamp(0.0, 4.0),
            "meteor_smoke" => t.meteor_smoke = n.clamp(0.0, 120.0),
            "meteor_size" => t.meteor_size = n.clamp(1.0, 40.0),
            "frost_cost" => t.frost_cost = n.clamp(0.0, 10_000.0),
            "frost_cooldown" => t.frost_cooldown = n.clamp(0.0, 3600.0),
            "frost_radius" => t.frost_radius = n.clamp(2.0, 48.0),
            "frost_seconds" => t.frost_seconds = n.clamp(0.5, 60.0),
            "frost_damage" => t.frost_damage = n.clamp(0.0, 10_000.0),
            "chain_cost" => t.chain_cost = n.clamp(0.0, 10_000.0),
            "chain_cooldown" => t.chain_cooldown = n.clamp(0.0, 3600.0),
            "chain_range" => t.chain_range = n.clamp(2.0, 96.0),
            "chain_jump" => t.chain_jump = n.clamp(1.0, 48.0),
            "chain_count" => t.chain_count = n.clamp(1.0, 32.0),
            "chain_damage" => t.chain_damage = n.clamp(0.0, 10_000.0),
            "nova_cost" => t.nova_cost = n.clamp(0.0, 10_000.0),
            "nova_cooldown" => t.nova_cooldown = n.clamp(0.0, 3600.0),
            "nova_rise" => t.nova_rise = n.clamp(0.0, 16.0),
            "nova_charge" => t.nova_charge = n.clamp(0.2, 10.0),
            "nova_radius" => t.nova_radius = n.clamp(2.0, 32.0),
            "nova_damage" => t.nova_damage = n.clamp(0.0, 10_000.0),
            "nova_fire" => t.nova_fire = n.clamp(0.0, 1.0),
            "shake" => t.shake = n.clamp(0.0, 3.0),
            "flash" => t.flash = n.clamp(0.0, 1.0),
            "dust" => t.dust = n.clamp(0.0, 1.0),
            "dust_seconds" => t.dust_seconds = n.clamp(0.0, 600.0),
            "dust_fog" => t.dust_fog = n.clamp(8.0, 512.0),
            "skirt_k" => t.skirt_k = n.clamp(0.05, 4.0),
            "skirt_c" => t.skirt_c = n.clamp(0.0, 4.0),
            "skirt_kick" => t.skirt_kick = n.clamp(0.0, 10.0),
            "skirt_trail" => t.skirt_trail = n.clamp(-45.0, 45.0),
            "skirt_turn" => t.skirt_turn = n.clamp(0.0, 4.0),
            "skirt_jump" => t.skirt_jump = n.clamp(0.0, 5.0),
            "skirt_max" => t.skirt_max = n.clamp(0.0, 80.0),
            "skirt_len" => t.skirt_len = n.clamp(4.0, 14.0),
            "skirt_width" => t.skirt_width = n.clamp(0.5, 2.0),
            "slit" => t.slit = n.clamp(0.0, 2.0),
            "legs_push" => t.legs_push = n.clamp(0.0, 2.0),
            "stance" => t.stance = n.clamp(0.0, 1.5),
            "sparks" => t.sparks = n.clamp(0.0, 3.0),
            "cast_kick" => t.cast_kick = n.clamp(0.0, 4.0),
            "recoil" => t.recoil = n.clamp(0.0, 3.0),
            "fire_windup" => t.windup[0] = n.clamp(0.0, 5.0),
            "step_windup" => t.windup[1] = n.clamp(0.0, 5.0),
            "meteor_windup" => t.windup[2] = n.clamp(0.0, 5.0),
            "frost_windup" => t.windup[3] = n.clamp(0.0, 5.0),
            "chain_windup" => t.windup[4] = n.clamp(0.0, 5.0),
            "nova_windup" => t.windup[5] = n.clamp(0.0, 5.0),
            "windup_scale" => t.windup_scale = n.clamp(0.0, 3.0),
            "mastery_max" => t.mastery_max = n.clamp(1.0, 50.0),
            "mastery_casts" => t.mastery_casts = n.clamp(1.0, 1000.0),
            "mastery_speed" => t.mastery_speed = n.clamp(0.0, 10.0),
            "fire_assist" => t.fire_assist = n.clamp(0.0, 20.0),
            "fire_homing" => t.fire_homing = n.clamp(0.0, 20.0),
            _ => {}
        }
    }
    (t, keys, classes)
}

fn settings<R>(f: impl FnOnce(&Settings) -> R) -> R {
    let mut cache = lock(&SETTINGS);
    let now = Instant::now();
    let due = cache
        .as_ref()
        .is_none_or(|c| now.duration_since(c.checked) > Duration::from_millis(500));
    if due {
        let path = config_path();
        let stamp = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
        match cache.as_mut() {
            Some(c) if c.stamp == stamp => c.checked = now,
            _ => {
                let (tune, keys, classes) = match std::fs::read_to_string(&path) {
                    Ok(text) => parse(&text),
                    Err(_) => {
                        if !path.exists()
                            && let Some(dir) = path.parent()
                            && std::fs::create_dir_all(dir).is_ok()
                        {
                            let _ = std::fs::write(&path, DEFAULT_FILE);
                        }
                        parse("")
                    }
                };
                *cache = Some(Settings {
                    tune,
                    keys,
                    classes,
                    stamp,
                    checked: now,
                });
            }
        }
    }
    f(cache.as_ref().expect("filled above"))
}

pub fn tune() -> Tune {
    settings(|s| s.tune)
}

fn debug(what: &str) {
    if !tune().debug {
        return;
    }
    use std::io::Write;
    let path = horizon_client::logging::game_home().join("kotost-mage-debug.log");
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "{what}");
    }
}

// ===================================================================================================
// Small things
// ===================================================================================================

const CH_CAST: &str = "kotost_mage:cast";
const CH_MANA: &str = "kotost_mage:mana";
const CH_FX: &str = "kotost_mage:fx";

const SPELL_FIRE: u8 = 1;
const SPELL_STEP: u8 = 2;
const SPELL_METEOR: u8 = 3;
const SPELL_FROST: u8 = 4;
const SPELL_CHAIN: u8 = 5;
const SPELL_NOVA: u8 = 6;
const SPELLS: usize = 6;

const FX_FIRE_SPAWN: u8 = 1;
const FX_FIRE_END: u8 = 2;
const FX_STEP: u8 = 3;
const FX_METEOR_MARK: u8 = 4;
const FX_METEOR_IMPACT: u8 = 5;
const FX_FAIL: u8 = 6;
const FX_FROST: u8 = 7;
const FX_CHAIN: u8 = 8;
const FX_NOVA_START: u8 = 9;
const FX_NOVA_BOOM: u8 = 10;
const FX_LIFT: u8 = 11;
/// A spell went off: who (entity id), which, where they stand, the pose's seconds.
const FX_CAST: u8 = 12;
/// Chain lightning's strike from the sky: where.
const FX_SKYBOLT: u8 = 13;
/// Since 0.6: a gathered spell went off (or fizzled): caster's entity id, spell, 1 if it went off.
const FX_RELEASE: u8 = 14;
/// Since 0.6: a fireball turned toward its target: id, where, its velocity now.
const FX_FIRE_STEER: u8 = 15;

struct Writer(Vec<u8>);

impl Writer {
    fn new(kind: u8) -> Self {
        Writer(vec![kind])
    }
    fn u32(mut self, v: u32) -> Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }
    fn f32(mut self, v: f32) -> Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }
    fn f64(mut self, v: f64) -> Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }
    fn v3(self, v: [f64; 3]) -> Self {
        self.f64(v[0]).f64(v[1]).f64(v[2])
    }
}

struct Reader<'a> {
    data: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn take<const N: usize>(&mut self) -> Option<[u8; N]> {
        let bytes = self.data.get(self.at..self.at + N)?;
        self.at += N;
        bytes.try_into().ok()
    }
    fn u8(&mut self) -> Option<u8> {
        self.take::<1>().map(|b| b[0])
    }
    fn u32(&mut self) -> Option<u32> {
        self.take::<4>().map(u32::from_le_bytes)
    }
    fn f32(&mut self) -> Option<f32> {
        self.take::<4>().map(f32::from_le_bytes).filter(|v| v.is_finite())
    }
    fn f64(&mut self) -> Option<f64> {
        self.take::<8>().map(f64::from_le_bytes).filter(|v| v.is_finite())
    }
    fn v3(&mut self) -> Option<[f64; 3]> {
        Some([self.f64()?, self.f64()?, self.f64()?])
    }
}

/// xorshift64*: a number in [0, 1).
fn rand01(rng: &mut u64) -> f64 {
    let mut x = if *rng == 0 { 0x9E37_79B9_7F4A_7C15 } else { *rng };
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    *rng = x;
    (x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11) as f64 / (1u64 << 53) as f64
}

/// A steady hash of a column, in [-1, 1].
fn noise(x: i32, z: i32, seed: u64) -> f64 {
    let mut h = (x as i64 as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (z as i64 as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
        ^ seed;
    h ^= h >> 31;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 27;
    (h >> 11) as f64 / (1u64 << 52) as f64 - 1.0
}

fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn mul(a: [f64; 3], k: f64) -> [f64; 3] {
    [a[0] * k, a[1] * k, a[2] * k]
}
fn len(a: [f64; 3]) -> f64 {
    (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt()
}
fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    len(sub(a, b))
}

fn dimension_floor(dim: &str) -> i32 {
    if dim == "minecraft:overworld" { -64 } else { 0 }
}

// ===================================================================================================
// Server
// ===================================================================================================

/// What a player has on of the mage's things, and whether the player is of their class.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
struct Gear {
    hat: bool,
    dress: bool,
    staff: bool,
    /// A magic item (`MAGIC_ITEMS`) in a hand.
    magic: bool,
    mage: bool,
}

const GEAR_HAT: u8 = 1;
const GEAR_DRESS: u8 = 2;
const GEAR_STAFF: u8 = 4;
const GEAR_MAGE: u8 = 8;
const GEAR_MAGIC: u8 = 16;

impl Gear {
    fn bits(self) -> u8 {
        u8::from(self.hat) * GEAR_HAT
            | u8::from(self.dress) * GEAR_DRESS
            | u8::from(self.staff) * GEAR_STAFF
            | u8::from(self.mage) * GEAR_MAGE
            | u8::from(self.magic) * GEAR_MAGIC
    }

    /// What the things give their wearer: mana added to the maximum, and the factor of every cooldown
    /// (1: as the config says). Another class gets `gear_foreign` of it.
    fn power(self, t: &Tune) -> (f32, f32) {
        let share = if self.mage { 1.0 } else { t.gear_foreign };
        let on = |worn: bool, v: f32| if worn { v } else { 0.0 };
        let mana = on(self.hat, t.hat_mana) + on(self.dress, t.dress_mana) + on(self.staff, t.staff_mana);
        let haste = on(self.hat, t.hat_haste) + on(self.dress, t.dress_haste) + on(self.staff, t.staff_haste);
        (mana * share, (1.0 - haste * share / 100.0).clamp(0.1, 1.0))
    }
}

fn wears(game: &Game<'_>, uuid: u128, slot: horizon_client::mod_api::EquipmentSlot, item: &str) -> bool {
    game.native_player_equipment(uuid, slot).is_some_and(|s| &*s.item == item)
}

/// The staff in either hand.
fn holds_staff(game: &Game<'_>, uuid: u128) -> bool {
    use horizon_client::mod_api::EquipmentSlot as Slot;
    wears(game, uuid, Slot::MainHand, ITEM_STAFF) || wears(game, uuid, Slot::OffHand, ITEM_STAFF)
}

/// A magic item (`MAGIC_ITEMS`) in either hand.
fn holds_magic(game: &Game<'_>, uuid: u128) -> bool {
    use horizon_client::mod_api::EquipmentSlot as Slot;
    [Slot::MainHand, Slot::OffHand].into_iter().any(|slot| {
        game.native_player_equipment(uuid, slot).is_some_and(|s| MAGIC_ITEMS.contains(&&*s.item))
    })
}

fn gear_of(game: &Game<'_>, p: &NativeServerPlayer) -> Gear {
    use horizon_client::mod_api::EquipmentSlot as Slot;
    Gear {
        hat: wears(game, p.uuid, Slot::Head, ITEM_HAT),
        dress: wears(game, p.uuid, Slot::Chest, ITEM_DRESS),
        staff: holds_staff(game, p.uuid),
        magic: holds_magic(game, p.uuid),
        mage: settings(|s| s.classes.of(&p.name) == CLASS_MAGE),
    }
}

struct Caster {
    mana: f32,
    /// The server tick each spell is ready again, by spell id.
    ready: [u64; SPELLS + 1],
    sent: Option<(i32, [i32; SPELLS], i32)>,
    /// The mage's things on the player, looked at every few ticks.
    gear: Gear,
    /// A spell gathering: which, the server tick it goes off.
    charge: Option<(u8, u64)>,
}

impl Caster {
    fn new(t: &Tune) -> Self {
        Caster {
            mana: t.mana_max,
            ready: [0; SPELLS + 1],
            sent: None,
            gear: Gear::default(),
            charge: None,
        }
    }
}

/// A spell's level (1..=`mastery_max`) after `casts` casts, and how far it is to the next (0..1).
fn mastery_level(casts: u32, t: &Tune) -> (u32, f32) {
    let per = t.mastery_casts.max(1.0);
    let max = t.mastery_max.max(1.0) as u32;
    let (mut level, mut left) = (1u32, casts as f32);
    while level < max && left >= per * level as f32 {
        left -= per * level as f32;
        level += 1;
    }
    let progress = if level >= max { 1.0 } else { left / (per * level as f32) };
    (level, progress)
}

/// How long a spell of `level` takes to gather and recharge, against level 1.
fn mastery_speed(level: u32, t: &Tune) -> f32 {
    (1.0 - t.mastery_speed / 100.0 * level.saturating_sub(1) as f32).max(0.4)
}

/// The players' casts of each spell, kept in `config/kotost-mage-mastery.txt` beside the server's settings:
/// one line a player, `<uuid hex> <name> <casts of each spell>`.
#[derive(Default)]
struct Mastery {
    loaded: bool,
    dirty: bool,
    casts: HashMap<u128, (String, [u32; SPELLS])>,
}

fn mastery_path() -> std::path::PathBuf {
    horizon_client::logging::game_home().join("config").join("kotost-mage-mastery.txt")
}

impl Mastery {
    fn load(&mut self) {
        if self.loaded {
            return;
        }
        self.loaded = true;
        let Ok(text) = std::fs::read_to_string(mastery_path()) else {
            return;
        };
        for line in text.lines() {
            let mut w = line.split_whitespace();
            let (Some(uuid), Some(name)) = (w.next().and_then(|u| u128::from_str_radix(u, 16).ok()), w.next()) else {
                continue;
            };
            let mut casts = [0u32; SPELLS];
            for c in &mut casts {
                *c = w.next().and_then(|v| v.parse().ok()).unwrap_or(0);
            }
            self.casts.insert(uuid, (name.to_owned(), casts));
        }
    }

    fn of(&mut self, uuid: u128) -> [u32; SPELLS] {
        self.load();
        self.casts.get(&uuid).map_or([0; SPELLS], |c| c.1)
    }

    fn add(&mut self, uuid: u128, name: &str, spell: u8) {
        self.load();
        let e = self.casts.entry(uuid).or_insert_with(|| (name.to_owned(), [0; SPELLS]));
        e.0 = name.to_owned();
        if let Some(c) = e.1.get_mut(usize::from(spell.max(1)) - 1) {
            *c = c.saturating_add(1);
        }
        self.dirty = true;
    }

    fn save(&mut self) {
        if !self.dirty {
            return;
        }
        self.dirty = false;
        let mut text = String::from("# Kotost Mage: how many times each player cast each spell (fireball step meteor frost chain nova)\n");
        let mut rows: Vec<_> = self.casts.iter().collect();
        rows.sort_by(|a, b| a.1.0.cmp(&b.1.0));
        for (uuid, (name, casts)) in rows {
            let nums: Vec<String> = casts.iter().map(u32::to_string).collect();
            text.push_str(&format!("{uuid:032x} {name} {}\n", nums.join(" ")));
        }
        let path = mastery_path();
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(path, text);
    }
}

struct Fireball {
    id: u32,
    owner: u128,
    dim: String,
    pos: [f64; 3],
    vel: [f64; 3],
    age: u32,
    /// The mob it locked on at the cast, if any (by id): it turns toward it.
    target: Option<u32>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Falling,
    Carving(u32),
    Rim,
    Fire,
    Smoke(u32),
}

/// What a crater digs: the meteor's numbers, or a supernova's smaller ones.
#[derive(Clone, Copy)]
struct Dig {
    depth: f64,
    clear: f64,
    rim: f64,
    debris: f64,
    fire: f64,
    lava: f64,
    smoke: f64,
}

struct Meteor {
    id: u32,
    dim: String,
    target: [f64; 3],
    start: [f64; 3],
    age: u32,
    fall: u32,
    radius: f64,
    phase: Phase,
    debris_left: u32,
    rng: u64,
    dig: Dig,
}

/// Absolute zero: the ring of ice going out, the mobs held in ice until it bursts.
struct Frost {
    dim: String,
    center: [f64; 3],
    age: u32,
    reached: f64,
    /// Mob id, where it is held, the ice around it.
    held: Vec<(u32, [f64; 3], Vec<[i32; 3]>)>,
    rng: u64,
}

/// A supernova charging over its caster.
struct Nova {
    dim: String,
    owner: u128,
    ground: [f64; 3],
    age: u32,
}

#[derive(Default)]
struct Server {
    tick: u64,
    casters: HashMap<u128, Caster>,
    fireballs: Vec<Fireball>,
    meteors: Vec<Meteor>,
    frosts: Vec<Frost>,
    novas: Vec<Nova>,
    next_id: u32,
    mastery: Mastery,
}

static SERVER: Mutex<Option<Server>> = Mutex::new(None);
/// Spells asked for by players' clients, waiting for their level's tick: (player, spell, when).
static QUEUE: Mutex<Vec<(u128, u8, Instant)>> = Mutex::new(Vec::new());

/// `native_net_from_client`: a player asks for a spell.
pub fn net_from_client(player: u128, channel: &str, data: &[u8]) {
    if channel != CH_CAST {
        return;
    }
    let Some(&spell) = data.first() else { return };
    if !(SPELL_FIRE..=SPELL_NOVA).contains(&spell) {
        return;
    }
    let mut q = lock(&QUEUE);
    if q.len() < 256 {
        q.push((player, spell, Instant::now()));
    }
}

fn send(game: &mut Game<'_>, uuid: u128, channel: &str, data: &[u8]) {
    let _ = game.native_net_send_to_player(uuid, channel, data);
}

fn broadcast(game: &mut Game<'_>, players: &[NativeServerPlayer], dim: &str, data: &[u8]) {
    for p in players.iter().filter(|p| p.dimension == dim) {
        send(game, p.uuid, CH_FX, data);
    }
}

/// Particles every player of the level sees, as far as 512 blocks.
fn particles_far(
    game: &mut Game<'_>,
    players: &[NativeServerPlayer],
    dim: &str,
    options: &ParticleOptions,
    at: [f64; 3],
    spread: [f64; 3],
    speed: f64,
    count: i32,
) {
    for p in players.iter().filter(|p| p.dimension == dim) {
        if dist(p.position, at) < 500.0 {
            let _ = game.native_send_particles_to(p.uuid, options, None, at, spread, speed, count, true);
        }
    }
}

fn simple(ty: ParticleType) -> ParticleOptions {
    ParticleOptions::simple(ty)
}

fn dust(rgb: [f32; 3], scale: f32) -> ParticleOptions {
    ParticleOptions {
        color: [rgb[0], rgb[1], rgb[2], 1.0],
        scale,
        ..ParticleOptions::simple(ParticleType::Dust)
    }
}

fn sound(game: &mut Game<'_>, name: &str, at: [f64; 3], volume: f32, pitch: f32) {
    let _ = game.native_play_sound_in_place(name, at, 7, volume, pitch);
}

fn solid(game: &Game<'_>, x: i32, y: i32, z: i32) -> bool {
    let state = game.world.block(x, y, z);
    let facts = game.res.facts();
    if facts.is_air(state) {
        return false;
    }
    let f = facts.get(state);
    f.full_collision || f.legacy_solid
}

fn solid_at(game: &Game<'_>, p: [f64; 3]) -> bool {
    solid(game, p[0].floor() as i32, p[1].floor() as i32, p[2].floor() as i32)
}

/// Water or lava (a fireball goes out in it, the meteor aims at its surface).
fn liquid_at(game: &Game<'_>, p: [f64; 3]) -> bool {
    let state = game.world.block(p[0].floor() as i32, p[1].floor() as i32, p[2].floor() as i32);
    let facts = game.res.facts();
    if facts.is_air(state) {
        return false;
    }
    let name = &facts.get(state).name;
    name.ends_with(":water") || name.ends_with(":lava")
}

/// Where a ray from `from` along `dir` (unit) meets a solid block within `max` blocks: the last point
/// before it, and how far that is.
fn ray(game: &Game<'_>, from: [f64; 3], dir: [f64; 3], max: f64) -> Option<([f64; 3], f64)> {
    ray_by(game, from, dir, max, false)
}

/// `ray`, stopping at water and lava too when `wet`.
fn ray_by(game: &Game<'_>, from: [f64; 3], dir: [f64; 3], max: f64, wet: bool) -> Option<([f64; 3], f64)> {
    let step = 0.2;
    let mut d = 0.0;
    while d <= max {
        let p = add(from, mul(dir, d));
        if solid_at(game, p) || (wet && liquid_at(game, p)) {
            let back = (d - step).max(0.0);
            return Some((add(from, mul(dir, back)), back));
        }
        d += step;
    }
    None
}

/// The living mob nearest the aim within `cone` degrees and `range` blocks, in sight from `eye`: its id
/// and the middle of its body.
fn aim_target(game: &Game<'_>, eye: [f64; 3], dir: [f64; 3], range: f64, cone: f64, dim: &str) -> Option<(u32, [f64; 3])> {
    if cone <= 0.0 {
        return None;
    }
    let cos_max = cone.to_radians().cos();
    let mut best: Option<(u32, [f64; 3], f64)> = None;
    for m in game.native_mobs(dim) {
        if m.health <= 0.0 {
            continue;
        }
        let c = add(m.pos, [0.0, 0.9, 0.0]);
        let to = sub(c, eye);
        let d = len(to);
        if !(1.5..=range).contains(&d) {
            continue;
        }
        let cos = (to[0] * dir[0] + to[1] * dir[1] + to[2] * dir[2]) / d;
        if cos < cos_max {
            continue;
        }
        if ray(game, eye, mul(to, 1.0 / d), d - 0.8).is_some() {
            continue;
        }
        let score = (1.0 - cos) * 40.0 + d * 0.01;
        if best.is_none_or(|b| score < b.2) {
            best = Some((m.id, c, score));
        }
    }
    best.map(|b| (b.0, b.1))
}

fn look(p: &NativeServerPlayer) -> ([f64; 3], [f64; 3]) {
    let yaw = f64::from(p.yaw).to_radians();
    let pitch = f64::from(p.pitch).to_radians();
    let dir = [-yaw.sin() * pitch.cos(), -pitch.sin(), yaw.cos() * pitch.cos()];
    let eye = [p.position[0], p.position[1] + f64::from(p.eye_height), p.position[2]];
    (eye, dir)
}

/// `native_level_tick_post`: the level `dim` is in place; everything that touches it happens here.
pub(crate) fn level_tick(game: &mut Game<'_>, dim: &str) {
    if game.native_window() {
        return;
    }
    let t = tune();
    let mut s = lock(&SERVER).take().unwrap_or_default();
    let players = game.native_server_players();
    let overworld = dim == "minecraft:overworld";
    if overworld {
        s.tick += 1;
    }
    // The spells asked for by the players of this level.
    let mine: Vec<(u128, u8)> = {
        let mut q = lock(&QUEUE);
        let mut mine = Vec::new();
        q.retain(|&(uuid, spell, at)| {
            if at.elapsed() > Duration::from_secs(3) {
                return false;
            }
            match players.iter().find(|p| p.uuid == uuid) {
                None => false,
                Some(p) if p.dimension == dim => {
                    mine.push((uuid, spell));
                    false
                }
                Some(_) => true,
            }
        });
        mine
    };
    for (uuid, spell) in mine {
        if let Some(p) = players.iter().find(|p| p.uuid == uuid) {
            cast(game, &mut s, &t, &players, p, spell, dim);
        }
    }
    // The spells gathered long enough go off, aimed where their casters look now.
    let due: Vec<(u128, u8)> = s
        .casters
        .iter()
        .filter_map(|(uuid, c)| c.charge.filter(|&(_, at)| s.tick >= at).map(|(spell, _)| (*uuid, spell)))
        .collect();
    for (uuid, spell) in due {
        match players.iter().find(|p| p.uuid == uuid) {
            Some(p) if p.dimension == dim => release(game, &mut s, &t, &players, p, spell, dim),
            Some(_) => {}
            None => {
                if let Some(c) = s.casters.get_mut(&uuid) {
                    c.charge = None;
                }
            }
        }
    }
    if overworld && s.tick % 200 == 0 {
        s.mastery.save();
    }
    step_fireballs(game, &mut s, &t, &players, dim);
    step_meteors(game, &mut s, &t, &players, dim);
    step_frosts(game, &mut s, &t, &players, dim);
    step_novas(game, &mut s, &t, &players, dim);
    if overworld {
        mana_tick(game, &mut s, &t, &players);
    }
    *lock(&SERVER) = Some(s);
}

fn mana_tick(game: &mut Game<'_>, s: &mut Server, t: &Tune, players: &[NativeServerPlayer]) {
    let tick = s.tick;
    for p in players {
        // What they wear and hold, looked at four times a second.
        let fresh = (tick % 5 == 0 || !s.casters.contains_key(&p.uuid)).then(|| gear_of(game, p));
        let levels = s.mastery.of(p.uuid).map(|n| mastery_level(n, t));
        let c = s.casters.entry(p.uuid).or_insert_with(|| Caster::new(t));
        if let Some(gear) = fresh {
            if gear != c.gear {
                debug(&format!("gear of {}: {gear:?}", p.name));
            }
            c.gear = gear;
        }
        let (more, haste) = c.gear.power(t);
        let max = t.mana_max + more;
        c.mana = (c.mana + t.mana_regen / 20.0).min(max);
        if tick % 4 != 0 {
            continue;
        }
        let cds: [f32; SPELLS] = std::array::from_fn(|i| c.ready[i + 1].saturating_sub(tick) as f32 / 20.0);
        let key = (
            (c.mana * 2.0) as i32 + ((max * 1000.0) as i32) * 7 + ((haste * 1000.0) as i32) * 13,
            cds.map(|v| (v * 10.0) as i32),
            c.gear.bits() as i32 + levels.iter().map(|l| (l.0 * 100) as i32 + (l.1 * 99.0) as i32).sum::<i32>() * 16,
        );
        if c.sent == Some(key) && tick % 100 != 0 {
            continue;
        }
        c.sent = Some(key);
        let mut data = Writer::new(0).f32(c.mana).f32(max);
        for cd in cds {
            data = data.f32(cd);
        }
        for cost in costs(t) {
            data = data.f32(cost);
        }
        // Since 0.5: each spell's whole cooldown as the things make it, then what is worn and whether
        // spells need the staff.
        for (i, total) in cooldowns(t).into_iter().enumerate() {
            data = data.f32(total * haste * mastery_speed(levels[i].0, t));
        }
        data = data.u32(u32::from(c.gear.bits())).u32(u32::from(t.need_staff));
        // Since 0.6: each spell's mastery level and the way to the next (0..1).
        for (level, progress) in levels {
            data = data.u32(level).f32(progress);
        }
        let _ = game.native_net_send_to_player(p.uuid, CH_MANA, &data.0[1..]);
    }
    if s.casters.len() > players.len() + 64 {
        s.casters.retain(|uuid, _| players.iter().any(|p| p.uuid == *uuid));
    }
}

/// Each spell's mana and cooldown (seconds), by spell id - 1.
fn costs(t: &Tune) -> [f32; SPELLS] {
    [t.fire_cost, t.step_cost, t.meteor_cost, t.frost_cost, t.chain_cost, t.nova_cost]
}

fn cooldowns(t: &Tune) -> [f32; SPELLS] {
    [
        t.fire_cooldown,
        t.step_cooldown,
        t.meteor_cooldown,
        t.frost_cooldown,
        t.chain_cooldown,
        t.nova_cooldown,
    ]
}

fn fail(game: &mut Game<'_>, uuid: u128, spell: u8, why: u8) {
    let data = Writer::new(FX_FAIL).u32(u32::from(spell)).u32(u32::from(why));
    send(game, uuid, CH_FX, &data.0);
}

fn cast(
    game: &mut Game<'_>,
    s: &mut Server,
    t: &Tune,
    players: &[NativeServerPlayer],
    p: &NativeServerPlayer,
    spell: u8,
    dim: &str,
) {
    if p.health <= 0.0 || p.game_mode == 3 {
        return;
    }
    let i = usize::from(spell.clamp(1, SPELLS as u8)) - 1;
    let tick = s.tick;
    // What they wear and hold right now: a staff put away a moment ago casts nothing.
    let gear = gear_of(game, p);
    let c = s.casters.entry(p.uuid).or_insert_with(|| Caster::new(t));
    c.gear = gear;
    if t.need_staff && !gear.magic {
        debug(&format!("cast {spell} by {}: no staff in hand", p.name));
        fail(game, p.uuid, spell, 4);
        return;
    }
    if c.charge.is_some() {
        // One spell at a time: the one gathering goes off first.
        return;
    }
    let (_, haste) = gear.power(t);
    let cost = costs(t)[i];
    if tick < c.ready[usize::from(spell)] {
        fail(game, p.uuid, spell, 1);
        return;
    }
    if c.mana < cost {
        fail(game, p.uuid, spell, 2);
        return;
    }
    let (level, _) = mastery_level(s.mastery.of(p.uuid)[i], t);
    let windup = t.windup[i] * t.windup_scale * haste * mastery_speed(level, t);
    let ticks = (windup * 20.0).round() as u64;
    // Everyone near sees the caster gather it (the pose, the circle growing, the crystal drawing light in),
    // then let it go; and hears the staff.
    let after = match spell {
        SPELL_NOVA => t.nova_charge + 0.35,
        SPELL_METEOR => 1.0,
        SPELL_FROST => 0.8,
        SPELL_STEP => 0.45,
        SPELL_CHAIN => 0.65,
        _ => 0.55,
    };
    let windup = ticks as f32 / 20.0;
    let data = Writer::new(FX_CAST)
        .u32(p.entity_id as u32)
        .u32(u32::from(spell))
        .v3(p.position)
        .f32(windup + after)
        .f32(windup)
        .u32(level);
    broadcast(game, players, dim, &data.0);
    let (eye, _) = look(p);
    sound(game, "kotost_mage_items:cast", eye, 1.0, if ticks > 0 { 0.85 } else { 1.0 });
    debug(&format!("cast {spell} by {}: gathers {windup:.2}s (level {level})", p.name));
    if ticks == 0 {
        release(game, s, t, players, p, spell, dim);
    } else if let Some(c) = s.casters.get_mut(&p.uuid) {
        c.charge = Some((spell, tick + ticks));
    }
}

/// A gathered spell goes off: aimed as the caster looks now, paid for, its cooldown and mastery counted.
fn release(
    game: &mut Game<'_>,
    s: &mut Server,
    t: &Tune,
    players: &[NativeServerPlayer],
    p: &NativeServerPlayer,
    spell: u8,
    dim: &str,
) {
    let i = usize::from(spell.clamp(1, SPELLS as u8)) - 1;
    let tick = s.tick;
    let gear = gear_of(game, p);
    let casts = s.mastery.of(p.uuid)[i];
    let Some(c) = s.casters.get_mut(&p.uuid) else {
        return;
    };
    c.charge = None;
    c.gear = gear;
    let fizzle = |game: &mut Game<'_>, why: u8| {
        fail(game, p.uuid, spell, why);
        let data = Writer::new(FX_RELEASE).u32(p.entity_id as u32).u32(u32::from(spell)).u32(0);
        broadcast(game, players, dim, &data.0);
    };
    if p.health <= 0.0 || p.game_mode == 3 {
        fizzle(game, 3);
        return;
    }
    // The staff put away while it gathered: nothing.
    if t.need_staff && !gear.magic {
        fizzle(game, 4);
        return;
    }
    let (_, haste) = gear.power(t);
    let (level, _) = mastery_level(casts, t);
    let (cost, cooldown) = (costs(t)[i], cooldowns(t)[i] * haste * mastery_speed(level, t));
    if c.mana < cost {
        fizzle(game, 2);
        return;
    }
    let done = match spell {
        SPELL_FIRE => fireball(game, s, t, players, p, dim),
        SPELL_STEP => shadow_step(game, t, players, p, dim),
        SPELL_METEOR => meteor(game, s, t, players, p, dim),
        SPELL_FROST => frost(game, s, t, players, p, dim),
        SPELL_CHAIN => chain(game, t, players, p, dim),
        _ => nova(game, s, t, players, p, dim),
    };
    debug(&format!("release {spell} by {} in {dim}: {done}", p.name));
    if !done {
        fizzle(game, 3);
        return;
    }
    if let Some(c) = s.casters.get_mut(&p.uuid) {
        c.mana -= cost;
        c.ready[usize::from(spell)] = tick + (cooldown * 20.0) as u64;
        c.sent = None;
    }
    s.mastery.add(p.uuid, &p.name, spell);
    let data = Writer::new(FX_RELEASE).u32(p.entity_id as u32).u32(u32::from(spell)).u32(1);
    broadcast(game, players, dim, &data.0);
}

// --- fireball --------------------------------------------------------------------------------------

fn fireball(
    game: &mut Game<'_>,
    s: &mut Server,
    t: &Tune,
    players: &[NativeServerPlayer],
    p: &NativeServerPlayer,
    dim: &str,
) -> bool {
    let (eye, look_dir) = look(p);
    // Aim help: the mob nearest the crosshair within `fire_assist` degrees, in sight, takes the shot.
    let target = aim_target(game, eye, look_dir, 48.0, f64::from(t.fire_assist), dim);
    let dir = match target {
        Some((_, at)) => {
            let to = sub(at, eye);
            mul(to, 1.0 / len(to).max(1e-3))
        }
        None => look_dir,
    };
    let pos = add(eye, mul(dir, 0.9));
    let vel = mul(dir, f64::from(t.fire_speed));
    s.next_id = s.next_id.wrapping_add(1);
    let id = s.next_id;
    s.fireballs.push(Fireball {
        id,
        owner: p.uuid,
        dim: dim.to_owned(),
        pos,
        vel,
        age: 0,
        target: target.map(|t| t.0),
    });
    let data = Writer::new(FX_FIRE_SPAWN)
        .u32(id)
        .v3(pos)
        .v3(vel)
        .f32(t.fire_gravity)
        .f32(t.fire_life);
    broadcast(game, players, dim, &data.0);
    sound(game, "kotost_mage_items:fire.launch", eye, 1.2, 1.0);
    true
}

fn step_fireballs(
    game: &mut Game<'_>,
    s: &mut Server,
    t: &Tune,
    players: &[NativeServerPlayer],
    dim: &str,
) {
    if !s.fireballs.iter().any(|f| f.dim == dim) {
        return;
    }
    let all = game.native_mobs(dim);
    let mobs: Vec<[f64; 3]> = all.iter().map(|m| m.pos).collect();
    let life = (t.fire_life * 20.0) as u32;
    let mut done: Vec<(u32, [f64; 3], bool)> = Vec::new();
    let mut steered: Vec<(u32, [f64; 3], [f64; 3])> = Vec::new();
    let max_turn = f64::from(t.fire_homing).to_radians();
    for f in s.fireballs.iter_mut().filter(|f| f.dim == dim) {
        f.age += 1;
        // Turn toward the mob it locked on, a little each tick, while that one lives.
        if let Some(id) = f.target
            && max_turn > 0.0
        {
            match all.iter().find(|m| m.id == id && m.health > 0.0) {
                Some(m) => {
                    let speed = len(f.vel);
                    let to = sub(add(m.pos, [0.0, 0.9, 0.0]), f.pos);
                    let (d, want) = (len(f.vel).max(1e-6), len(to).max(1e-6));
                    let (a, b) = (mul(f.vel, 1.0 / d), mul(to, 1.0 / want));
                    let cos = (a[0] * b[0] + a[1] * b[1] + a[2] * b[2]).clamp(-1.0, 1.0);
                    let angle = cos.acos();
                    if angle > 1e-3 && angle < 1.6 {
                        let k = (max_turn / angle).min(1.0);
                        let n = add(a, mul(sub(b, a), k));
                        f.vel = mul(n, speed / len(n).max(1e-6));
                        steered.push((f.id, f.pos, f.vel));
                    }
                }
                None => f.target = None,
            }
        }
        let mut hit = false;
        for _ in 0..4 {
            f.pos = add(f.pos, mul(f.vel, 0.25));
            f.vel[1] -= f64::from(t.fire_gravity) * 0.25;
            if solid_at(game, f.pos) || liquid_at(game, f.pos) {
                hit = true;
            }
            let near = |at: [f64; 3]| dist(add(at, [0.0, 0.9, 0.0]), f.pos) < 1.3;
            if mobs.iter().any(|m| near(*m))
                || players
                    .iter()
                    .any(|p| p.uuid != f.owner && p.dimension == dim && p.game_mode != 3 && near(p.position))
            {
                hit = true;
            }
            if hit {
                break;
            }
        }
        if hit || f.age >= life {
            done.push((f.id, f.pos, hit));
        }
    }
    for &(id, _, _) in &steered {
        if done.iter().any(|d| d.0 == id) {
            continue;
        }
        // Where it is after this tick's move, and its velocity: the clients fly it on from there.
        if let Some(f) = s.fireballs.iter().find(|f| f.id == id) {
            let data = Writer::new(FX_FIRE_STEER).u32(id).v3(f.pos).v3(f.vel);
            broadcast(game, players, dim, &data.0);
        }
    }
    for f in s.fireballs.iter().filter(|f| f.dim == dim) {
        if done.iter().any(|d| d.0 == f.id) {
            continue;
        }
        let back = sub(f.pos, mul(f.vel, 0.5));
        let _ = game.native_send_particles_in_place(
            &simple(ParticleType::Flame),
            None,
            f.pos,
            [0.12, 0.12, 0.12],
            0.01,
            4,
        );
        let _ = game.native_send_particles_in_place(
            &simple(ParticleType::SmallFlame),
            None,
            back,
            [0.2, 0.2, 0.2],
            0.01,
            3,
        );
        if f.age % 2 == 0 {
            let _ = game.native_send_particles_in_place(
                &simple(ParticleType::Smoke),
                None,
                back,
                [0.15, 0.15, 0.15],
                0.01,
                2,
            );
        }
    }
    for &(id, pos, hit) in &done {
        let data = Writer::new(FX_FIRE_END).u32(id).v3(pos).u32(u32::from(hit));
        broadcast(game, players, dim, &data.0);
        if hit {
            sound(game, "kotost_mage_items:fire.hit", pos, 2.5, 1.0);
            let ok = game.native_explode(
                dim,
                pos,
                t.fire_radius,
                t.fire_ignite,
                super::native_explosions::NativeBlast::Keep,
            );
            debug(&format!("fireball {id} hit at {pos:?}: explode {ok}"));
            let _ = game.native_send_particles_in_place(
                &simple(ParticleType::Flame),
                None,
                pos,
                [0.5, 0.5, 0.5],
                0.12,
                45,
            );
            let _ = game.native_send_particles_in_place(
                &simple(ParticleType::Lava),
                None,
                pos,
                [0.3, 0.3, 0.3],
                0.0,
                8,
            );
            let _ = game.native_send_particles_in_place(
                &simple(ParticleType::LargeSmoke),
                None,
                pos,
                [0.6, 0.6, 0.6],
                0.03,
                12,
            );
        } else {
            let _ = game.native_send_particles_in_place(
                &simple(ParticleType::Smoke),
                None,
                pos,
                [0.2, 0.2, 0.2],
                0.02,
                10,
            );
            sound(game, "kotost_mage_items:fire.fizzle", pos, 0.7, 1.0);
        }
    }
    s.fireballs.retain(|f| !done.iter().any(|d| d.0 == f.id));
}

// --- shadow step -----------------------------------------------------------------------------------

fn shadow_step(
    game: &mut Game<'_>,
    t: &Tune,
    players: &[NativeServerPlayer],
    p: &NativeServerPlayer,
    dim: &str,
) -> bool {
    let (eye, dir) = look(p);
    let eye_h = f64::from(p.eye_height);
    let max = f64::from(t.step_max.max(t.step_min));
    let min = f64::from(t.step_min);
    let far = match ray(game, eye, dir, max) {
        Some((_, d)) => (d - 0.3).max(0.0),
        None => max,
    };
    let fits = |game: &Game<'_>, x: i32, y: i32, z: i32| !solid(game, x, y, z) && !solid(game, x, y + 1, z);
    let mut target: Option<[f64; 3]> = None;
    let mut loose: Option<[f64; 3]> = None;
    let mut d = far;
    while d >= min && target.is_none() {
        let e = add(eye, mul(dir, d));
        let (x, z) = (e[0].floor() as i32, e[2].floor() as i32);
        let base = (e[1] - eye_h).floor() as i32;
        for dy in [0, 1, -1, 2, -2, -3] {
            let y = base + dy;
            if fits(&*game, x, y, z) {
                let at = [e[0], f64::from(y), e[2]];
                if solid(&*game, x, y - 1, z) {
                    target = Some(at);
                    break;
                }
                if loose.is_none() && dy == 0 {
                    loose = Some([e[0], e[1] - eye_h, e[2]]);
                }
            }
        }
        d -= 0.5;
    }
    let Some(to) = target.or(loose) else {
        return false;
    };
    let from = p.position;
    if dist(from, to) < min - 0.6 {
        return false;
    }
    if !game.native_teleport_player(p.uuid, dim, to, p.yaw, p.pitch) {
        return false;
    }
    for at in [from, to] {
        let mid = add(at, [0.0, 1.0, 0.0]);
        let _ = game.native_send_particles_in_place(
            &simple(ParticleType::LargeSmoke),
            None,
            mid,
            [0.35, 0.8, 0.35],
            0.02,
            30,
        );
        let _ = game.native_send_particles_in_place(
            &simple(ParticleType::SquidInk),
            None,
            mid,
            [0.3, 0.6, 0.3],
            0.05,
            14,
        );
        let _ = game.native_send_particles_in_place(
            &simple(ParticleType::ReversePortal),
            None,
            mid,
            [0.4, 0.8, 0.4],
            0.3,
            40,
        );
        let _ = game.native_send_particles_in_place(
            &dust([0.35, 0.1, 0.6], 1.6),
            None,
            mid,
            [0.4, 0.9, 0.4],
            0.0,
            20,
        );
        sound(game, "kotost_mage_items:step", at, 1.0, 1.0);
    }
    let data = Writer::new(FX_STEP).v3(from).v3(to);
    broadcast(game, players, dim, &data.0);
    true
}

// --- meteor ----------------------------------------------------------------------------------------

fn meteor(
    game: &mut Game<'_>,
    s: &mut Server,
    t: &Tune,
    players: &[NativeServerPlayer],
    p: &NativeServerPlayer,
    dim: &str,
) -> bool {
    let (eye, dir) = look(p);
    let range = f64::from(t.meteor_range);
    let hit = match ray_by(game, eye, dir, range, true) {
        Some((at, _)) => at,
        None => {
            // Aimed at the sky or the far distance: the ground under the far end.
            let far = add(eye, mul(dir, range));
            let (x, z) = (far[0].floor() as i32, far[2].floor() as i32);
            let mut y = far[1].floor() as i32;
            let bottom = (y - 160).max(dimension_floor(dim));
            while y > bottom && !solid(game, x, y, z) {
                y -= 1;
            }
            if y <= bottom {
                return false;
            }
            [far[0], f64::from(y + 1), far[2]]
        }
    };
    // Settle on the top of the column hit.
    let (x, z) = (hit[0].floor() as i32, hit[2].floor() as i32);
    let mut y = hit[1].floor() as i32 + 2;
    let bottom = y - 10;
    while y > bottom && !solid(game, x, y - 1, z) && !liquid_at(game, [f64::from(x), f64::from(y - 1), f64::from(z)]) {
        y -= 1;
    }
    let target = [f64::from(x) + 0.5, f64::from(y), f64::from(z) + 0.5];
    let mut h = [dir[0], 0.0, dir[2]];
    let hl = len(h);
    h = if hl < 1e-3 { [0.0, 0.0, 1.0] } else { mul(h, 1.0 / hl) };
    // From beyond the target and to one side, high up: the caster watches it come down.
    let side = [-h[2], 0.0, h[0]];
    let start = add(target, add(add(mul(h, 70.0), mul(side, 45.0)), [0.0, 150.0, 0.0]));
    s.next_id = s.next_id.wrapping_add(1);
    let id = s.next_id;
    let fall = (t.meteor_fall * 20.0).round().max(4.0) as u32;
    let radius = f64::from(t.meteor_radius);
    s.meteors.push(Meteor {
        id,
        dim: dim.to_owned(),
        target,
        start,
        age: 0,
        fall,
        radius,
        phase: Phase::Falling,
        debris_left: t.meteor_debris as u32,
        rng: 0x2545_F491_4F6C_DD1D ^ u64::from(id) ^ ((x as i64 as u64) << 21) ^ (z as i64 as u64),
        dig: Dig {
            depth: f64::from(t.meteor_depth),
            clear: f64::from(t.meteor_clear),
            rim: f64::from(t.meteor_rim),
            debris: f64::from(t.meteor_debris),
            fire: f64::from(t.meteor_fire),
            lava: f64::from(t.meteor_lava),
            smoke: f64::from(t.meteor_smoke),
        },
    });
    let data = Writer::new(FX_METEOR_MARK)
        .u32(id)
        .v3(target)
        .v3(start)
        .f32(radius as f32)
        .u32(fall)
        .f32(t.meteor_size);
    broadcast(game, players, dim, &data.0);
    sound(game, "kotost_mage_items:meteor.call", target, 6.0, 1.0);
    let _ = eye;
    true
}

fn step_meteors(
    game: &mut Game<'_>,
    s: &mut Server,
    t: &Tune,
    players: &[NativeServerPlayer],
    dim: &str,
) {
    if !s.meteors.iter().any(|m| m.dim == dim) {
        return;
    }
    let mut list = std::mem::take(&mut s.meteors);
    for m in list.iter_mut().filter(|m| m.dim == dim) {
        m.age += 1;
        match m.phase {
            Phase::Falling => {
                if m.age % 3 == 1 {
                    let n = ((m.radius * 1.6) as usize).clamp(16, 140);
                    for i in 0..n {
                        let a = i as f64 / n as f64 * std::f64::consts::TAU;
                        let at = add(m.target, [m.radius * a.cos(), 0.3, m.radius * a.sin()]);
                        particles_far(game, players, dim, &dust([1.0, 0.08, 0.05], 3.0), at, [0.05; 3], 0.0, 1);
                    }
                    let column = add(m.target, [0.0, 1.0, 0.0]);
                    particles_far(game, players, dim, &dust([1.0, 0.15, 0.1], 2.0), column, [0.25, 1.0, 0.25], 0.0, 5);
                }
                let u = (f64::from(m.age) / f64::from(m.fall)).min(1.0);
                let pos = add(m.start, mul(sub(m.target, m.start), u * u));
                let size = f64::from(t.meteor_size);
                particles_far(game, players, dim, &simple(ParticleType::Flame), pos, [size * 0.4; 3], 0.05, 30);
                particles_far(game, players, dim, &simple(ParticleType::Lava), pos, [size * 0.3; 3], 0.0, 6);
                particles_far(
                    game,
                    players,
                    dim,
                    &simple(ParticleType::LargeSmoke),
                    pos,
                    [size * 0.5; 3],
                    0.05,
                    18,
                );
                particles_far(
                    game,
                    players,
                    dim,
                    &simple(ParticleType::CampfireSignalSmoke),
                    pos,
                    [size * 0.3; 3],
                    0.01,
                    3,
                );
                if m.age == m.fall / 2 {
                    sound(game, "kotost_mage_items:meteor.fall", m.target, 12.0, 1.0);
                }
                if m.age >= m.fall {
                    impact(game, m, t, players, dim);
                    m.phase = if t.meteor_blocks { Phase::Carving(0) } else { Phase::Smoke(0) };
                }
            }
            Phase::Carving(ring) => {
                let rings = (m.radius / 3.0).clamp(6.0, 16.0) as u32;
                carve(game, m, t, ring, rings, dim);
                m.phase = if ring + 1 >= rings { Phase::Rim } else { Phase::Carving(ring + 1) };
            }
            Phase::Rim => {
                rim(game, m, t, dim, false);
                m.phase = Phase::Fire;
            }
            Phase::Fire => {
                rim(game, m, t, dim, true);
                m.phase = Phase::Smoke(0);
            }
            Phase::Smoke(n) => {
                smoke(game, m, t, players, dim, n);
                m.phase = Phase::Smoke(n + 1);
            }
        }
    }
    list.retain(|m| match m.phase {
        Phase::Smoke(n) => n < (m.dig.smoke * 20.0) as u32,
        _ => true,
    });
    s.meteors = list;
}

fn impact(game: &mut Game<'_>, m: &mut Meteor, t: &Tune, players: &[NativeServerPlayer], dim: &str) {
    let at = m.target;
    let r = m.radius;
    let data = Writer::new(FX_METEOR_IMPACT).u32(m.id).v3(at).f32(r as f32).f32(t.meteor_push);
    broadcast(game, players, dim, &data.0);
    sound(game, "kotost_mage_items:meteor.impact", at, 24.0, 1.0);
    sound(game, "kotost_mage_items:meteor.rumble", at, 30.0, 1.0);
    let blast = (r * 0.6).clamp(4.0, 32.0) as f32;
    let ok = game.native_explode(dim, add(at, [0.0, 1.0, 0.0]), blast, false, super::native_explosions::NativeBlast::Keep);
    debug(&format!("meteor {} impact at {at:?}, r {r}: explode {ok}", m.id));
    // The shockwave: every mob within twice the crater's radius thrown out and hurt, less towards the edge.
    let reach = r * 2.0;
    for mob in game.native_mobs(dim) {
        let off = sub(mob.pos, at);
        let d = len([off[0], 0.0, off[2]]).max(0.5);
        if d > reach || off[1].abs() > reach {
            continue;
        }
        let f = 1.0 - d / reach;
        let out = [off[0] / d, 0.0, off[2] / d];
        let push = f64::from(t.meteor_push) * f;
        let _ = game.native_push_mob(dim, mob.id, [out[0] * push, 0.5 + push * 0.6, out[2] * push]);
        let damage = t.meteor_damage * (f as f32).powf(1.5);
        if damage >= mob.health {
            let _ = game.native_kill_mob(dim, mob.id);
        } else if damage > 0.5 {
            let _ = game.native_set_mob_health(dim, mob.id, mob.health - damage);
        }
    }
    let rs = r * 0.5;
    particles_far(game, players, dim, &simple(ParticleType::ExplosionEmitter), at, [rs * 0.5, 2.0, rs * 0.5], 0.0, 14);
    particles_far(game, players, dim, &simple(ParticleType::Flash), add(at, [0.0, 3.0, 0.0]), [1.0; 3], 0.0, 4);
    particles_far(game, players, dim, &simple(ParticleType::Explosion), at, [rs, 3.0, rs], 0.0, 80);
    particles_far(game, players, dim, &simple(ParticleType::Lava), at, [rs * 0.6, 2.0, rs * 0.6], 0.0, 160);
    particles_far(game, players, dim, &simple(ParticleType::Flame), at, [rs, 4.0, rs], 0.6, 260);
    particles_far(game, players, dim, &simple(ParticleType::LargeSmoke), add(at, [0.0, 4.0, 0.0]), [rs, 5.0, rs], 0.25, 260);
    particles_far(game, players, dim, &simple(ParticleType::CampfireSignalSmoke), at, [rs * 0.4, 2.0, rs * 0.4], 0.05, 60);
}

/// One band of the crater, `ring` of `rings` from the middle out: debris thrown, the bowl dug, its floor
/// lined with burnt rock (and lava in the middle).
fn carve(game: &mut Game<'_>, m: &mut Meteor, t: &Tune, ring: u32, rings: u32, dim: &str) {
    use super::native_world_edit::{NativeBlockEdit, NativeBlockRun};
    let r = m.radius;
    let (r0, r1) = (r * f64::from(ring) / f64::from(rings), r * f64::from(ring + 1) / f64::from(rings));
    let (cx, gy, cz) = (m.target[0].floor() as i32, m.target[1].floor() as i32, m.target[2].floor() as i32);
    let floor_min = dimension_floor(dim) + 3;
    let depth = m.dig.depth;
    let top = gy + m.dig.clear as i32;
    let palette: Vec<String> = [
        "minecraft:air",
        "minecraft:magma_block",
        "minecraft:obsidian",
        "minecraft:blackstone",
        "minecraft:basalt",
        "minecraft:tuff",
        "minecraft:netherrack",
        "minecraft:coarse_dirt",
        "minecraft:gravel",
        "minecraft:lava",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    let mut edit = NativeBlockEdit {
        palette,
        filters: Vec::new(),
        runs: Vec::new(),
    };
    // The debris this band throws: its share of what is left, from the topmost blocks of its columns.
    let area_band = (r1 * r1 - r0 * r0).max(0.0);
    let share = if ring + 1 >= rings {
        m.debris_left
    } else {
        ((m.dig.debris * area_band / (r * r)).round() as u32).min(m.debris_left)
    };
    let columns = (std::f64::consts::PI * area_band).max(1.0);
    let chance = f64::from(share) / columns;
    let mut debris: Vec<(horizon_world::registry::BlockStateId, [f64; 3], [f64; 3])> = Vec::new();
    let reach = r1.ceil() as i32 + 1;
    for dx in -reach..=reach {
        for dz in -reach..=reach {
            let (fx, fz) = (f64::from(dx), f64::from(dz));
            let d = (fx * fx + fz * fz).sqrt();
            if d < r0 || d >= r1 {
                continue;
            }
            let (x, z) = (cx + dx, cz + dz);
            let n = noise(x, z, m.rng);
            let u = d / r;
            let dig = (depth * (1.0 - u * u) + n * 1.2).max(1.0).round() as i32;
            let floor = (gy - 1 - dig).max(floor_min);
            if top > floor {
                edit.runs.push(NativeBlockRun {
                    x,
                    z,
                    y: floor + 1,
                    height: (top - floor) as u32,
                    surface: None,
                    block: 0,
                    only: None,
                });
            }
            let pick = rand01(&mut m.rng);
            let lining = if u < 0.3 {
                if pick < 0.45 { 1 } else if pick < 0.65 { 2 } else { 3 }
            } else if u < 0.7 {
                if pick < 0.35 { 3 } else if pick < 0.6 { 4 } else if pick < 0.75 { 5 } else if pick < 0.9 { 6 } else { 1 }
            } else if pick < 0.4 {
                7
            } else if pick < 0.65 {
                8
            } else if pick < 0.85 {
                3
            } else {
                5
            };
            edit.runs.push(NativeBlockRun {
                x,
                z,
                y: floor - 1,
                height: 2,
                surface: None,
                block: lining,
                only: None,
            });
            if u < 0.18 && rand01(&mut m.rng) < m.dig.lava {
                edit.runs.push(NativeBlockRun {
                    x,
                    z,
                    y: floor,
                    height: 1,
                    surface: None,
                    block: 9,
                    only: None,
                });
            }
            if debris.len() < share as usize && rand01(&mut m.rng) < chance * 1.15 {
                // The topmost solid block of the column: a roof, a tree, the ground.
                let mut y = top.min(gy + 48);
                while y > floor && !solid(game, x, y, z) {
                    y -= 1;
                }
                if y > floor {
                    let state = game.world.block(x, y, z);
                    let f = game.res.facts().get(state);
                    if f.full_collision && !f.block_entity {
                        let k = f64::from(t.meteor_throw);
                        let out = (0.5 + 1.4 * rand01(&mut m.rng)) * k * (0.6 + 0.6 * (1.0 - u));
                        let up = (0.8 + 1.5 * rand01(&mut m.rng)) * k;
                        let (ox, oz) = if d > 0.1 { (fx / d, fz / d) } else { (0.0, 1.0) };
                        let jitter = (rand01(&mut m.rng) - 0.5) * 0.5;
                        debris.push((
                            state,
                            [f64::from(x) + 0.5, f64::from(y), f64::from(z) + 0.5],
                            [ox * out - oz * jitter, up, oz * out + ox * jitter],
                        ));
                    }
                }
            }
        }
    }
    let changed = game.native_edit_blocks(dim, &edit);
    let mut thrown = 0;
    for (state, pos, vel) in debris {
        if game.native_spawn_falling_block(
            dim,
            state,
            pos,
            vel,
            super::native_falling::NativeFalling {
                drop_item: false,
                hurts: true,
            },
        ) {
            thrown += 1;
        }
    }
    m.debris_left = m.debris_left.saturating_sub(thrown);
    debug(&format!(
        "meteor {} ring {ring}/{rings}: {} runs, edit {changed:?}, debris {thrown}",
        m.id,
        edit.runs.len()
    ));
}

/// The raised rim and the scorched ground around the crater (`fire`: the fires, a tick later, on the
/// ground the rim left).
fn rim(game: &mut Game<'_>, m: &mut Meteor, _t: &Tune, dim: &str, fire: bool) {
    use super::native_world_edit::{NativeBlockEdit, NativeBlockRun, NativeHeightmap};
    let r = m.radius;
    let (cx, cz) = (m.target[0].floor() as i32, m.target[2].floor() as i32);
    let palette: Vec<String> = [
        "minecraft:coarse_dirt",
        "minecraft:gravel",
        "minecraft:blackstone",
        "minecraft:tuff",
        "minecraft:fire",
        "minecraft:netherrack",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    let filters: Vec<String> = ["minecraft:air", "minecraft:grass_block"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    let mut edit = NativeBlockEdit {
        palette,
        filters,
        runs: Vec::new(),
    };
    let outer = r * 1.8;
    let reach = outer.ceil() as i32;
    for dx in -reach..=reach {
        for dz in -reach..=reach {
            let (fx, fz) = (f64::from(dx), f64::from(dz));
            let d = (fx * fx + fz * fz).sqrt();
            if d < r || d >= outer {
                continue;
            }
            let (x, z) = (cx + dx, cz + dz);
            let n = noise(x, z, m.rng ^ 0x55);
            if fire {
                let near = 1.0 - (d - r) / (outer - r);
                if rand01(&mut m.rng) < m.dig.fire * (0.4 + near) {
                    edit.runs.push(NativeBlockRun {
                        x,
                        z,
                        y: 0,
                        height: 1,
                        surface: Some(NativeHeightmap::MotionBlocking),
                        block: 4,
                        only: Some(0),
                    });
                }
                continue;
            }
            let w = r * 0.3;
            if d < r + w {
                let k = 1.0 - (d - r) / w;
                let h = (m.dig.rim * k.powf(1.5) * (0.75 + 0.35 * n)).round() as i32;
                if h > 0 {
                    let pick = rand01(&mut m.rng);
                    let block = if pick < 0.45 { 0 } else if pick < 0.7 { 1 } else if pick < 0.9 { 2 } else { 3 };
                    edit.runs.push(NativeBlockRun {
                        x,
                        z,
                        y: 0,
                        height: h as u32,
                        surface: Some(NativeHeightmap::MotionBlocking),
                        block,
                        only: Some(0),
                    });
                }
            }
            // Scorched: grass turns to burnt dirt and netherrack, thinning out with distance.
            let burn = 1.0 - (d - r) / (outer - r);
            if rand01(&mut m.rng) < 0.25 + 0.75 * burn {
                let block = if n > 0.55 { 5 } else { 0 };
                edit.runs.push(NativeBlockRun {
                    x,
                    z,
                    y: -1,
                    height: 1,
                    surface: Some(NativeHeightmap::MotionBlocking),
                    block,
                    only: Some(1),
                });
            }
        }
    }
    let changed = game.native_edit_blocks(dim, &edit);
    debug(&format!("meteor {} rim (fire {fire}): {} runs, {changed:?}", m.id, edit.runs.len()));
}

/// The smoke column and its cap, lava popping in the middle, ash falling around: `n` ticks after.
fn smoke(game: &mut Game<'_>, m: &mut Meteor, _t: &Tune, players: &[NativeServerPlayer], dim: &str, n: u32) {
    if n % 2 != 0 {
        return;
    }
    let at = m.target;
    let r = m.radius;
    let total = (m.dig.smoke * 20.0).max(1.0);
    let fade = (1.0 - f64::from(n) / f64::from(total)).clamp(0.2, 1.0);
    let k = |c: f64| ((c * fade).round() as i32).max(1);
    particles_far(
        game,
        players,
        dim,
        &simple(ParticleType::CampfireSignalSmoke),
        add(at, [0.0, -2.0, 0.0]),
        [r * 0.15, 1.0, r * 0.15],
        0.02,
        k(8.0),
    );
    let h = 6.0 + rand01(&mut m.rng) * 34.0;
    particles_far(
        game,
        players,
        dim,
        &simple(ParticleType::LargeSmoke),
        add(at, [0.0, h, 0.0]),
        [r * 0.25, 4.0, r * 0.25],
        0.04,
        k(12.0),
    );
    particles_far(
        game,
        players,
        dim,
        &simple(ParticleType::LargeSmoke),
        add(at, [0.0, 44.0 + rand01(&mut m.rng) * 10.0, 0.0]),
        [r * 0.8, 5.0, r * 0.8],
        0.03,
        k(14.0),
    );
    particles_far(game, players, dim, &simple(ParticleType::Lava), add(at, [0.0, -6.0, 0.0]), [r * 0.2, 1.0, r * 0.2], 0.0, k(3.0));
    for p in players.iter().filter(|p| p.dimension == dim && dist(p.position, at) < r * 5.0) {
        let above = add(p.position, [0.0, 8.0, 0.0]);
        let _ = game.native_send_particles_to(p.uuid, &simple(ParticleType::Ash), None, above, [14.0, 6.0, 14.0], 0.0, k(40.0), true);
        let _ = game.native_send_particles_to(p.uuid, &simple(ParticleType::WhiteAsh), None, above, [14.0, 6.0, 14.0], 0.0, k(16.0), true);
    }
}

// --- absolute zero -----------------------------------------------------------------------------------

fn frost(
    game: &mut Game<'_>,
    s: &mut Server,
    t: &Tune,
    players: &[NativeServerPlayer],
    p: &NativeServerPlayer,
    dim: &str,
) -> bool {
    let center = p.position;
    s.next_id = s.next_id.wrapping_add(1);
    s.frosts.push(Frost {
        dim: dim.to_owned(),
        center,
        age: 0,
        reached: 0.0,
        held: Vec::new(),
        rng: 0x51_7CC1_B727_220A ^ u64::from(s.next_id),
    });
    let data = Writer::new(FX_FROST).v3(center).f32(t.frost_radius).f32(t.frost_seconds);
    broadcast(game, players, dim, &data.0);
    let mid = add(center, [0.0, 1.0, 0.0]);
    let _ = game.native_send_particles_in_place(&simple(ParticleType::Snowflake), None, mid, [1.5, 0.8, 1.5], 0.25, 120);
    let _ = game.native_send_particles_in_place(&simple(ParticleType::Cloud), None, mid, [1.0, 0.4, 1.0], 0.15, 40);
    sound(game, "kotost_mage_items:frost.cast", center, 2.0, 1.0);
    true
}

fn step_frosts(game: &mut Game<'_>, s: &mut Server, t: &Tune, _players: &[NativeServerPlayer], dim: &str) {
    use super::native_world_edit::{NativeBlockEdit, NativeBlockRun, NativeHeightmap};
    if !s.frosts.iter().any(|f| f.dim == dim) {
        return;
    }
    let radius = f64::from(t.frost_radius);
    let speed = 1.2;
    let spread = (radius / speed).ceil() as u32;
    let end = spread + (t.frost_seconds * 20.0) as u32;
    let mut list = std::mem::take(&mut s.frosts);
    for f in list.iter_mut().filter(|f| f.dim == dim) {
        f.age += 1;
        let r1 = (f64::from(f.age) * speed).min(radius);
        if r1 > f.reached {
            let r0 = f.reached;
            f.reached = r1;
            let (cx, cz) = (f.center[0].floor() as i32, f.center[2].floor() as i32);
            let mut edit = NativeBlockEdit {
                palette: ["minecraft:ice", "minecraft:snow", "minecraft:obsidian"].map(str::to_owned).to_vec(),
                filters: ["minecraft:water", "minecraft:air", "minecraft:lava", "minecraft:fire"].map(str::to_owned).to_vec(),
                runs: Vec::new(),
            };
            let reach = r1.ceil() as i32;
            for dx in -reach..=reach {
                for dz in -reach..=reach {
                    let d = f64::from(dx * dx + dz * dz).sqrt();
                    if d < r0 || d >= r1 {
                        continue;
                    }
                    let (x, z) = (cx + dx, cz + dz);
                    let surface = Some(NativeHeightmap::MotionBlocking);
                    for (y, block, only) in [(-1, 0u32, 0u32), (0, 1, 1), (-1, 2, 2), (0, 1, 3)] {
                        edit.runs.push(NativeBlockRun {
                            x,
                            z,
                            y,
                            height: 1,
                            surface,
                            block,
                            only: Some(only),
                        });
                    }
                }
            }
            // The mobs the ring reaches: a block of ice around each, held where they stand.
            for mob in game.native_mobs(dim) {
                let off = sub(mob.pos, f.center);
                let d = len([off[0], 0.0, off[2]]);
                if d < r0 || d >= r1 || off[1].abs() > 6.0 || f.held.iter().any(|h| h.0 == mob.id) {
                    continue;
                }
                let (x, y, z) = (mob.pos[0].floor() as i32, mob.pos[1].floor() as i32, mob.pos[2].floor() as i32);
                let mut cells = Vec::new();
                for dx in -1..=1 {
                    for dz in -1..=1 {
                        if dx == 0 && dz == 0 {
                            continue;
                        }
                        for dy in 0..2 {
                            cells.push([x + dx, y + dy, z + dz]);
                        }
                    }
                }
                cells.push([x, y + 2, z]);
                for c in &cells {
                    edit.runs.push(NativeBlockRun {
                        x: c[0],
                        z: c[2],
                        y: c[1],
                        height: 1,
                        surface: None,
                        block: 0,
                        only: Some(1),
                    });
                }
                f.held.push((mob.id, mob.pos, cells));
            }
            let changed = game.native_edit_blocks(dim, &edit);
            debug(&format!("frost ring {r0:.1}..{r1:.1}: {changed:?}, held {}", f.held.len()));
            let n = (r1 * 3.0) as usize;
            for i in 0..n {
                let a = i as f64 / n as f64 * std::f64::consts::TAU + rand01(&mut f.rng) * 0.2;
                let at = add(f.center, [r1 * a.cos(), 0.6, r1 * a.sin()]);
                let _ = game.native_send_particles_in_place(&simple(ParticleType::Snowflake), None, at, [0.3, 0.3, 0.3], 0.02, 2);
            }
        }
        for (id, pos, _) in &f.held {
            let _ = game.native_teleport_mob(dim, *id, *pos);
        }
        if f.age % 6 == 0 {
            for (_, pos, _) in &f.held {
                let _ = game.native_send_particles_in_place(
                    &simple(ParticleType::Snowflake),
                    None,
                    add(*pos, [0.0, 1.2, 0.0]),
                    [0.6, 0.6, 0.6],
                    0.01,
                    4,
                );
            }
        }
        if f.age >= end {
            // The ice bursts: shards, damage, the blocks of ice gone.
            let mut edit = NativeBlockEdit {
                palette: vec!["minecraft:air".to_owned()],
                filters: vec!["minecraft:ice".to_owned()],
                runs: Vec::new(),
            };
            for (id, pos, cells) in &f.held {
                for c in cells {
                    edit.runs.push(NativeBlockRun {
                        x: c[0],
                        z: c[2],
                        y: c[1],
                        height: 1,
                        surface: None,
                        block: 0,
                        only: Some(0),
                    });
                }
                let mid = add(*pos, [0.0, 1.0, 0.0]);
                let _ = game.native_send_particles_in_place(&simple(ParticleType::Snowflake), None, mid, [0.8, 0.8, 0.8], 0.3, 40);
                let _ = game.native_send_particles_in_place(&simple(ParticleType::Cloud), None, mid, [0.6, 0.6, 0.6], 0.1, 12);
                let _ = game.native_send_particles_in_place(&dust([0.75, 0.9, 1.0], 1.2), None, mid, [0.9, 0.9, 0.9], 0.0, 30);
                sound(game, "kotost_mage_items:frost.shatter", mid, 1.4, 1.0);
                if let Some(mob) = game.native_mob(dim, *id) {
                    if t.frost_damage >= mob.health {
                        let _ = game.native_kill_mob(dim, *id);
                    } else if t.frost_damage > 0.0 {
                        let _ = game.native_set_mob_health(dim, *id, mob.health - t.frost_damage);
                    }
                }
            }
            let _ = game.native_edit_blocks(dim, &edit);
        }
    }
    list.retain(|f| !(f.dim == dim && f.age >= end));
    s.frosts.extend(list);
}

// --- chain lightning -----------------------------------------------------------------------------------

fn chain(game: &mut Game<'_>, t: &Tune, players: &[NativeServerPlayer], p: &NativeServerPlayer, dim: &str) -> bool {
    let (eye, dir) = look(p);
    let mobs = game.native_mobs(dim);
    let range = f64::from(t.chain_range);
    let centers: Vec<[f64; 3]> = mobs.iter().map(|m| add(m.pos, [0.0, 0.9, 0.0])).collect();
    // The first: the mob nearest the aim, ahead, in a narrow cone.
    // The whole body counts, not one point: feet to head (a tall or low mob, a jump, a step sideways while the
    // spell gathered must not make it miss), in a cone that widens with the distance.
    let mut best: Option<(usize, f64)> = None;
    for (i, m) in mobs.iter().enumerate() {
        for h in [0.2, 0.7, 1.2, 1.7] {
            let to = sub(add(m.pos, [0.0, h, 0.0]), eye);
            let along = to[0] * dir[0] + to[1] * dir[1] + to[2] * dir[2];
            if len(to) > range + 2.0 || along <= 0.5 {
                continue;
            }
            let off = len(sub(to, mul(dir, along)));
            if off > 1.8 + along * 0.18 {
                continue;
            }
            let score = off + along * 0.03;
            if best.is_none_or(|(_, b)| score < b) {
                best = Some((i, score));
            }
        }
    }
    let mut hit: Vec<usize> = Vec::new();
    if let Some((first, _)) = best {
        hit.push(first);
        while hit.len() < t.chain_count as usize {
            let last = centers[*hit.last().expect("one at least")];
            let next = (0..centers.len())
                .filter(|j| !hit.contains(j))
                .map(|j| (j, dist(centers[j], last)))
                .filter(|&(_, d)| d <= f64::from(t.chain_jump))
                .min_by(|a, b| a.1.total_cmp(&b.1));
            match next {
                Some((j, _)) => hit.push(j),
                None => break,
            }
        }
    }
    let hand = add(eye, add(mul(dir, 0.8), [0.0, -0.35, 0.0]));
    let mut points = vec![hand];
    points.extend(hit.iter().map(|&i| centers[i]));
    if hit.is_empty() {
        // Nobody to strike: the bolt goes where the player looks.
        match ray(game, eye, dir, range) {
            Some((at, _)) => points.push(at),
            None => return false,
        }
    }
    for &i in &hit {
        let (id, health) = (mobs[i].id, mobs[i].health);
        if t.chain_damage >= health {
            let _ = game.native_kill_mob(dim, id);
        } else if t.chain_damage > 0.0 {
            let _ = game.native_set_mob_health(dim, id, health - t.chain_damage);
        }
    }
    for w in points.windows(2) {
        let (a, b) = (w[0], w[1]);
        let n = (dist(a, b) * 2.0).ceil().max(1.0) as usize;
        for k in 0..=n {
            let q = add(a, mul(sub(b, a), k as f64 / n as f64));
            let _ = game.native_send_particles_in_place(&simple(ParticleType::ElectricSpark), None, q, [0.1, 0.1, 0.1], 0.05, 2);
        }
        sound(game, "kotost_mage_items:chain.zap", b, 1.0, 1.0);
    }
    let last = *points.last().expect("two points at least");
    if t.chain_bolt {
        // A strike from the sky on the last one: drawn by the clients, heard as the mod's own thunder.
        let lower = if hit.is_empty() { 0.0 } else { 0.9 };
        let ground = [last[0], last[1] - lower, last[2]];
        let data = Writer::new(FX_SKYBOLT).v3(ground);
        broadcast(game, players, dim, &data.0);
        sound(game, "kotost_mage_items:chain.thunder", ground, 6.0, 1.0);
        let _ = game.native_send_particles_in_place(&simple(ParticleType::Flash), None, add(ground, [0.0, 1.0, 0.0]), [0.0; 3], 0.0, 1);
        let _ = game.native_send_particles_in_place(&simple(ParticleType::ElectricSpark), None, ground, [0.6, 0.2, 0.6], 0.4, 40);
        debug(&format!("chain: {} hit, sky bolt", hit.len()));
    }
    let mut data = Writer::new(FX_CHAIN).u32(points.len() as u32);
    for q in &points {
        data = data.v3(*q);
    }
    broadcast(game, players, dim, &data.0);
    true
}

// --- supernova -----------------------------------------------------------------------------------------

fn nova(
    game: &mut Game<'_>,
    s: &mut Server,
    t: &Tune,
    players: &[NativeServerPlayer],
    p: &NativeServerPlayer,
    dim: &str,
) -> bool {
    if s.novas.iter().any(|n| n.owner == p.uuid) {
        return false;
    }
    let ground = p.position;
    s.novas.push(Nova {
        dim: dim.to_owned(),
        owner: p.uuid,
        ground,
        age: 0,
    });
    let lift = Writer::new(FX_LIFT).f32(t.nova_rise).f32(t.nova_charge);
    send(game, p.uuid, CH_FX, &lift.0);
    let data = Writer::new(FX_NOVA_START)
        .v3(ground)
        .f32(t.nova_rise)
        .f32(t.nova_radius)
        .f32(t.nova_charge);
    broadcast(game, players, dim, &data.0);
    true
}

fn step_novas(game: &mut Game<'_>, s: &mut Server, t: &Tune, players: &[NativeServerPlayer], dim: &str) {
    if !s.novas.iter().any(|n| n.dim == dim) {
        return;
    }
    let charge = (t.nova_charge * 20.0).round().max(1.0) as u32;
    let r = f64::from(t.nova_radius);
    let mut list = std::mem::take(&mut s.novas);
    for n in list.iter_mut().filter(|n| n.dim == dim) {
        n.age += 1;
        let core = add(n.ground, [0.0, f64::from(t.nova_rise) + 1.0, 0.0]);
        if n.age < charge {
            let k = f64::from(n.age) / f64::from(charge);
            let a = f64::from(n.age) * 0.45;
            for j in 0..3 {
                let ang = a + f64::from(j) * std::f64::consts::TAU / 3.0;
                let rr = 3.5 * (1.0 - k) + 0.6;
                let at = add(core, [ang.cos() * rr, (a * 0.7 + f64::from(j)).sin() * rr * 0.5, ang.sin() * rr]);
                let _ = game.native_send_particles_in_place(&simple(ParticleType::EndRod), None, at, [0.0; 3], 0.0, 1);
                let _ = game.native_send_particles_in_place(&simple(ParticleType::Flame), None, at, [0.05; 3], 0.0, 1);
            }
            let _ = game.native_send_particles_in_place(&simple(ParticleType::ReversePortal), None, core, [1.5; 3], 0.5, 6);
            // The charge's sound (a reversed hit, 1.8 s) ends right on the blast.
            if n.age == charge.saturating_sub(36).max(1) {
                sound(game, "kotost_mage_items:nova.charge", core, 3.0, 1.0);
            }
            continue;
        }
        if n.age > charge {
            continue;
        }
        // The blast.
        let data = Writer::new(FX_NOVA_BOOM).v3(core).f32(r as f32);
        broadcast(game, players, dim, &data.0);
        sound(game, "kotost_mage_items:nova.boom", core, 8.0, 1.0);
        sound(game, "kotost_mage_items:meteor.rumble", core, 6.0, 1.25);
        particles_far(game, players, dim, &simple(ParticleType::ExplosionEmitter), core, [r * 0.25; 3], 0.0, 4);
        particles_far(game, players, dim, &simple(ParticleType::Flash), core, [0.5; 3], 0.0, 2);
        particles_far(game, players, dim, &simple(ParticleType::EndRod), core, [r * 0.4; 3], 0.8, 200);
        particles_far(game, players, dim, &simple(ParticleType::Flame), core, [r * 0.6, 2.0, r * 0.6], 0.5, 220);
        particles_far(game, players, dim, &simple(ParticleType::Explosion), n.ground, [r * 0.5, 1.0, r * 0.5], 0.0, 30);
        for mob in game.native_mobs(dim) {
            let off = sub(mob.pos, n.ground);
            let d = len(off).max(0.5);
            if d > r * 1.5 {
                continue;
            }
            let f = 1.0 - d / (r * 1.5);
            let push = 2.0 * f;
            let _ = game.native_push_mob(dim, mob.id, [off[0] / d * push, 0.4 + push * 0.5, off[2] / d * push]);
            let damage = t.nova_damage * f as f32;
            if damage >= mob.health {
                let _ = game.native_kill_mob(dim, mob.id);
            } else if damage > 0.5 {
                let _ = game.native_set_mob_health(dim, mob.id, mob.health - damage);
            }
        }
        if t.nova_blocks {
            s.next_id = s.next_id.wrapping_add(1);
            let (x, z) = (n.ground[0].floor() as i32, n.ground[2].floor() as i32);
            let target = [f64::from(x) + 0.5, n.ground[1].floor(), f64::from(z) + 0.5];
            s.meteors.push(Meteor {
                id: s.next_id,
                dim: dim.to_owned(),
                target,
                start: target,
                age: 0,
                fall: 1,
                radius: r * 0.6,
                phase: Phase::Carving(0),
                debris_left: 120,
                rng: 0x7F4A_7C15_9E37_79B9 ^ u64::from(s.next_id),
                dig: Dig {
                    depth: r * 0.35,
                    clear: r * 0.6,
                    rim: 1.0,
                    debris: 120.0,
                    fire: f64::from(t.nova_fire),
                    lava: 0.0,
                    smoke: 3.0,
                },
            });
        }
    }
    list.retain(|n| !(n.dim == dim && n.age >= charge));
    s.novas.extend(list);
}

// ===================================================================================================
// Client
// ===================================================================================================

struct CFireball {
    id: u32,
    pos: [f64; 3],
    vel: [f64; 3],
    gravity: f64,
    born: Instant,
    life: f32,
}

struct CMeteor {
    id: u32,
    target: [f64; 3],
    start: [f64; 3],
    radius: f64,
    fall: f64,
    size: f64,
    born: Instant,
    impact: Option<Instant>,
}

/// A light that fades: where, colour, radius, born, seconds.
struct Glow {
    at: [f64; 3],
    rgb: u32,
    radius: f32,
    born: Instant,
    life: f32,
}

/// A supernova seen charging: ground, rise, radius, charge seconds, when, when it burst.
struct CNova {
    ground: [f64; 3],
    rise: f64,
    radius: f64,
    charge: f64,
    born: Instant,
    boom: Option<Instant>,
}

/// A cast seen: whose outfit (u32::MAX for this player's own), which spell, where they stood, when, the
/// pose's seconds, whether its sparks are out yet.
struct CCast {
    key: u32,
    mob: Option<u32>,
    spell: u8,
    at: [f64; 3],
    born: Instant,
    secs: f32,
    /// Seconds it gathers before it goes off (the server's `FX_RELEASE` sets it to when it did), its
    /// mastery level, when it went off, whether it fizzled instead.
    windup: f32,
    level: u32,
    released: Option<Instant>,
    fizzled: bool,
    seed: u64,
    sparked: bool,
    /// Sparks thrown so far while the pose holds: from the crystal, from the circle's edge.
    trail: f64,
    rising: f64,
    /// Motes drawn in toward the crystal so far while it gathers.
    drawn: f64,
}

/// A small glowing cube a spell throws: it flies on, slows in the air, floats up or falls and bounces.
struct Mote {
    pos: [f64; 3],
    vel: [f64; 3],
    born: Instant,
    life: f32,
    size: f64,
    rgb: u32,
    /// Upward pull, blocks/s² (negative: it falls).
    lift: f64,
}

#[derive(Default)]
struct Client {
    have: bool,
    mana: f32,
    max: f32,
    cds: [f32; SPELLS],
    costs: [f32; SPELLS],
    /// Each spell's whole cooldown as the server counts it (the worn things shorten it); none from a
    /// server before 0.5.
    totals: Option<[f32; SPELLS]>,
    /// What the server sees on this player (`GEAR_*`), and whether its spells need the staff.
    gear: u8,
    need_staff: bool,
    /// Each spell's mastery level and the way to the next (0..1); none from a server before 0.6.
    mastery: Option<[(u32, f32); SPELLS]>,
    /// Chain lightning's arcs: their points, when.
    arcs: Vec<(Vec<[f64; 3]>, Instant)>,
    /// Absolute zero's rings: centre, radius, seconds held, when.
    frosts: Vec<([f64; 3], f64, f32, Instant)>,
    novas: Vec<CNova>,
    /// The supernova lifting this player: the height to hold, until when.
    lift: Option<(f64, Instant, Instant)>,
    got: Option<Instant>,
    fail: Option<(u8, u8, Instant)>,
    fireballs: Vec<CFireball>,
    meteors: Vec<CMeteor>,
    glows: Vec<Glow>,
    /// Shake: strength, when.
    shakes: Vec<(f32, Instant)>,
    /// Flash: strength, when.
    flash: Option<(f32, Instant)>,
    /// Dust: strength, when.
    dust: Option<(f32, Instant)>,
    /// The staff crystals drawn last frame, by wearer key: world position.
    crystals: HashMap<u32, ([f64; 3], Instant)>,
    casts: Vec<CCast>,
    motes: Vec<Mote>,
    /// Chain lightning's strikes from the sky: where, when, seed.
    skybolts: Vec<([f64; 3], Instant, u64)>,
    /// This player's own cast: the camera's kick, strength, when.
    recoil: Option<(f32, Instant)>,
    /// The last frame the motes moved.
    moved: Option<Instant>,
    rng: u64,
}

static CLIENT: Mutex<Option<Client>> = Mutex::new(None);

fn client<R>(f: impl FnOnce(&mut Client) -> R) -> R {
    let mut guard = lock(&CLIENT);
    f(guard.get_or_insert_with(Client::default))
}

/// `native_key`: the spell keys send their spell to the server.
pub(crate) fn key(game: &mut Game<'_>, key: &str, pressed: bool) {
    if !pressed || !game.native_window() || !game.native_playing() {
        return;
    }
    let Some(name) = key.strip_prefix("key.keyboard.") else {
        return;
    };
    let keys = settings(|s| s.keys.clone());
    let Some(i) = keys.iter().position(|k| k == name) else {
        return;
    };
    let spell = i as u8 + 1;
    let sent = game.native_net_send_to_server(CH_CAST, &[spell]);
    debug(&format!("key {name}: spell {spell} sent {sent:?}"));
}

/// `native_net_from_server`: mana and the spells' effects.
pub(crate) fn net_from_server(game: &mut Game<'_>, channel: &str, data: &[u8]) {
    if channel == CH_MANA {
        let mut r = Reader { data, at: 0 };
        let Some(v) = (|| {
            let (mana, max) = (r.f32()?, r.f32()?);
            let mut cds = [0.0f32; SPELLS];
            for v in &mut cds {
                *v = r.f32()?;
            }
            let mut costs = [0.0f32; SPELLS];
            for v in &mut costs {
                *v = r.f32()?;
            }
            Some((mana, max, cds, costs))
        })() else {
            return;
        };
        // Since 0.5, after those: the cooldowns' totals, the worn things, whether the staff is needed.
        let more = (|| {
            let mut totals = [0.0f32; SPELLS];
            for v in &mut totals {
                *v = r.f32()?;
            }
            Some((totals, r.u32()? as u8, r.u32()? != 0))
        })();
        // Since 0.6: each spell's mastery level and the way to the next.
        let mastery = (|| {
            let mut m = [(1u32, 0.0f32); SPELLS];
            for v in &mut m {
                *v = (r.u32()?, r.f32()?);
            }
            Some(m)
        })();
        client(|c| {
            c.have = true;
            c.mastery = mastery;
            (c.mana, c.max, c.cds, c.costs) = v;
            match more {
                Some((totals, gear, need_staff)) => {
                    c.totals = Some(totals);
                    c.gear = gear;
                    c.need_staff = need_staff;
                }
                None => {
                    c.totals = None;
                    c.gear = 0;
                    c.need_staff = false;
                }
            }
            c.got = Some(Instant::now());
        });
        return;
    }
    if channel != CH_FX {
        return;
    }
    let mut r = Reader { data, at: 0 };
    let Some(kind) = r.u8() else { return };
    debug(&format!("client fx {kind} ({} bytes)", data.len()));
    let me = game.native_player().map(|p| [p.x, p.y, p.z]);
    let now = Instant::now();
    let t = tune();
    match kind {
        FX_FIRE_SPAWN => {
            let Some((id, pos, vel, g, life)) = (|| Some((r.u32()?, r.v3()?, r.v3()?, r.f32()?, r.f32()?)))() else {
                return;
            };
            client(|c| {
                if c.fireballs.len() < 64 {
                    c.fireballs.push(CFireball {
                        id,
                        pos,
                        vel,
                        gravity: f64::from(g),
                        born: now,
                        life,
                    });
                }
            });
        }
        FX_FIRE_STEER => {
            let Some((id, pos, vel)) = (|| Some((r.u32()?, r.v3()?, r.v3()?)))() else {
                return;
            };
            client(|c| {
                if let Some(f) = c.fireballs.iter_mut().find(|f| f.id == id) {
                    // Flown on from where the server has it now; its life left as it was.
                    let gone = now.duration_since(f.born).as_secs_f32();
                    f.life = (f.life - gone).max(0.1);
                    f.pos = pos;
                    f.vel = vel;
                    f.born = now;
                }
            });
        }
        FX_RELEASE => {
            let Some((eid, spell, ok)) = (|| Some((r.u32()?, r.u32()?, r.u32()?)))() else {
                return;
            };
            let other = game.mobs.iter().find(|m| m.id == eid && &*m.kind.id == "minecraft:player").map(|m| m.id);
            let own = game.player_model.as_ref().map(|m| m.id);
            let key = if other.is_some() { eid } else { u32::MAX };
            let model = other.or(own);
            use horizon_client::kotost_mage_pose as kp;
            if let Some(id) = model {
                if ok != 0 {
                    kp::release(id);
                } else {
                    kp::cancel(id);
                }
            }
            client(|c| {
                if let Some(k) = c.casts.iter_mut().rev().find(|k| k.key == key && k.spell as u32 == spell && k.released.is_none()) {
                    let s = now.duration_since(k.born).as_secs_f32();
                    if ok != 0 {
                        // Gone off now: the pose's rest follows from here.
                        k.released = Some(now);
                        k.secs = s + (k.secs - k.windup).max(0.3);
                        k.windup = s;
                    } else {
                        k.windup = s;
                        k.secs = s + 0.25;
                        k.fizzled = true;
                    }
                }
                if ok != 0 && key == u32::MAX {
                    let kick = match spell as u8 {
                        SPELL_METEOR | SPELL_FROST => 1.0,
                        SPELL_NOVA => 0.8,
                        SPELL_CHAIN => 0.7,
                        SPELL_STEP => 0.3,
                        _ => 0.6,
                    };
                    c.recoil = Some((kick * t.recoil, now));
                }
            });
        }
        FX_FIRE_END => {
            let Some((id, pos, hit)) = (|| Some((r.u32()?, r.v3()?, r.u32()?)))() else {
                return;
            };
            if hit != 0 {
                horizon_client_render::kotost_mage_mute::mute("minecraft:entity.generic.explode", pos, 6.0, 1.0);
            }
            client(|c| {
                c.fireballs.retain(|f| f.id != id);
                if hit != 0 {
                    spark(c, pos, 18, 0xFF9A40, 0xFFE0A0, 3.5, 0.6);
                    c.glows.push(Glow {
                        at: pos,
                        rgb: 0xFF7A20,
                        radius: 10.0,
                        born: now,
                        life: 0.6,
                    });
                    if let Some(me) = me {
                        let d = dist(me, pos);
                        if d < 12.0 {
                            c.shakes.push(((0.35 * (1.0 - d / 12.0)) as f32 * t.shake, now));
                        }
                    }
                }
            });
        }
        FX_STEP => {
            let Some((from, to)) = (|| Some((r.v3()?, r.v3()?)))() else {
                return;
            };
            client(|c| {
                for at in [from, to] {
                    c.glows.push(Glow {
                        at: add(at, [0.0, 1.0, 0.0]),
                        rgb: 0x8A4CFF,
                        radius: 7.0,
                        born: now,
                        life: 0.8,
                    });
                }
            });
        }
        FX_METEOR_MARK => {
            let Some((id, target, start, radius, fall, size)) =
                (|| Some((r.u32()?, r.v3()?, r.v3()?, r.f32()?, r.u32()?, r.f32()?)))()
            else {
                return;
            };
            client(|c| {
                c.meteors.retain(|m| m.id != id);
                c.meteors.push(CMeteor {
                    id,
                    target,
                    start,
                    radius: f64::from(radius),
                    fall: f64::from(fall),
                    size: f64::from(size),
                    born: now,
                    impact: None,
                });
            });
        }
        FX_METEOR_IMPACT => {
            let Some((id, at, radius, push)) = (|| Some((r.u32()?, r.v3()?, r.f32()?, r.f32()?)))() else {
                return;
            };
            let radius = f64::from(radius);
            let d = me.map_or(1e9, |me| dist(me, at));
            horizon_client_render::kotost_mage_mute::mute("minecraft:entity.generic.explode", at, 10.0, 2.0);
            client(|c| {
                if let Some(m) = c.meteors.iter_mut().find(|m| m.id == id) {
                    m.impact = Some(now);
                } else {
                    c.meteors.push(CMeteor {
                        id,
                        target: at,
                        start: at,
                        radius,
                        fall: 1.0,
                        size: 1.0,
                        born: now,
                        impact: Some(now),
                    });
                }
                let near = |k: f64| (1.0 - d / (radius * k)).clamp(0.0, 1.0) as f32;
                c.shakes.push((t.shake * (0.25 + 1.6 * near(7.0)), now));
                c.flash = Some((t.flash * (0.35 + 0.65 * near(6.0)), now));
                if d < radius * 5.0 {
                    c.dust = Some((t.dust * (0.4 + 0.6 * near(5.0)), now));
                }
            });
            // The shockwave reaches the player.
            if let Some(me) = me
                && d < radius * 2.0
            {
                let off = sub(me, at);
                let flat = len([off[0], 0.0, off[2]]).max(0.5);
                let f = (1.0 - d / (radius * 2.0)) * f64::from(push);
                game.native_push_player([
                    (off[0] / flat * f) as f32,
                    (0.4 + f * 0.5) as f32,
                    (off[2] / flat * f) as f32,
                ]);
            }
        }
        FX_FAIL => {
            let Some((spell, why)) = (|| Some((r.u32()?, r.u32()?)))() else {
                return;
            };
            client(|c| c.fail = Some((spell as u8, why as u8, now)));
            if let Some(me) = me {
                let _ = game.native_play_local_sound("kotost_mage_items:fail", add(me, [0.0, 1.5, 0.0]), 0.6, 1.0, 0);
            }
        }
        FX_CAST => {
            let Some((eid, spell, at, secs)) = (|| Some((r.u32()?, r.u32()?, r.v3()?, r.f32()?)))() else {
                return;
            };
            // Since 0.6: the seconds it gathers before it goes off, and its mastery level.
            let windup = r.f32().unwrap_or(0.0).clamp(0.0, secs.max(0.0));
            let level = r.u32().unwrap_or(1);
            let spell = spell.clamp(1, SPELLS as u32) as u8;
            // Whose: another player's model by its entity id, else this player's own.
            let other = game
                .mobs
                .iter()
                .find(|m| m.id == eid && &*m.kind.id == "minecraft:player")
                .map(|m| (m.id, staff_arm(m).unwrap_or_else(|| m.main_arm_left())));
            let mine = other.is_none() && me.is_some_and(|me| dist(me, at) < 4.0);
            let model = if mine {
                game.player_model
                    .as_ref()
                    .map(|m| (m.id, staff_arm(m).unwrap_or_else(|| m.main_arm_left())))
            } else {
                other
            };
            use horizon_client::kotost_mage_pose as kp;
            let kind = match spell {
                SPELL_FIRE => kp::THRUST,
                SPELL_STEP => kp::SWEEP,
                SPELL_FROST => kp::SLAM,
                SPELL_CHAIN => kp::PALM,
                _ => kp::RAISE,
            };
            if let Some((id, left)) = model {
                kp::start(id, kind, secs, left, windup);
            }
            let key = if mine { u32::MAX } else { eid };
            // The dress and the hat jump with the cast.
            if let Some(w) = lock(&SKIRTS).as_mut().and_then(|all| all.get_mut(&key)) {
                w.flare.v += 7.0 * t.cast_kick;
                w.back.v -= 5.0 * t.cast_kick;
                w.side.v += if spell % 2 == 0 { 4.0 } else { -4.0 } * t.cast_kick;
                w.hat.v += 9.0 * t.cast_kick;
            }
            client(|c| {
                c.rng = c.rng.wrapping_add(0x9E37_79B9_7F4A_7C15);
                if c.casts.len() > 24 {
                    c.casts.remove(0);
                }
                c.casts.push(CCast {
                    key,
                    mob: model.map(|m| m.0),
                    spell,
                    at,
                    born: now,
                    secs,
                    windup,
                    level,
                    released: None,
                    fizzled: false,
                    seed: c.rng,
                    sparked: false,
                    trail: 0.0,
                    rising: 0.0,
                    drawn: 0.0,
                });
            });
        }
        FX_SKYBOLT => {
            let Some(at) = r.v3() else { return };
            let d = me.map_or(1e9, |me| dist(me, at));
            client(|c| {
                c.rng = c.rng.wrapping_add(0x5851_F42D_4C95_7F2D);
                if c.skybolts.len() < 8 {
                    c.skybolts.push((at, now, c.rng));
                }
                c.glows.push(Glow {
                    at: add(at, [0.0, 2.0, 0.0]),
                    rgb: 0xCFE0FF,
                    radius: 16.0,
                    born: now,
                    life: 0.6,
                });
                spark(c, add(at, [0.0, 0.3, 0.0]), 24, 0xBFD4FF, 0xFFFFFF, 5.0, -6.0);
                let near = (1.0 - d / 40.0).clamp(0.0, 1.0) as f32;
                if near > 0.0 {
                    c.flash = Some((t.flash * 0.45 * near, now));
                    c.shakes.push((t.shake * 0.5 * near, now));
                }
            });
        }
        FX_FROST => {
            let Some((at, radius, secs)) = (|| Some((r.v3()?, r.f32()?, r.f32()?)))() else {
                return;
            };
            client(|c| {
                if c.frosts.len() < 16 {
                    c.frosts.push((at, f64::from(radius), secs, now));
                }
                if let Some(me) = me
                    && dist(me, at) < f64::from(radius)
                {
                    c.shakes.push((0.25 * t.shake, now));
                }
            });
        }
        FX_CHAIN => {
            let Some(n) = r.u32() else { return };
            let mut points = Vec::new();
            for _ in 0..n.min(64) {
                let Some(q) = r.v3() else { return };
                points.push(q);
            }
            client(|c| {
                if let Some(&last) = points.last() {
                    c.glows.push(Glow {
                        at: last,
                        rgb: 0x9FC8FF,
                        radius: 10.0,
                        born: now,
                        life: 0.5,
                    });
                }
                if c.arcs.len() < 16 {
                    c.arcs.push((points, now));
                }
            });
        }
        FX_NOVA_START => {
            let Some((ground, rise, radius, charge)) = (|| Some((r.v3()?, r.f32()?, r.f32()?, r.f32()?)))() else {
                return;
            };
            client(|c| {
                c.novas.retain(|n| dist(n.ground, ground) > 0.5);
                c.novas.push(CNova {
                    ground,
                    rise: f64::from(rise),
                    radius: f64::from(radius),
                    charge: f64::from(charge),
                    born: now,
                    boom: None,
                });
            });
        }
        FX_NOVA_BOOM => {
            let Some((core, radius)) = (|| Some((r.v3()?, r.f32()?)))() else {
                return;
            };
            let d = me.map_or(1e9, |me| dist(me, core));
            let radius = f64::from(radius);
            client(|c| {
                let near = |k: f64| (1.0 - d / (radius * k)).clamp(0.0, 1.0) as f32;
                if let Some(n) = c.novas.iter_mut().find(|n| dist(add(n.ground, [0.0, n.rise + 1.0, 0.0]), core) < 2.0) {
                    n.boom = Some(now);
                }
                c.flash = Some((t.flash * (0.3 + 0.7 * near(8.0)), now));
                c.shakes.push((t.shake * (0.2 + 1.0 * near(6.0)), now));
                c.glows.push(Glow {
                    at: core,
                    rgb: 0xFFF0C0,
                    radius: 16.0,
                    born: now,
                    life: 1.5,
                });
                c.lift = None;
            });
        }
        FX_LIFT => {
            let Some((rise, secs)) = (|| Some((r.f32()?, r.f32()?)))() else {
                return;
            };
            if let Some(me) = me {
                client(|c| {
                    let until = now + Duration::from_secs_f32(secs.clamp(0.1, 10.0));
                    c.lift = Some((me[1] + f64::from(rise), now, until));
                });
            }
        }
        _ => {}
    }
}

/// `native_tick` (the window's): a supernova holds its caster up in the air while it charges.
pub(crate) fn client_tick(game: &mut Game<'_>) {
    if !game.native_window() {
        return;
    }
    let now = Instant::now();
    let Some((target, _, until)) = client(|c| {
        if c.lift.is_some_and(|(_, _, until)| now > until) {
            c.lift = None;
        }
        c.lift
    }) else {
        return;
    };
    let Some((y, vy)) = game.player.as_ref().map(|p| (p.position.y, p.velocity.y)) else {
        return;
    };
    let _ = until;
    // Rise to the height and hold there: the velocity wanted, against what the player has.
    let want = ((target - y) * 0.25).clamp(-0.2, 0.35);
    let push = (want - vy).clamp(-0.5, 0.6);
    if push.abs() > 0.002 {
        game.native_push_player([0.0, push as f32, 0.0]);
    }
}

/// Where a fireball is now, `secs` after it was thrown (the server's ticks, predicted).
fn fireball_at(f: &CFireball, secs: f64) -> [f64; 3] {
    let n = secs * 20.0;
    [
        f.pos[0] + f.vel[0] * n,
        f.pos[1] + f.vel[1] * n - 0.5 * f.gravity * n * n,
        f.pos[2] + f.vel[2] * n,
    ]
}

/// Where a meteor's rock is, `u` (0..1) of the way down.
fn rock_at(m: &CMeteor, u: f64) -> [f64; 3] {
    add(m.start, mul(sub(m.target, m.start), u * u))
}

fn cube_prim(p: &mut horizon_client::world_primitives::WorldPrimitives, c: [f64; 3], h: f64, argb: u32) {
    p.filled_box([c[0] - h, c[1] - h, c[2] - h], [c[0] + h, c[1] + h, c[2] + h], argb);
}

fn argb(a: f64, rgb: u32) -> u32 {
    ((a.clamp(0.0, 1.0) * 255.0) as u32) << 24 | (rgb & 0xFF_FFFF)
}

fn circle(
    p: &mut horizon_client::world_primitives::WorldPrimitives,
    c: [f64; 3],
    r: f64,
    color: u32,
    width: f32,
    walls: bool,
) {
    let n = ((r * 2.5) as usize).clamp(24, 160);
    for i in 0..n {
        let a0 = i as f64 / n as f64 * std::f64::consts::TAU;
        let a1 = (i + 1) as f64 / n as f64 * std::f64::consts::TAU;
        let from = [c[0] + r * a0.cos(), c[1], c[2] + r * a0.sin()];
        let to = [c[0] + r * a1.cos(), c[1], c[2] + r * a1.sin()];
        let line = p.line(from, to, color, width);
        if walls {
            line.through_walls();
        }
    }
}

/// A jagged line from `a` to `b`, new every frame (`seed`), offsets up to `amp` blocks.
fn bolt(p: &mut horizon_client::world_primitives::WorldPrimitives, a: [f64; 3], b: [f64; 3], seed: u64, amp: f64) {
    let mut rng = seed | 1;
    let n = ((dist(a, b) / 1.2).ceil() as usize).clamp(3, 24);
    let mut prev = a;
    for k in 1..=n {
        let u = k as f64 / n as f64;
        let mut q = add(a, mul(sub(b, a), u));
        if k < n {
            let j = amp * (1.0 - (u - 0.5).abs() * 1.2);
            q = add(q, [(rand01(&mut rng) - 0.5) * j, (rand01(&mut rng) - 0.5) * j, (rand01(&mut rng) - 0.5) * j]);
        }
        let _ = p.line(prev, q, 0x5A_6FA8FF, 7.0);
        let _ = p.line(prev, q, 0xFF_F2F6FF, 2.5);
        prev = q;
    }
}

/// A lightning strike: thick and bright, `alive` its strength now.
fn bolt_big(p: &mut horizon_client::world_primitives::WorldPrimitives, a: [f64; 3], b: [f64; 3], seed: u64, amp: f64, alive: f64) {
    let mut rng = seed | 1;
    let n = ((dist(a, b) / 2.5).ceil() as usize).clamp(4, 40);
    let mut prev = a;
    for k in 1..=n {
        let u = k as f64 / n as f64;
        let mut q = add(a, mul(sub(b, a), u));
        if k < n {
            let j = amp * (1.0 - u * 0.7);
            q = add(q, [(rand01(&mut rng) - 0.5) * j, (rand01(&mut rng) - 0.5) * j * 0.3, (rand01(&mut rng) - 0.5) * j]);
        }
        let _ = p.line(prev, q, argb(0.45 * alive, 0x7FA0FF), 14.0);
        let _ = p.line(prev, q, argb(0.95 * alive, 0xF4F8FF), 4.0);
        prev = q;
    }
}

/// Throws `n` glowing cubes from `at`: half `rgb`, half `hot`, `speed` blocks/s out, `lift` up (negative falls).
fn spark(c: &mut Client, at: [f64; 3], n: usize, rgb: u32, hot: u32, speed: f64, lift: f64) {
    let now = Instant::now();
    for i in 0..n {
        if c.motes.len() >= 900 {
            c.motes.remove(0);
        }
        let dir = [
            (rand01(&mut c.rng) - 0.5) * 2.0,
            (rand01(&mut c.rng) - 0.5) * 2.0 + 0.35,
            (rand01(&mut c.rng) - 0.5) * 2.0,
        ];
        let l = len(dir).max(1e-3);
        let v = speed * (0.45 + rand01(&mut c.rng) * 0.75);
        let size = 0.035 + rand01(&mut c.rng) * 0.05;
        let life = 0.7 + rand01(&mut c.rng) as f32 * 0.9;
        c.motes.push(Mote {
            pos: at,
            vel: mul(dir, v / l),
            born: now,
            life,
            size,
            rgb: if i % 2 == 0 { rgb } else { hot },
            lift,
        });
    }
}

/// The spell's colours: the circle's and the runes'.
fn spell_colors(spell: u8) -> (u32, u32) {
    match spell {
        SPELL_FIRE => (0xB46CFF, 0xFF9A48),
        SPELL_STEP => (0x9A5CFF, 0xE2C8FF),
        SPELL_METEOR => (0xB46CFF, 0xFF5050),
        SPELL_FROST => (0x8FD8FF, 0xE8FAFF),
        SPELL_CHAIN => (0xA8B8FF, 0xFFFFFF),
        _ => (0xFFC860, 0xFFF6DC),
    }
}

/// Runes: each a few strokes in a unit square (across, out), drawn between the circle's two rings.
const RUNES: [&[[f32; 4]]; 10] = [
    &[[0.0, -0.5, 0.0, 0.5], [0.0, 0.0, 0.4, 0.35], [0.0, -0.3, 0.4, 0.05]],
    &[[-0.3, -0.5, -0.3, 0.5], [-0.3, 0.5, 0.3, 0.2], [0.3, 0.2, 0.3, -0.5]],
    &[[0.0, -0.5, 0.0, 0.5], [0.0, 0.25, 0.35, 0.0], [0.35, 0.0, 0.0, -0.25]],
    &[[-0.35, 0.5, 0.0, -0.5], [0.0, -0.5, 0.35, 0.5], [-0.2, 0.0, 0.2, 0.0]],
    &[[-0.35, -0.5, 0.35, 0.5], [0.35, -0.5, -0.35, 0.5]],
    &[[0.0, -0.5, 0.0, 0.5], [-0.35, -0.15, 0.35, 0.15], [-0.35, 0.15, 0.35, -0.15]],
    &[[-0.35, -0.45, 0.35, -0.45], [0.35, -0.45, -0.35, 0.45], [-0.35, 0.45, 0.35, 0.45]],
    &[[0.0, -0.5, 0.0, 0.5], [0.0, -0.5, 0.35, -0.2], [0.0, -0.1, 0.35, 0.2]],
    &[[-0.3, 0.5, -0.3, -0.5], [-0.3, -0.5, 0.3, 0.0], [0.3, 0.0, -0.3, 0.5]],
    &[[-0.35, 0.0, 0.0, -0.5], [0.0, -0.5, 0.35, 0.0], [0.35, 0.0, 0.0, 0.5], [0.0, 0.5, -0.35, 0.0]],
];

type Prims = horizon_client::world_primitives::WorldPrimitives;

/// A ring on the ground (or any level) round `c`, `rr` wide, turned by `spin`, every third piece left out
/// when `dash`.
fn flat_ring(p: &mut Prims, c: [f64; 3], rr: f64, spin: f64, color: u32, width: f32, dash: bool) {
    let n = ((rr * 14.0) as usize).clamp(24, 96);
    for i in 0..n {
        if dash && i % 3 == 2 {
            continue;
        }
        let a0 = i as f64 / n as f64 * std::f64::consts::TAU + spin;
        let a1 = (i + 1) as f64 / n as f64 * std::f64::consts::TAU + spin;
        let _ = p.line(
            [c[0] + rr * a0.cos(), c[1], c[2] + rr * a0.sin()],
            [c[0] + rr * a1.cos(), c[1], c[2] + rr * a1.sin()],
            color,
            width,
        );
    }
}

/// A magic circle on the ground at `c`: two rings with runes between, a turning hexagram, a small ring at
/// the heart, `r` blocks wide, turned by `spin`, `a` opaque.
fn rune_circle(p: &mut Prims, c: [f64; 3], r: f64, spin: f64, main: u32, glyph: u32, a: f64, seed: u64) {
    rune_ring(p, c, [1.0, 0.0, 0.0], [0.0, 0.0, 1.0], r, spin, main, glyph, a, seed, 1.0);
}

/// A magic circle at `c` in the plane of `u` and `v` (unit, at right angles): two rings with runes between,
/// a turning hexagram, a small ring at the heart, `r` blocks wide, turned by `spin`, `a` opaque, its lines
/// `wk` times the ground circle's width.
#[allow(clippy::too_many_arguments)]
fn rune_ring(p: &mut Prims, c: [f64; 3], u: [f64; 3], v: [f64; 3], r: f64, spin: f64, main: u32, glyph: u32, a: f64, seed: u64, wk: f32) {
    use std::f64::consts::TAU;
    if a <= 0.01 {
        return;
    }
    let at = |rr: f64, ang: f64| add(c, add(mul(u, rr * ang.cos()), mul(v, rr * ang.sin())));
    let ring = |p: &mut Prims, rr: f64, sp: f64, color: u32, w: f32, dash: bool| {
        let n = ((rr * 14.0) as usize).clamp(24, 96);
        for i in 0..n {
            if dash && i % 3 == 2 {
                continue;
            }
            let a0 = i as f64 / n as f64 * TAU + sp;
            let a1 = (i + 1) as f64 / n as f64 * TAU + sp;
            let _ = p.line(at(rr, a0), at(rr, a1), color, w * wk);
        }
    };
    ring(p, r, spin, argb(a, main), 4.0, false);
    ring(p, r * 1.03, spin, argb(a * 0.35, glyph), 7.0, false);
    ring(p, r * 0.8, spin, argb(a * 0.85, main), 2.5, false);
    ring(p, r * 0.3, -spin * 2.0, argb(a * 0.9, glyph), 2.5, true);
    // The runes, between the rings, turning with the outer one.
    let count = 12;
    let size = r * 0.13;
    for i in 0..count {
        let ang = i as f64 / count as f64 * TAU + spin;
        let centre = at(r * 0.9, ang);
        let (sn, cs) = ang.sin_cos();
        // Out: away from the heart; across: along the ring.
        let out = add(mul(u, cs), mul(v, sn));
        let across = add(mul(u, -sn), mul(v, cs));
        let rune = RUNES[((seed >> (i % 16)) as usize + i * 7) % RUNES.len()];
        for st in rune {
            let pt = |x: f32, y: f32| add(centre, add(mul(across, f64::from(x) * size), mul(out, f64::from(y) * size)));
            let _ = p.line(pt(st[0], st[1]), pt(st[2], st[3]), argb(a, glyph), 2.5 * wk);
        }
    }
    // A hexagram turning the other way.
    for k in 0..2 {
        let off = -spin * 1.6 + f64::from(k) * std::f64::consts::PI / 3.0;
        let pts: Vec<[f64; 3]> = (0..3).map(|j| at(r * 0.78, off + f64::from(j) * TAU / 3.0)).collect();
        for j in 0..3 {
            let _ = p.line(pts[j], pts[(j + 1) % 3], argb(a * 0.8, main), 2.0 * wk);
        }
    }
}

/// A shaft of light standing on `base`, `h` blocks tall, `w` blocks thick: a bright core in `core`, a wide
/// glow in `glow`, both fading toward the top; `a` opaque.
fn light_shaft(p: &mut Prims, base: [f64; 3], h: f64, w: f64, core: u32, glow: u32, a: f64) {
    if a <= 0.01 || h <= 0.05 {
        return;
    }
    let n = ((h / 0.7).ceil() as usize).clamp(3, 20);
    for j in 0..n {
        let (y0, y1) = (h * j as f64 / n as f64, h * (j + 1) as f64 / n as f64);
        let fall = (1.0 - (j as f64 + 0.5) / n as f64).powf(1.3);
        let (cw, gw) = (w * 0.3, w);
        p.filled_box(
            [base[0] - cw, base[1] + y0, base[2] - cw],
            [base[0] + cw, base[1] + y1, base[2] + cw],
            argb(a * 0.85 * fall, core),
        );
        p.filled_box(
            [base[0] - gw, base[1] + y0, base[2] - gw],
            [base[0] + gw, base[1] + y1, base[2] + gw],
            argb(a * 0.22 * fall, glow),
        );
    }
}

/// `native_world_primitives`: fireballs, the meteor's ring, rock and trail, the shockwave ring, chain
/// lightning's arcs, absolute zero's ring, the supernova's charge and blast.
pub fn primitives(
    game: &Game<'_>,
    _partial: f32,
    _camera: [f64; 3],
    p: &mut horizon_client::world_primitives::WorldPrimitives,
) {
    let now = Instant::now();
    let t = tune();
    // Where each caster stands now: this player, or another player's model.
    let me = game.native_player().map(|q| ([q.x, q.y, q.z], f64::from(q.yaw)));
    let first_person = game.camera_type.first_person();
    let keys: Vec<(u32, Option<u32>)> = client(|c| c.casts.iter().map(|k| (k.key, k.mob)).collect());
    // Each caster's feet and the way it faces (yaw, degrees).
    let feet: Vec<(u32, [f64; 3], f64)> = keys
        .into_iter()
        .filter_map(|(key, mob)| {
            if key == u32::MAX {
                return me.map(|(m, yaw)| (key, m, yaw));
            }
            let id = mob?;
            game.mobs
                .iter()
                .find(|m| m.id == id)
                .map(|m| (key, [m.position.x, m.position.y, m.position.z], f64::from(m.head_yaw)))
        })
        .collect();
    let mut solid_cache: HashMap<[i32; 3], bool> = HashMap::new();
    let mut is_solid = |q: [f64; 3]| {
        let cell = [q[0].floor() as i32, q[1].floor() as i32, q[2].floor() as i32];
        *solid_cache.entry(cell).or_insert_with(|| solid(game, cell[0], cell[1], cell[2]))
    };
    client(|c| {
        // Casts: the circle of runes under the caster, a ring rising round them, streaks of light, the
        // crystal's flash and the sparks it throws.
        c.casts.retain(|k| now.duration_since(k.born).as_secs_f32() < k.secs + 0.6);
        // Sparks to throw: where, the spell, how many, how fast, their lift.
        let mut sparks: Vec<([f64; 3], u8, usize, f64, f64)> = Vec::new();
        let dense = f64::from(t.sparks);
        let mut spark_puff: Vec<[f64; 3]> = Vec::new();
        for k in &mut c.casts {
            let s = now.duration_since(k.born).as_secs_f64();
            let total = f64::from(k.secs) + 0.6;
            let (at, yaw) = feet.iter().find(|f| f.0 == k.key).map_or((k.at, 0.0), |f| (f.1, f.2));
            let (main, glyph) = spell_colors(k.spell);
            // It gathers for `windup` seconds (0..1 here), then goes off: `sr` seconds since (negative before).
            let w = f64::from(k.windup);
            let gather = if w > 0.0 { (s / w).min(1.0) } else { 1.0 };
            let ease = gather * gather * (3.0 - 2.0 * gather);
            let sr = s - w;
            let out = !k.fizzled && sr >= 0.0;
            let fade_in = (s / 0.12).min(1.0);
            let fade_out = ((total - s) / 0.45).clamp(0.0, 1.0);
            let a = fade_in * fade_out;
            // The circle pops a little bigger the moment it goes off.
            let pop = if out { 1.0 + 0.25 * (1.0 - (sr / 0.15).min(1.0)) } else { 1.0 };
            let r = 1.55 * (0.3 + 0.7 * ease) * pop;
            if t.cast_fx {
                let ground = add(at, [0.0, 0.06, 0.0]);
                // Gathering, the runes turn faster and faster; once off, slowly.
                let spin = if sr < 0.0 { s * (0.9 + 2.6 * gather) } else { w * 2.2 + sr * 0.9 };
                rune_circle(p, ground, r, spin, main, glyph, a, k.seed);
                // A second, inner ring of runes for a spell mastered past level 4.
                if k.level >= 5 {
                    rune_circle(p, add(ground, [0.0, 0.02, 0.0]), r * 0.55, -spin * 1.4, glyph, main, a * 0.8, k.seed.rotate_left(7));
                }
                // A dashed ring rising round the waist; while it gathers, rings closing in on the caster.
                let rise = (s / total).min(1.0);
                flat_ring(
                    p,
                    add(at, [0.0, 0.35 + 1.4 * rise, 0.0]),
                    1.05 + 0.35 * rise,
                    -s * 2.2,
                    argb(a * 0.7 * (1.0 - rise * 0.6), main),
                    2.5,
                    true,
                );
                if sr < 0.0 {
                    for j in 0..2 {
                        let q = ((gather * 1.6 + f64::from(j) * 0.5) % 1.0).max(0.0);
                        let rr = 2.6 * (1.0 - q) + 0.5;
                        flat_ring(p, add(at, [0.0, 0.25 + 0.9 * q, 0.0]), rr, s * 3.0, argb(a * 0.55 * q, glyph), 2.0, false);
                    }
                }
                // Streaks of light going up at the circle's edge, once it is off.
                if out && sr < 0.7 {
                    let k2 = sr / 0.7;
                    for j in 0..6 {
                        let ang = f64::from(j) / 6.0 * std::f64::consts::TAU + (k.seed % 7) as f64;
                        let base = add(at, [r * 0.95 * ang.cos(), 0.1 + k2 * 1.5, r * 0.95 * ang.sin()]);
                        let _ = p.line(base, add(base, [0.0, 0.5 + 0.8 * (1.0 - k2), 0.0]), argb(0.8 * (1.0 - k2), glyph), 3.0);
                    }
                }
            }
            // The crystal now (drawn last frame), if it is.
            let crystal = c.crystals.get(&k.key).map(|&(cp, _)| cp);
            if t.cast_fx && t.pillar {
                // Shafts of light at the circle's edge: low and flickering while it gathers, up once it's off.
                let grow = if out { 0.35 + 0.65 * (sr / 0.35).min(1.0) } else { 0.35 * ease };
                let mut rng = k.seed | 1;
                for j in 0..8 {
                    let ang = f64::from(j) / 8.0 * std::f64::consts::TAU + rand01(&mut rng) * 0.5 + s * 0.9;
                    let h = (1.4 + 2.2 * rand01(&mut rng)) * grow;
                    let flick = 0.75 + 0.25 * (s * 9.0 + f64::from(j) * 1.7).sin();
                    let base = add(at, [r * 0.93 * ang.cos(), 0.06, r * 0.93 * ang.sin()]);
                    light_shaft(p, base, h, 0.09, glyph, main, a * 0.8 * flick);
                }
                // The pillar: from the crystal (or over the head) up into the sky, for the big spells.
                if out && matches!(k.spell, SPELL_METEOR | SPELL_NOVA | SPELL_FROST | SPELL_CHAIN) {
                    let life = (f64::from(k.secs) - w).min(1.4);
                    let pa = (sr / 0.08).min(1.0) * ((life + 0.35 - sr) / 0.35).clamp(0.0, 1.0);
                    let base = crystal.unwrap_or(add(at, [0.0, 2.3, 0.0]));
                    let pw = 0.1 + 0.22 * (sr / 0.2).min(1.0);
                    let h = 4.0 + 22.0 * (sr / 0.3).min(1.0);
                    light_shaft(p, base, h, pw, 0xFFFFFF, main, pa);
                    light_shaft(p, base, h * 0.6, pw * 2.2, glyph, main, pa * 0.35);
                }
                // A ring of runes standing up at the staff, facing where the caster faces; it grows as it
                // gathers.
                if let Some(cp) = crystal {
                    let yr = yaw.to_radians();
                    let facing = [-yr.sin(), 0.0, yr.cos()];
                    let right = [yr.cos(), 0.0, yr.sin()];
                    let fp_own = first_person && k.key == u32::MAX;
                    let rr = if fp_own { 0.34 } else { 0.8 } * (0.3 + 0.7 * ease) * pop;
                    let centre = add(cp, mul(facing, if fp_own { 0.15 } else { 0.28 }));
                    let spin = if sr < 0.0 { s * (1.8 + 4.0 * gather) } else { w * 4.0 + sr * 1.8 };
                    rune_ring(p, centre, right, [0.0, 1.0, 0.0], rr, spin, main, glyph, a * 0.9, k.seed.rotate_left(13), if fp_own { 0.75 } else { 1.1 });
                }
            }
            // While it gathers: the crystal swells with light and draws light in, threads of it reaching in.
            if let Some(cp) = crystal
                && sr < 0.0
                && !k.fizzled
            {
                let throb = 1.0 + 0.15 * (s * 18.0).sin();
                cube_prim(p, cp, (0.03 + 0.07 * ease) * throb, argb(0.9 * ease, 0xFFFFFF));
                for j in 0..6 {
                    let ang = f64::from(j) / 6.0 * std::f64::consts::TAU + s * 2.5;
                    let reach = 1.3 * (1.0 - ((s * 2.2 + f64::from(j) * 0.17) % 1.0));
                    let from = add(cp, [reach * ang.cos(), 0.35 * (ang * 2.0).sin(), reach * ang.sin()]);
                    let tip = add(cp, mul(sub(from, cp), 0.35));
                    let _ = p.line(from, tip, argb(0.7 * ease, glyph), 2.0);
                }
            }
            // The crystal the moment it goes off: a flash, rays, sparks once.
            if let Some(cp) = crystal
                && out
                && sr < 0.35
            {
                let q = sr / 0.35;
                cube_prim(p, cp, 0.06 + 0.22 * q, argb(0.95 * (1.0 - q), 0xF6EEFF));
                cube_prim(p, cp, 0.12 + 0.42 * q, argb(0.45 * (1.0 - q), main));
                for j in 0..8 {
                    let ang = f64::from(j) / 8.0 * std::f64::consts::TAU + sr * 3.0;
                    let tilt = if j % 2 == 0 { 0.6 } else { -0.4 };
                    let dir = [ang.cos(), tilt, ang.sin()];
                    let l0 = 0.1 + 0.3 * q;
                    let l1 = l0 + 0.35 * (1.0 - q) + 0.1;
                    let _ = p.line(add(cp, mul(dir, l0)), add(cp, mul(dir, l1)), argb(0.9 * (1.0 - q), glyph), 2.0);
                }
            }
            let from = crystal.unwrap_or(add(at, [0.0, 1.6, 0.0]));
            // Gathering: motes pulled in from round the crystal.
            if sr < 0.0 && !k.fizzled && t.cast_fx {
                let want = s * 40.0 * dense;
                while k.drawn + 1.0 <= want {
                    k.drawn += 1.0;
                    let ang = k.drawn * 2.399;
                    let up = ((k.drawn * 0.618) % 1.0) * 2.0 - 1.0;
                    let rr = 1.2 + 0.4 * ((k.drawn * 0.371) % 1.0);
                    let ring = (1.0 - up * up).sqrt();
                    let off = [rr * ring * ang.cos(), rr * up * 0.8, rr * ring * ang.sin()];
                    if c.motes.len() >= 900 {
                        c.motes.remove(0);
                    }
                    c.motes.push(Mote {
                        pos: add(from, off),
                        vel: mul(off, -1.0 / 0.32),
                        born: now,
                        life: 0.3,
                        size: 0.035,
                        rgb: if k.drawn as u64 % 3 == 0 { glyph } else { main },
                        lift: 0.0,
                    });
                }
            }
            if !k.sparked && out && (crystal.is_some() || sr > 0.05) {
                k.sparked = true;
                // A burst from the crystal, and a ring of sparks from the circle's edge floating up.
                sparks.push((from, k.spell, (36.0 * dense) as usize, 3.4, 0.9));
                if t.cast_fx {
                    let n = (24.0 * dense) as usize;
                    for j in 0..n {
                        let ang = j as f64 / n as f64 * std::f64::consts::TAU;
                        sparks.push((add(at, [1.45 * ang.cos(), 0.1, 1.45 * ang.sin()]), k.spell, 1, 0.6, 1.8));
                    }
                }
            }
            // Fizzled: a grey puff from the crystal, once.
            if k.fizzled && !k.sparked {
                k.sparked = true;
                spark_puff.push(from);
            }
            // Once off, while the pose holds: a stream from the crystal, and sparks rising off the circle.
            if out && s < f64::from(k.secs) {
                let want = sr * 34.0 * dense;
                while k.trail + 1.0 <= want {
                    k.trail += 1.0;
                    sparks.push((from, k.spell, 1, 1.3, 0.5));
                }
            }
            if !k.fizzled && s < f64::from(k.secs) && t.cast_fx {
                let want = s * 16.0 * dense;
                while k.rising + 1.0 <= want {
                    k.rising += 1.0;
                    let ang = k.rising * 2.399;
                    let rr = 0.6 + 0.85 * ((k.rising * 0.618) % 1.0);
                    sparks.push((add(at, [rr * ang.cos(), 0.1, rr * ang.sin()]), k.spell, 1, 0.35, 2.2));
                }
            }
        }
        for at in spark_puff {
            spark(c, at, 14, 0x707078, 0xB0B0B8, 1.2, 0.8);
        }
        for (at, spell, n, speed, lift) in sparks {
            let (main, glyph) = spell_colors(spell);
            spark(c, at, n, main, glyph, speed, lift);
        }
        // The motes: inertia, air drag, their pull up or down, a bounce off the ground.
        let dt = c.moved.map_or(0.0, |m| now.duration_since(m).as_secs_f64()).min(0.05);
        c.moved = Some(now);
        c.motes.retain(|m| now.duration_since(m.born).as_secs_f32() < m.life);
        for m in &mut c.motes {
            let drag = (-2.2 * dt).exp();
            m.vel = mul(m.vel, drag);
            m.vel[1] += m.lift * dt;
            let next = add(m.pos, mul(m.vel, dt));
            if m.vel[1] < 0.0 && is_solid(next) {
                m.vel = [m.vel[0] * 0.6, -m.vel[1] * 0.45, m.vel[2] * 0.6];
            } else {
                m.pos = next;
            }
            let age = now.duration_since(m.born).as_secs_f32() / m.life;
            let fade = f64::from(1.0 - age * age);
            let twinkle = 0.75 + 0.25 * ((now.duration_since(m.born).as_secs_f64() * 23.0 + m.size * 400.0).sin());
            cube_prim(p, m.pos, m.size * (0.5 + 0.5 * fade), argb(0.95 * fade * twinkle, m.rgb));
            cube_prim(p, m.pos, m.size * 1.8 * fade, argb(0.25 * fade, m.rgb));
        }
        // Chain lightning's strike from the sky.
        c.skybolts.retain(|(_, at, _)| now.duration_since(*at).as_secs_f32() < 0.45);
        let frame = (now.duration_since(c.got.unwrap_or(now)).as_millis() / 45) as u64;
        for &(g, born, seed) in &c.skybolts {
            let s = now.duration_since(born).as_secs_f64();
            let top = add(g, [((seed % 13) as f64 - 6.0) * 0.6, 70.0, ((seed % 11) as f64 - 5.0) * 0.6]);
            let alive = if s < 0.12 || (0.2..0.3).contains(&s) { 1.0 } else { 0.35 };
            let sd = seed ^ frame.wrapping_mul(0x9E37_79B9);
            bolt_big(p, top, g, sd, 4.5, alive);
            let mid = add(g, [0.0, 25.0 + (seed % 9) as f64, 0.0]);
            let side = add(g, [((seed >> 3) % 9) as f64 - 4.0, 4.0, ((seed >> 5) % 9) as f64 - 4.0]);
            bolt_big(p, mid, side, sd ^ 0x55, 2.0, alive * 0.6);
            if s < 0.15 {
                circle(p, add(g, [0.0, 0.1, 0.0]), 1.0 + s * 20.0, argb(0.8 * (1.0 - s / 0.15), 0xE8F0FF), 5.0, false);
            }
        }
    });
    // Aim help while this player's own spell gathers: what it will hit.
    if t.aim
        && let Some(q) = game.native_player()
        && let Some((spell, gather)) = client(|c| {
            c.casts.iter().rev().find(|k| k.key == u32::MAX && k.released.is_none() && !k.fizzled).and_then(|k| {
                let s = now.duration_since(k.born).as_secs_f32();
                (s < k.windup).then(|| (k.spell, s / k.windup))
            })
        })
    {
        aim_preview(game, p, &t, [q.x, q.y, q.z], q.yaw, q.pitch, spell, f64::from(gather));
    }
    {
        static LAST: Mutex<Option<Instant>> = Mutex::new(None);
        let mut last = lock(&LAST);
        if last.is_none_or(|t| now.duration_since(t) > Duration::from_secs(1)) {
            *last = Some(now);
            let counts = client(|c| (c.fireballs.len(), c.meteors.len(), c.arcs.len(), c.frosts.len(), c.novas.len()));
            if counts != (0, 0, 0, 0, 0) {
                debug(&format!("shapes: fireballs/meteors/arcs/frosts/novas {counts:?}"));
            }
        }
    }
    client(|c| {
        c.arcs.retain(|(_, at)| now.duration_since(*at).as_secs_f32() < 0.45);
        let frame = (now.duration_since(c.got.unwrap_or(now)).as_millis() / 50) as u64;
        for (k, (points, _)) in c.arcs.iter().enumerate() {
            for (i, w) in points.windows(2).enumerate() {
                bolt(p, w[0], w[1], frame.wrapping_mul(0x9E37_79B9) ^ (k as u64) << 8 ^ i as u64, 1.4);
            }
        }
        c.frosts.retain(|(_, _, secs, at)| now.duration_since(*at).as_secs_f32() < secs + 1.5);
        for &(at, radius, secs, born) in &c.frosts {
            let s = now.duration_since(born).as_secs_f64();
            let grow = (s * 24.0).min(radius);
            let ground = add(at, [0.0, 0.2, 0.0]);
            let fade = (1.0 - (s - f64::from(secs)).max(0.0) / 1.5).clamp(0.0, 1.0);
            circle(p, ground, grow, argb(0.85 * fade, 0xBFEFFF), 5.0, true);
            circle(p, add(ground, [0.0, 0.4, 0.0]), grow * 0.96, argb(0.4 * fade, 0x7FD0FF), 3.0, true);
            if s < 0.6 {
                circle(p, add(ground, [0.0, 1.0, 0.0]), grow * 1.05, argb(0.6 * (1.0 - s / 0.6), 0xFFFFFF), 8.0, true);
            }
        }
        c.novas.retain(|n| match n.boom {
            Some(b) => now.duration_since(b).as_secs_f32() < 2.0,
            None => now.duration_since(n.born).as_secs_f64() < n.charge + 3.0,
        });
        for n in &c.novas {
            let core = add(n.ground, [0.0, n.rise + 1.0, 0.0]);
            match n.boom {
                None => {
                    let s = now.duration_since(n.born).as_secs_f64();
                    let k = (s / n.charge).min(1.0);
                    // Rings closing in on the caster, turning, and a white-hot core.
                    for j in 0..3 {
                        let rr = (4.0 - 3.0 * k) * (1.0 + 0.25 * f64::from(j));
                        let spin = s * (2.0 + f64::from(j)) + f64::from(j);
                        let tilt = [spin.cos() * 0.6, 1.0, spin.sin() * 0.6];
                        let m = 28;
                        for i in 0..m {
                            let a0 = i as f64 / m as f64 * std::f64::consts::TAU;
                            let a1 = (i + 1) as f64 / m as f64 * std::f64::consts::TAU;
                            let pt = |a: f64| {
                                add(core, [rr * a.cos(), rr * a.sin() * tilt[0] * 0.8, rr * a.sin() * tilt[2].abs().max(0.4)])
                            };
                            let _ = p.line(pt(a0), pt(a1), argb(0.5 + 0.5 * k, if j == 1 { 0xFFD27A } else { 0xFFF6DC }), 3.0);
                        }
                    }
                    circle(p, add(n.ground, [0.0, 0.15, 0.0]), n.radius * (0.3 + 0.7 * k), argb(0.6, 0xFFC860), 3.0, false);
                    let star = add(core, [0.0, 1.3, 0.0]);
                    cube_prim(p, star, 0.08 + 0.17 * k, argb(0.95, 0xFFFFF4));
                    cube_prim(p, star, 0.16 + 0.3 * k, argb(0.4, 0xFFD27A));
                }
                Some(b) => {
                    let s = now.duration_since(b).as_secs_f64();
                    let fade = (1.0 - s / 0.35).max(0.0);
                    if fade > 0.0 {
                        cube_prim(p, add(core, [0.0, 1.3, 0.0]), 0.6 + 2.0 * s, argb(0.7 * fade, 0xFFFFF0));
                    }
                    let wave = (s / 0.9).min(1.0);
                    if wave < 1.0 {
                        let rr = n.radius * 0.3 + n.radius * 2.0 * wave;
                        circle(p, add(n.ground, [0.0, 0.5, 0.0]), rr, argb(0.95 * (1.0 - wave), 0xFFFFFF), 7.0, true);
                        circle(p, add(n.ground, [0.0, 1.5, 0.0]), rr * 0.95, argb(0.6 * (1.0 - wave), 0xFF9A30), 4.0, true);
                    }
                }
            }
        }
        c.fireballs.retain(|f| now.duration_since(f.born).as_secs_f32() < f.life + 0.5);
        for f in &c.fireballs {
            let secs = now.duration_since(f.born).as_secs_f64();
            let at = fireball_at(f, secs);
            let pulse = 1.0 + 0.12 * (secs * 30.0).sin();
            // A small white-hot core in a soft glow of rays turning every way (not a stack of cubes).
            cube_prim(p, at, 0.11 * pulse, 0xFF_FFF8D0);
            cube_prim(p, at, 0.2 * pulse, 0x90_FFC050);
            let spin = secs * 9.0;
            for j in 0..12 {
                let a = f64::from(j) * 2.399 + spin;
                let up = (f64::from(j) / 11.0) * 2.0 - 1.0;
                let ring = (1.0 - up * up).sqrt();
                let d = [ring * a.cos(), up, ring * a.sin()];
                let l = (0.3 + 0.12 * (secs * 23.0 + f64::from(j)).sin()) * pulse;
                let _ = p.line(add(at, mul(d, 0.08)), add(at, mul(d, l)), argb(0.75, if j % 3 == 0 { 0xFFE8A0 } else { 0xFF8A20 }), 3.5);
            }
            // The trail: a ribbon thinning out behind, and two sparks winding round it.
            let dir = {
                let v = [f.vel[0], f.vel[1] - f.gravity * secs * 20.0, f.vel[2]];
                mul(v, 1.0 / len(v).max(1e-6))
            };
            let side = {
                let s = [-dir[2], 0.0, dir[0]];
                let l = len(s);
                if l < 1e-3 { [1.0, 0.0, 0.0] } else { mul(s, 1.0 / l) }
            };
            let up = [
                side[1] * dir[2] - side[2] * dir[1],
                side[2] * dir[0] - side[0] * dir[2],
                side[0] * dir[1] - side[1] * dir[0],
            ];
            let mut prev = at;
            let mut prev_helix: [Option<[f64; 3]>; 2] = [None, None];
            for k in 1..14 {
                let tk = (secs - f64::from(k) * 0.018).max(0.0);
                let back = fireball_at(f, tk);
                let fade = 1.0 - f64::from(k) / 14.0;
                let color = if k < 4 { 0xFFB040 } else if k < 9 { 0xFF6A18 } else { 0xA02A10 };
                let _ = p.line(prev, back, argb(0.7 * fade, color), (9.0 * fade + 1.5) as f32);
                prev = back;
                for (h, ph) in prev_helix.iter_mut().enumerate() {
                    let ang = tk * 26.0 + h as f64 * std::f64::consts::PI;
                    let rr = 0.22 * fade + 0.05;
                    let q = add(back, add(mul(side, rr * ang.cos()), mul(up, rr * ang.sin())));
                    if let Some(last) = *ph {
                        let _ = p.line(last, q, argb(0.85 * fade, 0xFFE0A0), 2.0);
                    }
                    *ph = Some(q);
                }
            }
        }
        c.meteors.retain(|m| match m.impact {
            Some(t) => now.duration_since(t).as_secs_f32() < 4.0,
            None => now.duration_since(m.born).as_secs_f32() < 30.0,
        });
        for m in &c.meteors {
            let ground = add(m.target, [0.0, 0.15, 0.0]);
            match m.impact {
                None => {
                    let secs = now.duration_since(m.born).as_secs_f64();
                    let u = (secs * 20.0 / m.fall).min(1.0);
                    // The aim: a red ring on the ground, a smaller one pulsing, a cross and a beam.
                    let blink = 0.55 + 0.45 * (secs * 10.0).sin().abs();
                    circle(p, ground, m.radius, argb(blink, 0xFF2020), 4.0, false);
                    circle(p, ground, m.radius, argb(0.25, 0xFF2020), 2.0, true);
                    circle(p, ground, m.radius * (0.35 + 0.15 * u), argb(0.9, 0xFF5030), 3.0, false);
                    for a in [0.0f64, 1.0, 2.0, 3.0] {
                        let ang = a * std::f64::consts::FRAC_PI_2 + secs * 0.8;
                        let (s, co) = ang.sin_cos();
                        let _ = p.line(
                            add(ground, [co * m.radius * 0.15, 0.0, s * m.radius * 0.15]),
                            add(ground, [co * m.radius * 0.95, 0.0, s * m.radius * 0.95]),
                            argb(0.7 * blink, 0xFF3020),
                            2.0,
                        );
                    }
                    let _ = p.beam(ground, 256, argb(0.6, 0xFF3020));
                    // The rock, glowing in layers, and its burning trail.
                    let at = rock_at(m, u);
                    let s = m.size * 0.5;
                    let flick = 1.0 + 0.06 * (secs * 40.0).sin();
                    cube_prim(p, at, s * 0.55, 0xFF_FFF0B8);
                    cube_prim(p, at, s * 0.75 * flick, 0xD0_FF8A20);
                    cube_prim(p, at, s * 1.0 * flick, 0x70_FF4A08);
                    cube_prim(p, at, s * 1.35, 0x30_FF2A00);
                    let path = sub(m.start, m.target);
                    let dir = mul(path, 1.0 / len(path).max(1e-3));
                    for k in 1..18 {
                        let kk = f64::from(k);
                        let back = add(at, mul(dir, kk * s * 0.9));
                        let fade = 1.0 - kk / 18.0;
                        let color = if k < 5 {
                            argb(0.6 * fade, 0xFF7A18)
                        } else if k < 10 {
                            argb(0.45 * fade, 0xB02A08)
                        } else {
                            argb(0.4 * fade, 0x303030)
                        };
                        cube_prim(p, back, s * (0.55 + 0.5 * fade), color);
                    }
                }
                Some(t) => {
                    let secs = now.duration_since(t).as_secs_f64();
                    // The fireball where it hit, sinking into the crater, and the shockwave's ring.
                    let fade = (1.0 - secs / 3.0).max(0.0);
                    if fade > 0.0 {
                        let s = m.radius * 0.25 * (0.6 + 0.6 * secs.min(1.0));
                        cube_prim(p, add(m.target, [0.0, s * 0.3, 0.0]), s * 0.6, argb(0.8 * fade, 0xFFE8A0));
                        cube_prim(p, add(m.target, [0.0, s * 0.3, 0.0]), s, argb(0.45 * fade, 0xFF6A10));
                    }
                    let wave = (secs / 1.6).min(1.0);
                    if wave < 1.0 {
                        let rr = m.radius * 0.5 + m.radius * 3.0 * wave;
                        circle(p, add(ground, [0.0, 0.6, 0.0]), rr, argb(0.9 * (1.0 - wave), 0xFFF0D0), 6.0, true);
                        circle(p, add(ground, [0.0, 1.6, 0.0]), rr * 0.97, argb(0.5 * (1.0 - wave), 0xFFA040), 4.0, true);
                    }
                }
            }
        }
    });
}

/// The living mob (not a player) nearest the aim within `cone` degrees and `range` blocks, in sight: the
/// middle of its body (the client's copy of the server's `aim_target`).
fn client_aim_target(game: &Game<'_>, eye: [f64; 3], dir: [f64; 3], range: f64, cone: f64) -> Option<[f64; 3]> {
    if cone <= 0.0 {
        return None;
    }
    let cos_max = cone.to_radians().cos();
    let mut best: Option<([f64; 3], f64)> = None;
    for m in &game.mobs {
        if !m.alive() || &*m.kind.id == "minecraft:player" {
            continue;
        }
        let c = [m.position.x, m.position.y + 0.9, m.position.z];
        let to = sub(c, eye);
        let d = len(to);
        if !(1.5..=range).contains(&d) {
            continue;
        }
        let cos = (to[0] * dir[0] + to[1] * dir[1] + to[2] * dir[2]) / d;
        if cos < cos_max || ray(game, eye, mul(to, 1.0 / d), d - 0.8).is_some() {
            continue;
        }
        let score = (1.0 - cos) * 40.0 + d * 0.01;
        if best.is_none_or(|b| score < b.1) {
            best = Some((c, score));
        }
    }
    best.map(|b| b.0)
}

/// Brackets round a mob the spell has locked on: a turning ring at its middle, four ticks pointing in.
fn lock_marker(p: &mut Prims, c: [f64; 3], spin: f64, color: u32) {
    flat_ring(p, add(c, [0.0, -0.85, 0.0]), 0.75, spin, argb(0.85, color), 3.0, true);
    for j in 0..4 {
        let ang = f64::from(j) * std::f64::consts::FRAC_PI_2 + spin;
        let (s, co) = ang.sin_cos();
        let _ = p.line(add(c, [co * 1.1, 0.0, s * 1.1]), add(c, [co * 0.7, 0.0, s * 0.7]), argb(0.9, color), 3.0);
    }
}

/// The ground under `at` (up to `down` blocks): the top of the first solid block below.
fn ground_under(game: &Game<'_>, at: [f64; 3], down: i32) -> Option<[f64; 3]> {
    let (x, z) = (at[0].floor() as i32, at[2].floor() as i32);
    let top = at[1].floor() as i32 + 1;
    (0..=down + 1)
        .map(|dy| top - dy)
        .find(|&y| solid(game, x, y - 1, z) && !solid(game, x, y, z))
        .map(|y| [at[0], f64::from(y), at[2]])
}

/// What a gathering spell will hit, drawn for its caster (`gather`: 0..1 of the way gathered): the fireball's
/// path and blast (and the mob it locked on), the step's landing, the meteor's ring, the ice's reach, the
/// bolt's first mob, the supernova's blast.
#[allow(clippy::too_many_arguments)]
fn aim_preview(game: &Game<'_>, p: &mut Prims, t: &Tune, feet: [f64; 3], yaw: f32, pitch: f32, spell: u8, gather: f64) {
    let (yr, pr) = (f64::from(yaw).to_radians(), f64::from(pitch).to_radians());
    let dir = [-yr.sin() * pr.cos(), -pr.sin(), yr.cos() * pr.cos()];
    let eye = add(feet, [0.0, 1.62, 0.0]);
    let (main, glyph) = spell_colors(spell);
    let ease = gather * gather * (3.0 - 2.0 * gather);
    let pulse = 0.65 + 0.35 * (gather * 14.0).sin().abs();
    let spin = gather * 4.0;
    match spell {
        SPELL_FIRE => {
            let lock = client_aim_target(game, eye, dir, 48.0, f64::from(t.fire_assist));
            let aim = match lock {
                Some(c) => {
                    let to = sub(c, eye);
                    mul(to, 1.0 / len(to).max(1e-3))
                }
                None => dir,
            };
            // The path as the server flies it (quarter ticks), dashed, fading with distance.
            let mut pos = add(eye, mul(aim, 0.9));
            let mut vel = mul(aim, f64::from(t.fire_speed));
            let mut last = pos;
            let steps = ((t.fire_life * 80.0) as usize).min(480);
            let mut hit = None;
            for i in 1..=steps {
                pos = add(pos, mul(vel, 0.25));
                vel[1] -= f64::from(t.fire_gravity) * 0.25;
                if solid_at(game, pos) {
                    hit = Some(last);
                    break;
                }
                if lock.is_some_and(|c| dist(c, pos) < 1.0) {
                    hit = Some(pos);
                    break;
                }
                if i % 3 == 0 {
                    let fade = 1.0 - i as f64 / steps as f64;
                    if (i / 3) % 2 == 0 {
                        let _ = p.line(last, pos, argb(0.75 * fade * ease, 0xFFB050), 2.5);
                    }
                    last = pos;
                }
            }
            if let Some(h) = hit {
                let rr = f64::from(t.fire_radius) * (0.4 + 0.6 * ease);
                flat_ring(p, add(h, [0.0, 0.05, 0.0]), rr, spin, argb(0.8 * pulse, 0xFF8030), 3.0, false);
                flat_ring(p, add(h, [0.0, 0.05, 0.0]), rr * 0.4, -spin, argb(0.6, 0xFFE0A0), 2.0, true);
            }
            if let Some(c) = lock {
                lock_marker(p, c, spin, 0xFF6020);
            }
        }
        SPELL_STEP => {
            let max = f64::from(t.step_max.max(t.step_min));
            let far = ray(game, eye, dir, max).map_or(max, |(_, d)| (d - 0.3).max(0.0));
            if far < f64::from(t.step_min) {
                return;
            }
            let e = add(eye, mul(dir, far));
            let land = ground_under(game, [e[0], e[1] - 1.62, e[2]], 4).unwrap_or([e[0], e[1] - 1.62, e[2]]);
            flat_ring(p, add(land, [0.0, 0.05, 0.0]), 0.55 * (0.5 + 0.5 * ease), spin, argb(0.85 * pulse, glyph), 3.0, false);
            // The shape standing there: a dashed column the caster's height.
            for j in 0..6 {
                let y0 = f64::from(j) * 0.3;
                if j % 2 == 0 {
                    let _ = p.line(add(land, [0.0, y0, 0.0]), add(land, [0.0, y0 + 0.3, 0.0]), argb(0.7 * ease, main), 4.0);
                }
            }
        }
        SPELL_METEOR => {
            let range = f64::from(t.meteor_range);
            let Some((hit, _)) = ray_by(game, eye, dir, range, true) else {
                return;
            };
            let g = ground_under(game, hit, 8).unwrap_or(hit);
            let rr = f64::from(t.meteor_radius);
            circle(p, add(g, [0.0, 0.2, 0.0]), rr * (0.25 + 0.75 * ease), argb(0.7 * pulse, 0xFF3020), 3.0, false);
            circle(p, add(g, [0.0, 0.2, 0.0]), rr * 0.12, argb(0.8, 0xFF8040), 3.0, false);
        }
        SPELL_FROST => {
            let rr = f64::from(t.frost_radius) * ease;
            circle(p, add(feet, [0.0, 0.15, 0.0]), rr, argb(0.6 * pulse, 0xBFEFFF), 3.0, false);
        }
        SPELL_CHAIN => {
            let range = f64::from(t.chain_range);
            let mut best: Option<([f64; 3], f64)> = None;
            for m in &game.mobs {
                if !m.alive() || &*m.kind.id == "minecraft:player" {
                    continue;
                }
                let c = [m.position.x, m.position.y + 0.9, m.position.z];
                let to = sub(c, eye);
                let along = to[0] * dir[0] + to[1] * dir[1] + to[2] * dir[2];
                if len(to) > range || along <= 0.5 {
                    continue;
                }
                let off = len(sub(to, mul(dir, along)));
                if off > 1.5 + along * 0.12 {
                    continue;
                }
                let score = off + along * 0.03;
                if best.is_none_or(|b| score < b.1) {
                    best = Some((c, score));
                }
            }
            if let Some((c, _)) = best {
                lock_marker(p, c, spin, 0xE8E060);
            }
        }
        _ => {
            let rr = f64::from(t.nova_radius) * ease;
            circle(p, add(feet, [0.0, 0.15, 0.0]), rr, argb(0.6 * pulse, 0xFFC860), 3.0, false);
        }
    }
}

/// `native_visuals`: coloured lights (fireballs, the rock, the impact, shadow steps, staff crystals).
pub fn visuals(game: &Game<'_>, lights: &mut Vec<horizon_client::mod_api::NativeColoredLight>) {
    let Some(me) = game.native_player().map(|p| [p.x, p.y, p.z]) else {
        return;
    };
    let now = Instant::now();
    let mut found: Vec<([f64; 3], u32, f32, f32)> = Vec::new();
    client(|c| {
        for f in &c.fireballs {
            found.push((fireball_at(f, now.duration_since(f.born).as_secs_f64()), 0xFF8A30, 9.0, 1.0));
        }
        for m in &c.meteors {
            match m.impact {
                None => {
                    let u = (now.duration_since(m.born).as_secs_f64() * 20.0 / m.fall).min(1.0);
                    found.push((rock_at(m, u), 0xFF6A20, 16.0, 1.0));
                }
                Some(t) => {
                    let fade = 1.0 - now.duration_since(t).as_secs_f32() / 4.0;
                    if fade > 0.0 {
                        found.push((add(m.target, [0.0, 3.0, 0.0]), 0xFF7A30, 16.0, fade));
                    }
                }
            }
        }
        for &(at, _, secs, born) in &c.frosts {
            let fade = 1.0 - now.duration_since(born).as_secs_f32() / (secs + 1.5);
            if fade > 0.0 {
                found.push((add(at, [0.0, 1.0, 0.0]), 0x8FDFFF, 12.0, fade));
            }
        }
        for n in &c.novas {
            if n.boom.is_none() {
                let k = (now.duration_since(n.born).as_secs_f64() / n.charge).min(1.0) as f32;
                found.push((add(n.ground, [0.0, n.rise + 1.0, 0.0]), 0xFFE0A0, 6.0 + 10.0 * k, 0.6 + 0.4 * k));
            }
        }
        for (points, at) in &c.arcs {
            let fade = 1.0 - now.duration_since(*at).as_secs_f32() / 0.45;
            for q in points.iter().skip(1) {
                found.push((*q, 0x9FC8FF, 7.0, fade.max(0.0)));
            }
        }
        c.glows.retain(|g| now.duration_since(g.born).as_secs_f32() < g.life);
        for g in &c.glows {
            let fade = 1.0 - now.duration_since(g.born).as_secs_f32() / g.life;
            found.push((g.at, g.rgb, g.radius, fade));
        }
        c.crystals.retain(|_, (_, at)| now.duration_since(*at) < Duration::from_millis(300));
        for (key, (at, _)) in &c.crystals {
            let pulse = 0.75 + 0.25 * (now.duration_since(c.got.unwrap_or(now)).as_secs_f32() * 3.0).sin().abs();
            // Brighter and wider for a moment when the staff casts.
            let burst = c
                .casts
                .iter()
                .filter(|k| k.key == *key)
                .map(|k| (1.0 - now.duration_since(k.born).as_secs_f32() / 0.5).max(0.0))
                .fold(0.0f32, f32::max);
            found.push((*at, 0xB070FF, 4.5 + 7.0 * burst, (pulse + burst).min(1.0)));
        }
        for k in &c.casts {
            let s = now.duration_since(k.born).as_secs_f32();
            let fade = (1.0 - s / (k.secs + 0.6)).max(0.0);
            if fade > 0.0 {
                found.push((add(k.at, [0.0, 0.4, 0.0]), spell_colors(k.spell).0, 6.0, 0.7 * fade));
            }
        }
    });
    found.retain(|(at, ..)| dist(*at, me) < 31.0);
    found.sort_by(|a, b| dist(a.0, me).total_cmp(&dist(b.0, me)));
    for (at, rgb, radius, intensity) in found.into_iter().take(8) {
        lights.push(horizon_client::mod_api::NativeColoredLight {
            at,
            rgb,
            radius: radius.clamp(0.25, 16.0),
            intensity: intensity.clamp(0.0, 1.0),
        });
    }
}

/// `native_camera_shake`.
pub fn camera_shake(shake: &mut super::native_hooks::NativeShake) {
    let now = Instant::now();
    let mut amount = 0.0f32;
    let mut phase = 0.0f32;
    let mut kick = 0.0f32;
    client(|c| {
        c.shakes.retain(|(_, at)| now.duration_since(*at).as_secs_f32() < 4.0);
        for (strength, at) in &c.shakes {
            let s = now.duration_since(*at).as_secs_f32();
            amount += strength * (-s * 1.4).exp();
            phase = phase.max(s);
        }
        // This player's own cast: the view kicks up and settles back, with a small tremble.
        if let Some((strength, at)) = c.recoil {
            let s = now.duration_since(at).as_secs_f32();
            if s > 1.2 {
                c.recoil = None;
            } else {
                kick = strength * (1.0 - (-s * 35.0).exp()) * (-s * 6.5).exp();
            }
        }
    });
    if kick > 0.0001 {
        shake.pitch -= kick * 3.2;
        shake.roll += kick * 0.8;
        shake.offset[1] += kick * 0.04;
    }
    if amount <= 0.001 {
        return;
    }
    let a = amount.min(3.0);
    let s = phase * 40.0;
    shake.pitch += a * 2.5 * (s * 1.3).sin();
    shake.yaw += a * 2.0 * (s * 1.7 + 1.0).sin();
    shake.roll += a * 1.5 * (s * 0.9 + 2.0).sin();
    shake.offset[1] += a * 0.12 * (s * 2.1).sin();
}

/// How thick the meteor's dust is now, 0..1.
fn dust_now(t: &Tune) -> f32 {
    let now = Instant::now();
    client(|c| {
        let Some((strength, at)) = c.dust else { return 0.0 };
        let s = now.duration_since(at).as_secs_f32();
        let total = t.dust_seconds.max(1.0);
        if s > total {
            c.dust = None;
            return 0.0;
        }
        let rise = (s / 2.5).min(1.0);
        let fall = ((total - s) / (total * 0.35)).min(1.0);
        strength * rise * fall
    })
}

const DUST_RGB: [f32; 3] = [0.32, 0.2, 0.12];

/// `native_compute_fog_color`: the sky and fog turn brown-orange in the meteor's dust.
pub fn fog_color(fog_type: horizon_client::fog::FogType, rgb: &mut [f32; 3]) {
    if fog_type != horizon_client::fog::FogType::Atmospheric {
        return;
    }
    let k = dust_now(&tune()) * 0.92;
    if k <= 0.0 {
        return;
    }
    for i in 0..3 {
        rgb[i] += (DUST_RGB[i] - rgb[i]) * k;
    }
}

/// `native_render_fog`: in the dust the world fades out close by.
pub fn render_fog(fog_type: horizon_client::fog::FogType, fog: &mut horizon_client::fog::FogData) {
    if fog_type != horizon_client::fog::FogType::Atmospheric {
        return;
    }
    let t = tune();
    let k = dust_now(&t);
    if k <= 0.0 {
        return;
    }
    for i in 0..3 {
        fog.color[i] += (DUST_RGB[i] - fog.color[i]) * k * 0.92;
    }
    let end = t.dust_fog;
    let toward = |v: f32, to: f32| v + (to.min(v) - v) * k;
    fog.environmental_start = toward(fog.environmental_start, end * 0.1);
    fog.environmental_end = toward(fog.environmental_end, end);
    fog.render_distance_start = toward(fog.render_distance_start, end * 0.1);
    fog.render_distance_end = toward(fog.render_distance_end, end);
    fog.sky_end = toward(fog.sky_end, end * 1.5);
    fog.cloud_end = toward(fog.cloud_end, end * 2.0);
}

/// `native_hud`: the mana bar, the spells' cooldowns, the flash.
pub fn hud(game: &Game<'_>, batch: &mut horizon_client::hud::HudBatch) {
    let t = tune();
    let now = Instant::now();
    let (w, h) = (batch.width, batch.height);
    // The flash covers the whole screen, under the bar.
    let flash = client(|c| {
        let (strength, at) = c.flash?;
        let s = now.duration_since(at).as_secs_f32();
        if s > 1.6 {
            c.flash = None;
            return None;
        }
        let k = if s < 0.08 { s / 0.08 } else { (1.0 - (s - 0.08) / 1.52).powi(2) };
        Some(strength * k)
    });
    if let Some(k) = flash
        && k > 0.0
    {
        let a = (k.min(1.0) * 235.0) as u32;
        game.native_fill(batch, [0.0, 0.0, w, h], a << 24 | 0xFFF2DA);
    }
    if !t.hud {
        return;
    }
    let state = client(|c| {
        c.have.then(|| {
            let since = c.got.map_or(0.0, |g| now.duration_since(g).as_secs_f32());
            let cds = c.cds.map(|v| (v - since).max(0.0));
            // This player's own spell gathering now: which, how far (0..1), its level.
            let gathering = c.casts.iter().rev().find(|k| k.key == u32::MAX && k.released.is_none() && !k.fizzled).and_then(|k| {
                let s = now.duration_since(k.born).as_secs_f32();
                (k.windup > 0.0 && s < k.windup).then(|| (k.spell, s / k.windup, k.level))
            });
            (c.mana, c.max, cds, c.costs, c.fail, c.totals, c.need_staff && c.gear & GEAR_MAGIC == 0, c.mastery, gathering, c.gear & GEAR_MAGIC != 0)
        })
    });
    let Some((mana, max, cds, costs, fail, totals, no_staff, mastery, gathering, magic)) = state else {
        return;
    };
    // No magic item in a hand: no spells and no mana bar (the mana itself stays as it is).
    if !magic {
        return;
    }
    const NAMES: [&str; SPELLS] = ["Fireball", "Shadow step", "Meteor", "Absolute zero", "Chain lightning", "Supernova"];
    // The spell gathering: its name and level over a bar filling under the crosshair.
    if let Some((spell, q, level)) = gathering {
        let i = usize::from(spell.clamp(1, SPELLS as u8)) - 1;
        let (main, glyph) = spell_colors(spell);
        let bw = 92.0f32;
        let (bx, by) = (w / 2.0 - bw / 2.0, h / 2.0 + 16.0);
        game.native_fill(batch, [bx - 1.0, by - 1.0, bw + 2.0, 5.0], 0xB0_000000);
        game.native_fill(batch, [bx, by, bw * q.clamp(0.0, 1.0), 3.0], 0xFF00_0000 | main);
        game.native_fill(batch, [bx, by, bw * q.clamp(0.0, 1.0), 1.0], 0xFF00_0000 | glyph);
        let label = format!("{} · lv {level}", NAMES[i]);
        let lw = game.native_text_width(&label);
        game.native_text(batch, &label, [w / 2.0 - lw / 2.0, by + 6.0], glyph, true);
    }
    let keys = settings(|s| s.keys.clone());
    let x0 = (w / 2.0 - 91.0 - 8.0 - 104.0).max(4.0);
    let chip = 15.0f32;
    let gap = 17.8f32;
    let y_bar = h - 9.0;
    // The bar.
    game.native_fill(batch, [x0 - 1.0, y_bar - 1.0, 109.0, 7.0], 0xA0_000000);
    let fill = (mana / max.max(1.0)).clamp(0.0, 1.0) * 107.0;
    game.native_fill(batch, [x0, y_bar, fill, 5.0], 0xFF_2F5BEA);
    game.native_fill(batch, [x0, y_bar, fill, 2.0], 0xFF_7FA8FF);
    let text = format!("{} / {}", mana.floor() as i32, max.round() as i32);
    let tw = game.native_text_width(&text);
    game.native_text(batch, &text, [x0 + 107.0 - tw, y_bar - 35.0], 0xA8C8FF, true);
    // The spells: a square each, its key, dark while it recharges, red without the mana.
    let colors: [u32; SPELLS] = [0xFF_D2541E, 0xFF_6A3FC0, 0xFF_B01818, 0xFF_3FA8D8, 0xFF_E0D040, 0xFF_F0E8C8];
    let totals = totals.unwrap_or_else(|| cooldowns(&t));
    for i in 0..SPELLS {
        let x = x0 + i as f32 * gap;
        let y = y_bar - 17.0;
        game.native_fill(batch, [x, y, chip, chip], 0xC0_000000);
        game.native_fill(batch, [x + 1.0, y + 1.0, chip - 2.0, chip - 2.0], colors[i]);
        let part = (cds[i] / totals[i].max(0.05)).clamp(0.0, 1.0);
        if part > 0.0 {
            game.native_fill(batch, [x + 1.0, y + 1.0, chip - 2.0, (chip - 2.0) * part], 0xB8_000000);
        }
        if mana < costs[i] {
            game.native_fill(batch, [x + 1.0, y + 1.0, chip - 2.0, chip - 2.0], 0x70_FF0000);
        }
        // No staff in a hand: every spell is out of reach, greyed.
        if no_staff {
            game.native_fill(batch, [x + 1.0, y + 1.0, chip - 2.0, chip - 2.0], 0xC8_30303A);
        }
        // The one gathering now: a bright frame round it.
        if gathering.is_some_and(|g| usize::from(g.0) == i + 1) {
            let f = 0xFF_FFF0C0;
            game.native_fill(batch, [x - 1.0, y - 1.0, chip + 2.0, 1.0], f);
            game.native_fill(batch, [x - 1.0, y + chip, chip + 2.0, 1.0], f);
            game.native_fill(batch, [x - 1.0, y, 1.0, chip], f);
            game.native_fill(batch, [x + chip, y, 1.0, chip], f);
        }
        let label = keys[i].to_uppercase();
        let lw = game.native_text_width(&label);
        let ink = match (no_staff, i) {
            (true, _) => 0x8A8A96,
            (false, 5) => 0x403020,
            _ => 0xFFFFFF,
        };
        // Recharging a second or more: the seconds left instead of the key.
        if !no_staff && cds[i] >= 1.0 {
            let secs = format!("{}", cds[i].ceil() as i32);
            let sw = game.native_text_width(&secs);
            game.native_text(batch, &secs, [x + chip / 2.0 - sw / 2.0, y + 4.0], 0xFFD8A0, true);
        } else {
            game.native_text(batch, &label, [x + chip / 2.0 - lw / 2.0, y + 4.0], ink, no_staff || i != 5);
        }
        // Mastery: the level over the chip's corner, the way to the next one under it.
        if let Some(m) = mastery {
            let (level, progress) = m[i];
            game.native_fill(batch, [x, y + chip + 0.5, chip, 1.0], 0x90_000000);
            game.native_fill(batch, [x, y + chip + 0.5, chip * progress.clamp(0.0, 1.0), 1.0], 0xFF_E8C860);
            if level > 1 {
                let lv = format!("{level}");
                let vw = game.native_text_width(&lv);
                game.native_text(batch, &lv, [x + chip - vw * 0.5 - 1.0, y - 7.0], 0xFFE070, true);
            }
        }
    }
    if let Some((spell, why, at)) = fail
        && now.duration_since(at) < Duration::from_millis(1200)
    {
        let name = NAMES.get(usize::from(spell.max(1)) - 1).copied().unwrap_or("");
        let reason = match why {
            1 => "cooling down",
            2 => "not enough mana",
            4 => "needs the staff in hand",
            _ => "no target",
        };
        let line = format!("{name}: {reason}");
        game.native_text(batch, &line, [x0, y_bar - 46.0], 0xFF6060, true);
    }
}

// ===================================================================================================
// The outfit on the player model
// ===================================================================================================

#[derive(Clone, Copy, Default)]
struct Spring {
    x: f32,
    v: f32,
}

impl Spring {
    fn step(&mut self, target: f32, k: f32, c: f32, dt: f32) {
        let a = -k * (self.x - target) - c * self.v;
        self.v += a * dt;
        self.x += self.v * dt;
    }
}

#[derive(Clone, Copy, Default)]
struct Skirt {
    seen: bool,
    t: f32,
    age: u32,
    vy: f64,
    yaw: f32,
    step: i64,
    /// Swing back (+) and forth, swing to the side, the hem flaring up.
    back: Spring,
    side: Spring,
    flare: Spring,
    /// The hat's tip and its charm nodding.
    hat: Spring,
    rng: u64,
}

static SKIRTS: Mutex<Option<HashMap<u32, Skirt>>> = Mutex::new(None);

fn random_pm(rng: &mut u64) -> f32 {
    (rand01(rng) * 2.0 - 1.0) as f32
}

/// The wearer's springs stepped to now: (swing back, swing to the side, flare, the hat's nod), degrees.
fn skirt_pose(m: &horizon_client::mobs::Mob, key: u32, partial: f32, t: &Tune) -> (f32, f32, f32, f32) {
    let s = horizon_client::mobs::anim::state(m, partial);
    let now_t = m.age as f32 + partial;
    let vy = m.position.y - m.previous.y;
    let (dx, dz) = (m.position.x - m.previous.x, m.position.z - m.previous.z);
    let pace = ((dx * dx + dz * dz).sqrt() as f32 / 0.2158).clamp(0.0, 1.6);
    let yaw = m.previous_body_yaw + (m.body_yaw - m.previous_body_yaw) * partial;
    let mut guard = lock(&SKIRTS);
    let all = guard.get_or_insert_with(HashMap::new);
    if all.len() > 256 {
        all.clear();
    }
    let w = all.entry(key).or_default();
    let dt = now_t - w.t;
    let step_now = (s.pos * 0.6662 / std::f32::consts::PI).floor() as i64;
    if !w.seen || !(0.0..=10.0).contains(&dt) {
        *w = Skirt {
            seen: true,
            t: now_t,
            age: m.age,
            vy,
            yaw,
            step: step_now,
            rng: 0x9E37_79B9_7F4A_7C15 ^ (u64::from(key) << 17),
            ..Skirt::default()
        };
    } else {
        if m.age != w.age {
            if m.age.wrapping_sub(w.age) <= 2 {
                let a = ((vy - w.vy) as f32).clamp(-0.8, 0.8);
                w.flare.v += -a * 30.0 * t.skirt_jump;
                w.back.v += -a * 10.0 * t.skirt_jump;
                w.hat.v += -a * 40.0 * t.skirt_jump;
            }
            w.age = m.age;
            w.vy = vy;
        }
        if dt > 0.0 {
            if step_now != w.step {
                let amount = s.speed.clamp(0.0, 1.0) * pace.max(0.3);
                if amount > 0.05 {
                    let foot = if step_now.rem_euclid(2) == 0 { 1.0 } else { -1.0 };
                    let r = random_pm(&mut w.rng);
                    w.side.v += t.skirt_kick * amount * foot * (1.0 + 0.25 * r);
                    let r = random_pm(&mut w.rng);
                    w.back.v += t.skirt_kick * 0.6 * amount * (1.0 + 0.25 * r);
                    w.flare.v += t.skirt_kick * 0.5 * amount;
                    w.hat.v += t.skirt_kick * 1.2 * amount * (1.0 + 0.3 * r);
                }
                w.step = step_now;
            }
            let mut turn = yaw - w.yaw;
            while turn > 180.0 {
                turn -= 360.0;
            }
            while turn < -180.0 {
                turn += 360.0;
            }
            w.side.v -= turn * t.skirt_turn * 0.2;
            w.yaw = yaw;
            let target = t.skirt_trail * pace.min(1.3);
            let steps = (dt / 0.25).ceil().max(1.0);
            let h = dt / steps;
            for _ in 0..steps as u32 {
                w.back.step(target, t.skirt_k, t.skirt_c, h);
                w.side.step(0.0, t.skirt_k, t.skirt_c, h);
                w.flare.step(0.0, t.skirt_k * 1.3, t.skirt_c * 1.2, h);
                w.hat.step(target * 0.6, t.skirt_k * 1.6, t.skirt_c * 0.8, h);
            }
            w.back.x = w.back.x.clamp(-t.skirt_max, t.skirt_max);
            w.side.x = w.side.x.clamp(-t.skirt_max, t.skirt_max);
            w.flare.x = w.flare.x.clamp(-6.0, 12.0);
            w.hat.x = w.hat.x.clamp(-30.0, 30.0);
            w.t = now_t;
        }
    }
    (w.back.x, w.side.x, w.flare.x, w.hat.x)
}

type Poly = horizon_client::mobs::Polygon;
type ModelPart = horizon_client::mobs::Part;
type V3 = [f32; 3];

fn quad(v: [[f32; 3]; 4]) -> Poly {
    let e1 = [v[1][0] - v[0][0], v[1][1] - v[0][1], v[1][2] - v[0][2]];
    let e2 = [v[2][0] - v[0][0], v[2][1] - v[0][1], v[2][2] - v[0][2]];
    let mut n = [
        e1[1] * e2[2] - e1[2] * e2[1],
        e1[2] * e2[0] - e1[0] * e2[2],
        e1[0] * e2[1] - e1[1] * e2[0],
    ];
    let l = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1e-6);
    n = [n[0] / l, n[1] / l, n[2] / l];
    let uv = [[1.0, 0.0], [0.0, 0.0], [0.0, 1.0], [1.0, 1.0]];
    Poly {
        normal: n,
        vertices: std::array::from_fn(|i| [v[i][0], v[i][1], v[i][2], uv[i][0], uv[i][1]]),
        cull_back: false,
    }
}

/// A box from `a` to `b` (model pixels), six faces.
fn cuboid(a: [f32; 3], b: [f32; 3]) -> Vec<Poly> {
    let [x0, y0, z0] = a;
    let [x1, y1, z1] = b;
    vec![
        quad([[x1, y1, z0], [x0, y1, z0], [x0, y1, z1], [x1, y1, z1]]),
        quad([[x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1]]),
        quad([[x0, y0, z0], [x0, y0, z1], [x0, y1, z1], [x0, y1, z0]]),
        quad([[x1, y0, z1], [x1, y0, z0], [x1, y1, z0], [x1, y1, z1]]),
        quad([[x1, y0, z0], [x0, y0, z0], [x0, y1, z0], [x1, y1, z0]]),
        quad([[x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]]),
    ]
}

/// A box from `a` to `b` (x, z) cut into slabs at the heights `ys` (top to bottom), so that `fitted`
/// can bend its sides.
fn stacked(a: [f32; 2], b: [f32; 2], ys: &[f32]) -> Vec<Poly> {
    ys.windows(2)
        .flat_map(|y| cuboid([a[0], y[0], a[1]], [b[0], y[1], b[1]]))
        .collect()
}

/// Pieces built on the game's torso (half-width 4) moved onto a torso `hw(y)` wide at each height:
/// what lies within the torso is scaled with it, what stands out keeps its margin outside it.
fn fitted(mut polys: Vec<Poly>, hw: &dyn Fn(f32) -> f32) -> Vec<Poly> {
    for p in &mut polys {
        let mut moved = false;
        for v in &mut p.vertices {
            let w = hw(v[1]);
            if (w - 4.0).abs() < 1e-4 {
                continue;
            }
            let ax = v[0].abs();
            let nx = if ax <= 4.0 { ax * w / 4.0 } else { w + ax - 4.0 };
            v[0] = nx.copysign(v[0]);
            moved = true;
        }
        if moved {
            let at = |i: usize| [p.vertices[i][0], p.vertices[i][1], p.vertices[i][2]];
            let (a, b, c) = (at(0), at(1), at(2));
            let e1 = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
            let e2 = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
            let n = [e1[1] * e2[2] - e1[2] * e2[1], e1[2] * e2[0] - e1[0] * e2[2], e1[0] * e2[1] - e1[1] * e2[0]];
            let l = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            if l > 1e-6 {
                p.normal = [n[0] / l, n[1] / l, n[2] / l];
            }
        }
    }
    polys
}

/// A quad facing `out` (its corners turned round if they face the other way).
fn quad_out(mut v: [V3; 4], out: V3) -> Poly {
    let q = quad(v);
    if q.normal[0] * out[0] + q.normal[1] * out[1] + q.normal[2] * out[2] >= 0.0 {
        return q;
    }
    v.reverse();
    quad(v)
}

/// The neckline on each breast of the dress (box rows down from its top edge): deep at the inner edge,
/// high at the outer.
const NECK_IN: f32 = 2.6;
const NECK_OUT: f32 = 1.1;

/// The bust inside the dress, on the body bone: kotost_body's breasts (the shape of tools/bust.py, moved by
/// its springs `j`), bare above the neckline (the wearer's skin), the corset's cloth below it hugging their
/// shape, a gold edge along the neckline.
fn dress_bust(j: horizon_client::kotost_body::Jiggle) -> Vec<(ModelPart, Paint)> {
    use horizon_client::kotost_body as kb;
    let (w, h, d) = (kb::BUST_W, kb::BUST_H, kb::BUST_D);
    // The cloth stands a little off the breast, the gold a little off the cloth.
    const G: f32 = 0.1;
    const GG: f32 = 0.17;
    const BAND: [f32; 2] = [0.12, 0.32];
    // u 0 at the inner edge (the cleavage) to 1 at the outer.
    let cut = |u: f32| NECK_OUT + (NECK_IN - NECK_OUT) * (1.0 - u).powf(1.4);
    let boxp = kb::bust_box_pose();
    let mut out = Vec::new();
    for side in 0..2usize {
        let s = if side == 0 { 1.0f32 } else { -1.0 };
        let m = |p: V3| [p[0] * s, p[1], p[2]];
        let face = |v: [V3; 4], o: V3| quad_out(v.map(m), m(o));
        let (mut skin, mut cloth, mut gold) = (Vec::new(), Vec::new(), Vec::new());
        // The front, in four columns so the neckline curves.
        const COLS: usize = 4;
        for k in 0..COLS {
            let (u0, u1) = (k as f32 / COLS as f32, (k + 1) as f32 / COLS as f32);
            let (c0, c1) = (cut(u0), cut(u1));
            let x = |u: f32, g: f32| -g + u * (w + 2.0 * g);
            let front = [0.0, 0.0, -1.0];
            skin.push(face([[x(u0, 0.0), 0.0, -d], [x(u1, 0.0), 0.0, -d], [x(u1, 0.0), c1, -d], [x(u0, 0.0), c0, -d]], front));
            let z = -d - G;
            cloth.push(face([[x(u0, G), c0, z], [x(u1, G), c1, z], [x(u1, G), h + G, z], [x(u0, G), h + G, z]], front));
            let z = -d - GG;
            gold.push(face(
                [[x(u0, GG), c0 - BAND[0], z], [x(u1, GG), c1 - BAND[0], z], [x(u1, GG), c1 + BAND[1], z], [x(u0, GG), c0 + BAND[1], z]],
                front,
            ));
        }
        // The outer side and the inner one (the cleavage).
        for (xs, c, o) in [(w, cut(1.0), 1.0f32), (0.0, cut(0.0), -1.0)] {
            let x = |g: f32| xs + o * g;
            let side_out = [o, 0.0, 0.0];
            skin.push(face([[x(0.0), 0.0, -d], [x(0.0), 0.0, 0.0], [x(0.0), c, 0.0], [x(0.0), c, -d]], side_out));
            cloth.push(face([[x(G), c, -d - G], [x(G), c, 0.0], [x(G), h + G, 0.0], [x(G), h + G, -d - G]], side_out));
            gold.push(face(
                [[x(GG), c - BAND[0], -d - GG], [x(GG), c - BAND[0], 0.0], [x(GG), c + BAND[1], 0.0], [x(GG), c + BAND[1], -d - GG]],
                side_out,
            ));
        }
        // The underside.
        cloth.push(face([[-G, h + G, -d - G], [w + G, h + G, -d - G], [w + G, h + G, 0.0], [-G, h + G, 0.0]], [0.0, 1.0, 0.0]));
        let pose = kb::bust_part_pose(side, j);
        let hold = |polys: Vec<Poly>| part("dress_bust", pose, Vec::new(), vec![part("dress_bust_box", boxp, polys, Vec::new())]);
        out.push((hold(skin), Paint::Skin(0xFFFFFF)));
        out.push((hold(cloth), Paint::Lit(C_CORSET)));
        out.push((hold(gold), Paint::Lit(C_GOLD)));
    }
    out
}

fn part(name: &str, pose: [f32; 9], polygons: Vec<Poly>, children: Vec<ModelPart>) -> ModelPart {
    ModelPart {
        name: name.to_owned(),
        pose,
        polygons,
        children,
    }
}

fn at(x: f32, y: f32, z: f32, rx: f32, ry: f32, rz: f32) -> [f32; 9] {
    [x, y, z, rx.to_radians(), ry.to_radians(), rz.to_radians(), 1.0, 1.0, 1.0]
}

fn solo(polygons: Vec<Poly>) -> ModelPart {
    part("piece", at(0.0, 0.0, 0.0, 0.0, 0.0, 0.0), polygons, Vec::new())
}

// The palette (reference: refs/mage/target_1_main.webp).
const C_DRESS: u32 = 0x3A2258;
const C_LINING: u32 = 0x1A0F28;
const C_CORSET: u32 = 0x221632;
const C_LACE: u32 = 0x0E0A14;
const C_GOLD: u32 = 0xD6AE45;
const C_GOLD_HI: u32 = 0xF2D060;
const C_HAT: u32 = 0x301D4A;
const C_BAND: u32 = 0xB08A30;
const C_BOOT: u32 = 0x1A1520;
const C_SOLE: u32 = 0x0D0A10;
const C_STOCKING: u32 = 0x241A3A;
const C_STOCKING_TOP: u32 = 0x45345F;
const C_WOOD: u32 = 0x2E1D16;
const C_SKIN: u32 = 0xF2D8CC;
const C_HAT_STAR: u32 = 0xB896F0;
const C_PETAL: u32 = 0x8E4CCF;
const C_PETAL_HI: u32 = 0xC9A0F2;

/// How a piece of the outfit is painted: lit cloth of a colour, a glow (full light, unshaded), or the
/// wearer's own skin (one texel of the cheek) tinted, softly shaded so it reads light.
#[derive(Clone, Copy)]
enum Paint {
    Lit(u32),
    Glow(u32),
    Skin(u32),
}

/// A ring of `n` quads round the y axis at `c` (x, z), from the ellipse `a` to the ellipse `b`, each
/// [x radius, z radius, y]; the first corner at `phase` radians. Two ellipses at one height make a flat
/// annulus, at two heights a frustum's side.
fn ring(n: usize, phase: f32, c: [f32; 2], a: [f32; 3], b: [f32; 3]) -> Vec<Poly> {
    let pt = |e: [f32; 3], i: usize| -> V3 {
        let ang = phase + i as f32 / n as f32 * std::f32::consts::TAU;
        [c[0] + e[0] * ang.cos(), e[2], c[1] + e[1] * ang.sin()]
    };
    (0..n).map(|i| quad([pt(a, i), pt(a, i + 1), pt(b, i + 1), pt(b, i)])).collect()
}

/// A flat `n`-sided lid at `c` (x, z), the ellipse `e` [x radius, z radius, y].
fn lid(n: usize, phase: f32, c: [f32; 2], e: [f32; 3]) -> Vec<Poly> {
    let pt = |i: usize| -> V3 {
        let ang = phase + i as f32 / n as f32 * std::f32::consts::TAU;
        [c[0] + e[0] * ang.cos(), e[2], c[1] + e[1] * ang.sin()]
    };
    let mid = [c[0], e[2], c[1]];
    (0..n).map(|i| quad([mid, pt(i), pt(i + 1), pt(i + 1)])).collect()
}

/// A flat strip `w` wide from `p` to `q` (x, z) lying on the hat's brim, a hair over its top.
fn brim_strip(p: [f32; 2], q: [f32; 2], w: f32) -> Poly {
    let d = [q[0] - p[0], q[1] - p[1]];
    let l = (d[0] * d[0] + d[1] * d[1]).sqrt().max(1e-4);
    let s = [-d[1] / l * w * 0.5, d[0] / l * w * 0.5];
    let y = |x: f32, z: f32| HAT_Y0 - HAT_TH + brim_droop((x * x + z * z).sqrt()) - 0.07;
    let v = |x: f32, z: f32| [x, y(x, z), z];
    quad([v(p[0] + s[0], p[1] + s[1]), v(q[0] + s[0], q[1] + s[1]), v(q[0] - s[0], q[1] - s[1]), v(p[0] - s[0], p[1] - s[1])])
}

/// The brim's underside where it meets the crown (head bone space), its thickness, its inner and outer radii.
const HAT_Y0: f32 = -8.1;
const HAT_TH: f32 = 0.6;
const HAT_RI: f32 = 4.4;
const HAT_RO: f32 = 10.0;

/// How far the brim falls at radius `r`: none at the crown, a pixel at the edge.
fn brim_droop(r: f32) -> f32 {
    let u = ((r - HAT_RI) / (HAT_RO - HAT_RI)).clamp(0.0, 1.0);
    1.0 * u * u
}

/// The hat on the head bone, sitting on the head's top: a wide round brim falling to its edge with a lilac
/// star on it, a crown of eight-sided pieces narrowing and bending back to a crooked tip that nods on its
/// spring, a gold band with a buckle, a violet flower on the band, and a crystal charm hanging from the brim.
fn hat_parts(nod: f32, sway: f32, head_x: f32) -> Vec<(ModelPart, Paint)> {
    use std::f32::consts::{PI, TAU};
    const N: usize = 8;
    const NB: usize = 20;
    // Flats facing the front, the back and the sides.
    let ph = PI / 8.0;
    let o = [0.0f32, 0.0];
    let y0 = HAT_Y0;
    let mut felt = Vec::new();
    // The brim: top and underside in three bands, falling toward the edge, and the edge itself.
    let radii = [HAT_RI, 6.2, 8.0, HAT_RO];
    for w in radii.windows(2) {
        let (ra, rb) = (w[0], w[1]);
        let (da, db) = (brim_droop(ra), brim_droop(rb));
        felt.extend(ring(NB, 0.0, o, [ra, ra, y0 - HAT_TH + da], [rb, rb, y0 - HAT_TH + db]));
        felt.extend(ring(NB, 0.0, o, [ra, ra, y0 + da], [rb, rb, y0 + db]));
    }
    let de = brim_droop(HAT_RO);
    felt.extend(ring(NB, 0.0, o, [HAT_RO, HAT_RO, y0 + de], [HAT_RO, HAT_RO, y0 - HAT_TH + de]));
    // The crown: (radius at the top, height, bend back, roll) of each piece, the first one's foot `foot`
    // wide; each piece reaches a little below its foot along its own slope, so a bend opens no gap.
    let segs: [(f32, f32, f32, f32); 8] = [
        (4.55, 2.5, 0.0, 0.0),
        (3.8, 2.2, -4.0, 0.0),
        (3.0, 2.0, -7.0, 2.0),
        (2.25, 1.9, -11.0, 4.0),
        (1.6, 1.8, -16.0, 6.0),
        (1.05, 1.6, -22.0, 8.0),
        (0.6, 1.4, -26.0, 8.0),
        (0.1, 1.2, -24.0, 6.0),
    ];
    let foot = 5.1f32;
    let yc = y0 - HAT_TH + 0.05;
    let n = segs.len() as f32;
    let base_pose = |k: f32| at(0.0, yc, 0.0, nod * 0.35 * k, 0.0, sway * 0.3 * k);
    let mut tip: Option<ModelPart> = None;
    for (i, &(r1, h, bend, roll)) in segs.iter().enumerate().rev() {
        let r0 = if i == 0 { foot } else { segs[i - 1].0 };
        let y = if i == 0 { yc } else { -segs[i - 1].1 };
        let k = (i as f32 + 1.0) / n;
        let ext = 0.7f32;
        let rx = r0 + (r0 - r1) / h * ext;
        let mut polys = ring(N, ph, o, [rx, rx, ext], [r0, r0, 0.0]);
        polys.extend(ring(N, ph, o, [r0, r0, 0.0], [r1, r1, -h]));
        if i + 1 == segs.len() {
            polys.extend(lid(N, ph, o, [r1, r1, -h]));
        }
        let children = tip.take().into_iter().collect();
        tip = Some(part(
            "cone",
            at(0.0, y, 0.0, bend + nod * 0.35 * k, 0.0, roll + sway * 0.3 * k),
            polys,
            children,
        ));
    }
    let mut hat = solo(felt);
    hat.children.extend(tip);
    // The crown's first piece: its radius `t` pixels above its foot; the band, buckle and flower ride on it.
    let (r_top, h0) = (segs[0].0, segs[0].1);
    let crown_r = |t: f32| foot - (foot - r_top) * t / h0;
    let slope = ((foot - r_top) / h0).atan().to_degrees();
    let pose0 = base_pose(1.0 / n);
    let (b0, b1) = (0.3f32, 1.5f32);
    let mut band = ring(N, ph, o, [crown_r(b0) + 0.14, crown_r(b0) + 0.14, -b0], [crown_r(b1) + 0.14, crown_r(b1) + 0.14, -b1]);
    band.extend(ring(N, ph, o, [crown_r(b0), crown_r(b0), -b0], [crown_r(b0) + 0.14, crown_r(b0) + 0.14, -b0]));
    band.extend(ring(N, ph, o, [crown_r(b1), crown_r(b1), -b1], [crown_r(b1) + 0.14, crown_r(b1) + 0.14, -b1]));
    let band = part("band", pose0, band, Vec::new());
    // The buckle on the front flat, leaning back with the crown.
    let mid = (b0 + b1) * 0.5;
    let inr = (crown_r(mid) + 0.14) * (PI / 8.0).cos();
    let mut bk = Vec::new();
    bk.extend(cuboid([-1.35, -0.85, -0.3], [1.35, -0.5, 0.0]));
    bk.extend(cuboid([-1.35, 0.5, -0.3], [1.35, 0.85, 0.0]));
    bk.extend(cuboid([-1.35, -0.85, -0.3], [-0.95, 0.85, 0.0]));
    bk.extend(cuboid([0.95, -0.85, -0.3], [1.35, 0.85, 0.0]));
    bk.extend(cuboid([-0.2, -0.55, -0.4], [0.2, 0.55, -0.1]));
    let buckle = part(
        "buckle_hold",
        pose0,
        Vec::new(),
        vec![part("buckle", at(0.0, -mid, -inr, -slope, 0.0, 0.0), bk, Vec::new())],
    );
    // The flower on the front right diagonal flat: five petals, five smaller lighter ones, a gold heart.
    let fr = (crown_r(mid) + 0.2) * (PI / 8.0).cos();
    let fd = [-(PI / 4.0).sin(), -(PI / 4.0).cos()];
    let flower_at = at(fr * fd[0], -mid, fr * fd[1], -slope, 45.0, 0.0);
    let mut petals = Vec::new();
    let mut inner = Vec::new();
    for j in 0..5 {
        let a = j as f32 * 72.0;
        petals.push(part("petal", at(0.0, 0.0, 0.0, 0.0, 0.0, a), cuboid([-0.5, -1.45, -0.35], [0.5, -0.2, 0.0]), Vec::new()));
        inner.push(part("petal", at(0.0, 0.0, 0.0, 0.0, 0.0, a + 36.0), cuboid([-0.32, -0.95, -0.55], [0.32, -0.15, -0.25]), Vec::new()));
    }
    let flower = part("flower_hold", pose0, Vec::new(), vec![part("flower", flower_at, Vec::new(), petals)]);
    let flower_in = part("flower_hold", pose0, Vec::new(), vec![part("flower", flower_at, Vec::new(), inner)]);
    let heart = part(
        "flower_hold",
        pose0,
        Vec::new(),
        vec![part("flower", flower_at, cuboid([-0.35, -0.35, -0.8], [0.35, 0.35, -0.3]), Vec::new())],
    );
    // The star on the brim: eight points round the crown, a ring inside it.
    let mut star = Vec::new();
    let pts: Vec<[f32; 2]> = (0..16)
        .map(|j| {
            let r = if j % 2 == 0 { 9.1 } else { 6.5 };
            let a = j as f32 / 16.0 * TAU - PI / 2.0;
            [r * a.cos(), r * a.sin()]
        })
        .collect();
    for j in 0..16 {
        star.push(brim_strip(pts[j], pts[(j + 1) % 16], 0.42));
    }
    for j in 0..24 {
        let a0 = j as f32 / 24.0 * TAU;
        let a1 = (j + 1) as f32 / 24.0 * TAU;
        let r = 5.75;
        star.push(brim_strip([r * a0.cos(), r * a0.sin()], [r * a1.cos(), r * a1.sin()], 0.3));
    }
    // A small diamond at each point's tip.
    for j in (0..16).step_by(2) {
        let a = j as f32 / 16.0 * TAU - PI / 2.0;
        let (c, s) = (a.cos(), a.sin());
        star.push(brim_strip([9.35 * c, 9.35 * s], [9.85 * c, 9.85 * s], 0.42));
    }
    // The charm: a gold chain from under the brim, front left, and a crystal; it hangs straight down
    // whatever the head does, and swings with the hat's spring.
    let crystal = part(
        "charm_gem",
        [0.0, 3.7, 0.0, 0.0, 45f32.to_radians(), 0.0, 0.9, 1.6, 0.9],
        cuboid([-0.6, -0.6, -0.6], [0.6, 0.6, 0.6]),
        Vec::new(),
    );
    let mut chain = Vec::new();
    for j in 0..3 {
        let y = 0.3 + j as f32 * 0.95;
        chain.extend(cuboid([-0.18, y, -0.18], [0.18, y + 0.7, 0.18]));
    }
    let (cx, cz) = (6.3f32, -4.3f32);
    let hang = at(cx, y0 + brim_droop((cx * cx + cz * cz).sqrt()) - 0.1, cz, -head_x.to_degrees() - nod * 1.4, 0.0, sway * 1.2);
    let charm = part("charm", hang, chain, Vec::new());
    let gem = part("charm_hold", hang, Vec::new(), vec![crystal]);
    vec![
        (hat, Paint::Lit(C_HAT)),
        (solo(star), Paint::Lit(C_HAT_STAR)),
        (band, Paint::Lit(C_BAND)),
        (buckle, Paint::Lit(C_GOLD_HI)),
        (flower, Paint::Lit(C_PETAL)),
        (flower_in, Paint::Lit(C_PETAL_HI)),
        (heart, Paint::Lit(C_GOLD_HI)),
        (charm, Paint::Lit(C_GOLD)),
        (gem, Paint::Glow(0xC890FF)),
    ]
}

/// A leg as the skirt sees it, in the body bone's space: where its bone pivots, the way it points (down
/// the leg), its half-thickness.
#[derive(Clone, Copy)]
struct Leg {
    p0: V3,
    d: V3,
    r: f32,
}

fn mat3_zyx(r: [f32; 3]) -> glam::Mat3 {
    glam::Mat3::from_rotation_z(r[2]) * glam::Mat3::from_rotation_y(r[1]) * glam::Mat3::from_rotation_x(r[0])
}

/// The two legs in the body bone's space, from this frame's poses.
fn legs_in_body(poses: &mut horizon_client::mobs::anim::Poses) -> Vec<Leg> {
    if !poses.has("body") {
        return Vec::new();
    }
    let b = *poses.get("body");
    let rb = mat3_zyx([b[3], b[4], b[5]]).transpose();
    let pb = glam::Vec3::new(b[0], b[1], b[2]);
    let mut out = Vec::new();
    for name in ["right_leg", "left_leg"] {
        if !poses.has(name) || poses.is_hidden_named(name) {
            continue;
        }
        let l = *poses.get(name);
        let p = rb * (glam::Vec3::new(l[0], l[1], l[2]) - pb);
        let d = rb * (mat3_zyx([l[3], l[4], l[5]]) * glam::Vec3::Y);
        out.push(Leg {
            p0: p.to_array(),
            d: d.normalize_or_zero().to_array(),
            r: 2.75,
        });
    }
    out
}

/// How far out from `c` (x, z) along `u` the cloth at height `y` must stand to clear `leg`, if it is in the
/// way: the far side of the leg's cross-section there (an ellipse when the leg swings).
fn clear_leg(leg: &Leg, y: f32, c: [f32; 2], u: [f32; 2]) -> Option<f32> {
    if leg.d[1] < 0.25 {
        return None;
    }
    let s = (y - leg.p0[1]) / leg.d[1];
    if !(-1.0..=13.5).contains(&s) {
        return None;
    }
    let lc = [leg.p0[0] + leg.d[0] * s, leg.p0[2] + leg.d[2] * s];
    let o = [c[0] - lc[0], c[1] - lc[1]];
    let dh = [leg.d[0], leg.d[2]];
    let dl = (dh[0] * dh[0] + dh[1] * dh[1]).sqrt();
    let (e1, e2) = if dl > 1e-4 {
        ([dh[0] / dl, dh[1] / dl], [-dh[1] / dl, dh[0] / dl])
    } else {
        ([1.0, 0.0], [0.0, 1.0])
    };
    let (a1, a2) = ((leg.r / leg.d[1]).min(leg.r * 3.0), leg.r);
    let dot = |a: [f32; 2], b: [f32; 2]| a[0] * b[0] + a[1] * b[1];
    let o2 = [dot(o, e1) / a1, dot(o, e2) / a2];
    let u2 = [dot(u, e1) / a1, dot(u, e2) / a2];
    let a = dot(u2, u2);
    let b = 2.0 * dot(o2, u2);
    let cc = dot(o2, o2) - 1.0;
    let disc = b * b - 4.0 * a * cc;
    if disc <= 0.0 || a < 1e-8 {
        return None;
    }
    let far = (-b + disc.sqrt()) / (2.0 * a);
    (far > 0.0).then_some(far)
}

/// The dress's skirt on the body bone: an A-line from the waist to the ankles with a dark lining, swung by
/// its springs, pushed out where the legs are, open in a slit to the thigh over the leg away from the staff
/// (`slit_left`: the left leg, +x), edged in gold. Returns (cloth, lining, gold).
fn skirt_part(back: f32, side: f32, flare: f32, t: &Tune, legs: &[Leg], slit_left: bool, body_w: &dyn Fn(f32) -> f32) -> (ModelPart, ModelPart, ModelPart) {
    const SEGS: usize = 24;
    const FRACS: [f32; 9] = [0.0, 0.08, 0.17, 0.29, 0.42, 0.56, 0.71, 0.86, 1.0];
    let top_y = 10.2f32;
    let lenp = t.skirt_len;
    let wide = t.skirt_width;
    let fl = flare * 0.03;
    // Wider hips (an hourglass body) widen the skirt's top as much.
    let hip = (body_w(12.0) - 4.0).max(0.0);
    let (sb, cb) = back.to_radians().sin_cos();
    let (ss, cs) = side.to_radians().sin_cos();
    // The slit: over the leg's front (the left leg: +x, -z), from the thigh down, widening to the hem.
    let slit_c = if slit_left { -1.22f32 } else { -(std::f32::consts::PI - 1.22) };
    let f_top = 0.03f32;
    let half = |f: f32| -> f32 {
        if t.slit <= 0.0 || f <= f_top {
            0.0
        } else {
            1.0 * t.slit * ((f - f_top) / (1.0 - f_top)).powf(0.5)
        }
    };
    // Each ring: SEGS + 1 points round from one edge of the slit to the other (closed above it).
    let mut rings: Vec<[V3; SEGS + 1]> = Vec::new();
    let mut centers: Vec<[f32; 2]> = Vec::new();
    for &f in &FRACS {
        let h = f * lenp;
        let rx = 4.6 + hip + 1.65 * f.powf(1.2) * wide * (1.0 + fl * f);
        let rz = 2.72 + 0.5 * hip + 1.85 * f.powf(1.1) * wide * (1.0 + fl * f);
        let k = f.powf(1.3);
        let cx = h * ss * k;
        let cz = h * sb * k;
        let y = top_y + h * (1.0 - (1.0 - cb * cs) * k * 0.5);
        let hw = half(f);
        let a0 = slit_c + hw;
        let span = std::f32::consts::TAU - 2.0 * hw;
        let mut ring = [[0.0f32; 3]; SEGS + 1];
        let mut radii = [0.0f32; SEGS + 1];
        let mut dirs = [[0.0f32; 2]; SEGS + 1];
        for (i, q) in ring.iter_mut().enumerate() {
            let a = a0 + span * i as f32 / SEGS as f32;
            let (x, z) = (rx * a.cos(), rz * a.sin());
            let r = (x * x + z * z).sqrt().max(1e-3);
            let u = [x / r, z / r];
            let mut want = r;
            if t.legs_push > 0.0 {
                // The leg seen fatter by SOFT, then SOFT taken off: the cloth over it is a round tent,
                // not a spike; a far kick stretches it only so far.
                const SOFT: f32 = 1.8;
                for leg in legs {
                    let fat = Leg { r: leg.r + SOFT, ..*leg };
                    if let Some(far) = clear_leg(&fat, y, [cx, cz], u) {
                        let need = (far - SOFT + 0.3).min(r + 4.5);
                        if need > want {
                            want = r + (need - r) * t.legs_push.min(1.5);
                        }
                    }
                }
            }
            // Round the hips (to 12.4 down): never inside the torso's box and its jacket, so their
            // corners don't poke through the skirt's top.
            if y < 12.4 {
                let (bx, bz) = (body_w(y) + 0.38, 2.42);
                let to_box = (bx / u[0].abs().max(1e-3)).min(bz / u[1].abs().max(1e-3));
                want = want.max(to_box + 0.05);
            }
            radii[i] = want;
            dirs[i] = u;
            *q = [cx + u[0] * want, y, cz + u[1] * want];
        }
        // Soften the bulges a leg makes: each point at least near its neighbours' mean, three times.
        let closed = hw <= 0.0;
        for _ in 0..3 {
        let before = radii;
        for i in 0..=SEGS {
            let (l, r) = match (i, closed) {
                (0, true) => (before[SEGS - 1], before[1]),
                (0, false) => (before[0], before[1]),
                (i, _) if i == SEGS => (before[SEGS - 1], if closed { before[1] } else { before[SEGS] }),
                (i, _) => (before[i - 1], before[i + 1]),
            };
            let soft = (l + r) * 0.5 * 0.985;
            if soft > radii[i] {
                radii[i] = soft;
                ring[i] = [cx + dirs[i][0] * soft, y, cz + dirs[i][1] * soft];
            }
        }
        }
        rings.push(ring);
        centers.push([cx, cz]);
    }
    let out_by = |p: V3, c: [f32; 2], k: f32| -> V3 { [c[0] + (p[0] - c[0]) * k, p[1], c[1] + (p[2] - c[1]) * k] };
    let mut cloth = Vec::new();
    let mut lining = Vec::new();
    let mut gold = Vec::new();
    for r in 0..rings.len() - 1 {
        for i in 0..SEGS {
            let (a, b, c, d) = (rings[r][i], rings[r][i + 1], rings[r + 1][i + 1], rings[r + 1][i]);
            cloth.push(quad([a, b, c, d]));
            let (ca, cb2) = (centers[r], centers[r + 1]);
            lining.push(quad([out_by(d, cb2, 0.965), out_by(c, cb2, 0.965), out_by(b, ca, 0.965), out_by(a, ca, 0.965)]));
        }
    }
    let last = rings.len() - 1;
    let lc = centers[last];
    // The hem: a gold band just above the bottom edge, a little outside.
    for i in 0..SEGS {
        let (a, b) = (rings[last][i], rings[last][i + 1]);
        let up = |p: V3, by: f32| [p[0], p[1] - by, p[2]];
        gold.push(quad([
            out_by(up(a, 0.9), lc, 1.02),
            out_by(up(b, 0.9), lc, 1.02),
            out_by(b, lc, 1.025),
            out_by(a, lc, 1.025),
        ]));
    }
    // The slit's edges: gold strips up both sides, from the hem to the top of the slit.
    if t.slit > 0.0 {
        for r in 0..last {
            if half(FRACS[r + 1]) <= 0.0 {
                continue;
            }
            for (e, n) in [(0usize, 1usize), (SEGS, SEGS - 1)] {
                let strip = |ring: &[V3; SEGS + 1], c: [f32; 2]| -> (V3, V3) {
                    let p = ring[e];
                    let q = ring[n];
                    let d = [q[0] - p[0], q[2] - p[2]];
                    let l = (d[0] * d[0] + d[1] * d[1]).sqrt().max(1e-3);
                    let w = 0.75f32.min(l);
                    let inner = [p[0] + d[0] / l * w, p[1], p[2] + d[1] / l * w];
                    (out_by(p, c, 1.02), out_by(inner, c, 1.02))
                };
                let (p0, q0) = strip(&rings[r], centers[r]);
                let (p1, q1) = strip(&rings[r + 1], centers[r + 1]);
                gold.push(quad([p0, q0, q1, p1]));
            }
        }
    }
    // Embroidery: small gold diamonds above the hem, round the back and the left.
    for &i in &[4usize, 8, 12, 16, 20] {
        let (r0, r1) = (last - 1, last);
        let mid = |p: V3, q: V3| [(p[0] + q[0]) * 0.5, (p[1] + q[1]) * 0.5, (p[2] + q[2]) * 0.5];
        let c0 = mid(rings[r0][i], rings[r1][i]);
        let c1 = mid(rings[r0][i + 1], rings[r1][i + 1]);
        let centre = mid(c0, c1);
        let along = [c1[0] - c0[0], 0.0, c1[2] - c0[2]];
        let l = (along[0] * along[0] + along[2] * along[2]).sqrt().max(1e-3);
        let along = [along[0] / l * 0.55, 0.0, along[2] / l * 0.55];
        let cc = [(centers[r0][0] + centers[r1][0]) * 0.5, (centers[r0][1] + centers[r1][1]) * 0.5];
        let o = |p: V3| out_by(p, cc, 1.03);
        let up = [centre[0], centre[1] - 0.8, centre[2]];
        let down = [centre[0], centre[1] + 0.8, centre[2]];
        let left = [centre[0] - along[0], centre[1], centre[2] - along[2]];
        let right = [centre[0] + along[0], centre[1], centre[2] + along[2]];
        gold.push(quad([o(up), o(right), o(down), o(left)]));
    }
    (solo(cloth), solo(lining), solo(gold))
}

/// The bodice on the body bone: bare shoulders and collarbones (the wearer's skin) above a sweetheart
/// neckline edged in gold, a choker with a small violet gem, then the dark corset with gold lacing at the
/// front between gold lines.
fn bodice_parts(hw: &dyn Fn(f32) -> f32) -> Vec<(ModelPart, Paint)> {
    // The skin over the shoulders and the chest down to below the neckline.
    let mut skin = cuboid([-4.3, -0.3, -2.3], [4.3, 3.7, 2.3]);
    // The hips, seen where the slit opens up to the waist (else the skin's own clothes show there).
    skin.extend(stacked([-4.3, -2.3], [4.3, 2.3], &[10.0, 11.0, 12.2]));
    let skin = solo(fitted(skin, hw));
    // Collarbones: two faint lines from the throat out to the shoulders.
    let mut bones = Vec::new();
    for s in [-1.0f32, 1.0] {
        let (x0, y0, x1, y1) = (0.45 * s, 1.2f32, 3.1 * s, 0.75f32);
        let z = -2.33f32;
        bones.push(quad([[x0, y0 - 0.11, z], [x1, y1 - 0.11, z], [x1, y1 + 0.11, z], [x0, y0 + 0.11, z]]));
    }
    // The neckline: the front's top edge dips between the breasts, rises over them, falls to the sides.
    let xs = [-4.38f32, -3.2, -2.0, -1.0, 0.0, 1.0, 2.0, 3.2, 4.38];
    let tops = [2.0f32, 2.15, 2.4, 2.85, 3.35, 2.85, 2.4, 2.15, 2.0];
    let (zf, zb, xw, low) = (-2.38f32, 2.38f32, 4.38f32, 5.6f32);
    let mut cloth = Vec::new();
    let mut trim = Vec::new();
    for i in 0..xs.len() - 1 {
        let (xa, xb, ta, tb) = (xs[i], xs[i + 1], tops[i], tops[i + 1]);
        cloth.push(quad([[xa, ta, zf], [xb, tb, zf], [xb, low, zf], [xa, low, zf]]));
        cloth.push(quad([[xa, ta, zf], [xb, tb, zf], [xb, tb, -2.26], [xa, ta, -2.26]]));
        trim.push(quad([[xa, ta - 0.08, zf - 0.06], [xb, tb - 0.08, zf - 0.06], [xb, tb + 0.42, zf - 0.06], [xa, ta + 0.42, zf - 0.06]]));
    }
    let top = tops[0];
    cloth.push(quad([[-xw, top, zb], [xw, top, zb], [xw, low, zb], [-xw, low, zb]]));
    for s in [-1.0f32, 1.0] {
        cloth.push(quad([[s * xw, top, zf], [s * xw, top, zb], [s * xw, low, zb], [s * xw, low, zf]]));
        cloth.push(quad([[s * xw, top, zf], [s * xw, top, zb], [s * 4.26, top, zb], [s * 4.26, top, zf]]));
    }
    cloth.push(quad([[-xw, top, zb], [xw, top, zb], [xw, top, 2.26], [-xw, top, 2.26]]));
    trim.extend(cuboid([-4.44, top - 0.08, -2.2], [4.44, top + 0.42, 2.44]));
    // The choker, and its gem turned to a diamond.
    let choker = solo(cuboid([-2.35, -0.05, -2.43], [2.35, 0.62, -2.3]));
    let gem = part("choker_gem", at(0.0, 0.95, -2.42, 0.0, 0.0, 45.0), cuboid([-0.33, -0.33, -0.12], [0.33, 0.33, 0.06]), Vec::new());
    // The corset in one-pixel rows, so it follows an hourglass body in to the waist and out to the hips.
    let corset = solo(fitted(stacked([-4.46, -2.46], [4.46, 2.46], &[5.5, 6.5, 7.5, 8.0, 8.5, 9.5, 10.5]), hw));
    let mut gold = cuboid([-4.55, 5.25, -2.55], [4.55, 5.75, 2.55]);
    gold.extend(cuboid([-4.55, 9.55, -2.55], [4.55, 10.05, 2.55]));
    gold.extend(trim);
    let gold = fitted(gold, hw);
    let cloth = fitted(cloth, hw);
    // The lacing: a black panel, gold laces crossing on it.
    let panel = solo(fitted(stacked([-1.15, -2.56], [1.15, -2.47], &[5.8, 6.5, 7.5, 8.5, 9.5]), hw));
    let mut laces = solo(Vec::new());
    for i in 0..4 {
        let y = 6.3 + i as f32 * 0.9;
        for a in [38.0f32, -38.0] {
            laces.children.push(part(
                "lace",
                at(0.0, y, -2.6, 0.0, 0.0, a),
                cuboid([-0.95, -0.1, -0.05], [0.95, 0.1, 0.05]),
                Vec::new(),
            ));
        }
    }
    vec![
        (skin, Paint::Skin(0xFFFFFF)),
        (solo(bones), Paint::Skin(0xD2B4AA)),
        (solo(cloth), Paint::Lit(C_CORSET)),
        (corset, Paint::Lit(C_CORSET)),
        (solo(gold), Paint::Lit(C_GOLD)),
        (panel, Paint::Lit(C_LACE)),
        (laces, Paint::Lit(C_GOLD)),
        (choker, Paint::Lit(C_LACE)),
        (part("choker_hold", at(0.0, 0.0, 0.0, 0.0, 0.0, 0.0), Vec::new(), vec![gem]), Paint::Glow(0xA874F0)),
    ]
}

/// A sleeve on an arm bone (`x0`..`x1` the arm's width, its depth 4): the shoulder left bare (skin), a puff
/// below it edged in gold, a fitted upper sleeve, then a wide bell from the elbow, lined dark inside and
/// edged in gold at the wrist.
fn sleeve_parts(x0: f32, x1: f32) -> Vec<(ModelPart, Paint)> {
    let ph = std::f32::consts::PI / 8.0;
    let c = [(x0 + x1) * 0.5, 0.0];
    let hw = (x1 - x0) * 0.5;
    let e = |dx: f32, rz: f32, y: f32| [hw + dx, rz, y];
    // Inside the arm: what the puff's and the bell's flat tops close down to.
    let core = |y: f32| [hw * 0.9, 1.8, y];
    let skin = solo(cuboid([x0 - 0.3, -2.3, -2.3], [x1 + 0.3, -0.15, 2.3]));
    let mut cloth = Vec::new();
    // The puff: out from under the shoulder, round, back in to the fitted sleeve.
    // (An 8-sided ring of x, z radii rx, rz holds the arm's sleeve layer, hw + 0.25 by 2.25, when
    // (hw + 0.25) / rx + 2.25 / rz < 1.3.)
    let (p0, p1, p2) = (e(1.45, 3.5, -0.2), e(1.6, 3.65, 1.2), e(0.55, 2.6, 2.7));
    cloth.extend(ring(8, ph, c, core(-0.2), p0));
    cloth.extend(ring(8, ph, c, p0, p1));
    cloth.extend(ring(8, ph, c, p1, p2));
    cloth.extend(ring(8, ph, c, p2, core(2.7)));
    cloth.extend(cuboid([x0 - 0.36, 1.4, -2.36], [x1 + 0.36, 5.0, 2.36]));
    // The bell: from the elbow wide to the wrist, open.
    let (b0, b1, b2) = (e(1.4, 3.55, 5.0), e(1.65, 3.75, 7.0), e(2.15, 4.15, 8.7));
    cloth.extend(ring(8, ph, c, core(5.0), b0));
    cloth.extend(ring(8, ph, c, b0, b1));
    cloth.extend(ring(8, ph, c, b1, b2));
    let inside = |q: [f32; 3]| [q[0] * 0.93, q[1] * 0.93, q[2]];
    let mut lining = ring(8, ph, c, inside(b0), inside(b1));
    lining.extend(ring(8, ph, c, inside(b1), inside(b2)));
    // Gold: the puff's top edge and the bell's cuff.
    let mut gold = ring(8, ph, c, e(1.56, 3.6, -0.32), e(1.64, 3.68, 0.3));
    gold.extend(ring(8, ph, c, e(1.4, 3.45, -0.32), e(1.56, 3.6, -0.32)));
    gold.extend(ring(8, ph, c, e(2.06, 4.08, 8.15), e(2.28, 4.28, 8.78)));
    gold.extend(ring(8, ph, c, e(2.04, 4.06, 8.78), e(2.28, 4.28, 8.78)));
    vec![
        (skin, Paint::Skin(0xFFFFFF)),
        (solo(cloth), Paint::Lit(C_DRESS)),
        (solo(lining), Paint::Lit(C_LINING)),
        (solo(gold), Paint::Lit(C_GOLD)),
    ]
}

/// Stockings with garters and boots on a leg bone (`outer` -1 for the right leg, +1 for the left).
/// The thigh above the stocking is the wearer's skin.
fn leg_parts(outer: f32) -> Vec<(ModelPart, Paint)> {
    let boot = solo(cuboid([-2.5, 8.2, -2.5], [2.5, 12.0, 2.5]));
    let mut sole = cuboid([-2.62, 11.15, -2.95], [2.62, 12.0, 2.62]);
    sole.extend(cuboid([-2.6, 9.25, -2.6], [2.6, 9.75, 2.6]));
    let stocking = solo(cuboid([-2.33, 5.2, -2.33], [2.33, 8.3, 2.33]));
    let top = solo(cuboid([-2.43, 4.7, -2.43], [2.43, 5.6, 2.43]));
    let mut straps = cuboid([-0.32, -0.8, -2.42], [0.32, 4.8, -2.33]);
    let xo = outer * 2.375;
    straps.extend(cuboid([xo - 0.045, -0.8, -0.32], [xo + 0.045, 4.8, 0.32]));
    let mut gold = cuboid([-0.45, 4.35, -2.5], [0.45, 4.85, -2.38]);
    gold.extend(cuboid([xo - 0.1, 4.35, -0.45], [xo + 0.1, 4.85, 0.45]));
    // A buckle on the boot's strap, outside.
    let xb = outer * 2.62;
    gold.extend(cuboid([xb - 0.08, 9.15, -0.55], [xb + 0.08, 9.85, 0.55]));
    let thigh = solo(cuboid([-2.29, -0.4, -2.29], [2.29, 4.8, 2.29]));
    vec![
        (boot, Paint::Lit(C_BOOT)),
        (solo(sole), Paint::Lit(C_SOLE)),
        (stocking, Paint::Lit(C_STOCKING)),
        (top, Paint::Lit(C_STOCKING_TOP)),
        (solo(straps), Paint::Lit(C_LACE)),
        (solo(gold), Paint::Lit(C_GOLD)),
        (thigh, Paint::Skin(0xFFFFFF)),
    ]
}

/// The staff in the main hand (arm bone space; `mirror` for a left arm): dark wood ringed in gold, a ring
/// at the head holding the crystal, a gold spike over it. `turn` is the staff's turn in the hand (degrees
/// about x, y, z, the game's order), `flash` 0..1 swells the crystal.
fn staff_parts(mirror: bool, spin: f32, turn: [f32; 3], flash: f32) -> (ModelPart, ModelPart, ModelPart) {
    let sx = if mirror { -1.0 } else { 1.0 };
    let hand = at(-0.5 * sx, 9.5, -1.2, turn[0], turn[1], turn[2]);
    let head_y = -24.4f32;
    let ring_r = 3.1f32;
    let mut wood = cuboid([-0.72, -20.6, -0.72], [0.72, 13.4, 0.72]);
    // The ring of the head: twelve pieces round the crystal, in the staff's plane.
    let mut ring = Vec::new();
    for j in 0..12 {
        let th = j as f32 / 12.0 * std::f32::consts::TAU;
        let (x, y) = (ring_r * th.sin(), head_y + ring_r * th.cos());
        ring.push(part(
            "ring",
            at(x, y, 0.0, 0.0, 0.0, -th.to_degrees() - 90.0),
            cuboid([-0.45, -0.9, -0.45], [0.45, 0.9, 0.45]),
            Vec::new(),
        ));
    }
    wood.extend(cuboid([-0.9, -21.6, -0.9], [0.9, -20.4, 0.9]));
    let mut gold = cuboid([-1.0, 12.6, -1.0], [1.0, 13.9, 1.0]);
    for y in [-6.0f32, -12.5, -18.6] {
        gold.extend(cuboid([-0.98, y, -0.98], [0.98, y + 0.75, 0.98]));
    }
    gold.extend(cuboid([-1.0, 2.2, -1.0], [1.0, 2.8, 1.0]));
    // Gold studs on the ring, and the spike on top.
    for th in [0.0f32, 90.0, 180.0, 270.0] {
        let r = th.to_radians();
        let (x, y) = ((ring_r + 0.1) * r.sin(), head_y + (ring_r + 0.1) * r.cos());
        gold.extend(cuboid([x - 0.55, y - 0.55, -0.6], [x + 0.55, y + 0.55, 0.6]));
    }
    gold.extend(cuboid([-0.45, head_y - ring_r - 2.6, -0.45], [0.45, head_y - ring_r - 0.6, 0.45]));
    let swell = 1.0 + 0.45 * flash;
    // A gem of two turned prisms, a bipyramid's look in boxes, spinning slowly.
    let facet = part(
        "facet",
        at(0.0, 0.0, 0.0, 0.0, 45.0, 0.0),
        {
            let mut p = cuboid([-0.62, -1.15, -0.62], [0.62, 1.15, 0.62]);
            p.extend(cuboid([-0.32, -1.9, -0.32], [0.32, 1.9, 0.32]));
            p
        },
        Vec::new(),
    );
    let crystal = part(
        "crystal",
        [0.0, head_y, 0.0, 0.0, spin.to_radians(), 0.0, swell, swell, swell],
        {
            let mut p = cuboid([-0.85, -1.0, -0.85], [0.85, 1.0, 0.85]);
            p.extend(cuboid([-0.45, -1.7, -0.45], [0.45, 1.7, 0.45]));
            p
        },
        vec![facet],
    );
    let mut wood_part = part("staff", hand, wood, Vec::new());
    wood_part.children.extend(ring);
    (
        wood_part,
        part("staff_gold", hand, gold, Vec::new()),
        part("staff_head", hand, Vec::new(), vec![crystal]),
    )
}

fn tint(color: u32, rgb: u32) -> u32 {
    let ch = |c: u32, shift: u32| ((c >> shift) & 0xFF) as u32;
    let r = ch(color, 0) * ((rgb >> 16) & 0xFF) / 255;
    let g = ch(color, 8) * ((rgb >> 8) & 0xFF) / 255;
    let b = ch(color, 16) * (rgb & 0xFF) / 255;
    let a = ch(color, 24);
    r | g << 8 | b << 16 | a << 24
}

fn is_item(slot: &Option<std::sync::Arc<str>>, item: &str) -> bool {
    slot.as_deref() == Some(item)
}

/// The arm holding the mage's staff, the main hand first: `Some(true)` for the left one.
fn staff_arm(m: &horizon_client::mobs::Mob) -> Option<bool> {
    if is_item(&m.extra.main_hand, ITEM_STAFF) {
        Some(m.main_arm_left())
    } else if is_item(&m.extra.off_hand, ITEM_STAFF) {
        Some(!m.main_arm_left())
    } else {
        None
    }
}

/// How far forward an arm holding an item hangs (`ArmPose::Item`: pi / 10), degrees.
const STAFF_HOLD: f32 = 18.0;
/// Third person, standing with the staff: its top leans this far forward and out (degrees, in the body's
/// space), its foot by the foot — a walking staff, whatever the arm's own turn.
const STAFF_FWD: f32 = 7.0;
const STAFF_OUT: f32 = 5.0;

/// `player_elytra_geometry`'s HEAD: the outfit of the player `m` drawn this frame.
pub fn outfit(
    game: &Game<'_>,
    m: &horizon_client::mobs::Mob,
    camera: glam::DVec3,
    partial: f32,
    light: u32,
    out: &mut Vec<horizon_client::mesher::Vertex>,
) {
    use horizon_client::mobs::{anim, render};
    // The dress is cloth, not a chestplate: kotost_body keeps its bust and its hourglass body under it.
    horizon_client::kotost_body::cloth(ITEM_DRESS);
    let t = tune();
    // Each thing is drawn while its item is on: the hat on the head, the dress on the chest, the staff
    // in a hand.
    let staff_left = staff_arm(m);
    let (want_hat, want_dress, want_staff) = (
        is_item(&m.extra.head, ITEM_HAT),
        is_item(&m.extra.chest, ITEM_DRESS),
        staff_left.is_some(),
    );
    if !(want_hat || want_dress || want_staff) || horizon_client::mobs::trader::is_invisible(m) {
        return;
    }
    let own = game.player_model.as_ref().is_some_and(|p| std::ptr::eq(p, m));
    let key = if own { u32::MAX } else { m.id };
    let fp = own && game.camera_type.first_person();
    if own {
        horizon_client::kotost_mage_pose::set_own(Some(m.id), fp);
    }
    // The slit and the idle stance go to the leg away from the staff (without one: from the main hand).
    let slit_left = !staff_left.unwrap_or_else(|| m.main_arm_left());
    if want_dress {
        horizon_client::kotost_mage_pose::dressed(m.id, slit_left, t.stance);
    }
    let (layer, _) = render::body(m);
    let body = m.kind.layer(&layer).unwrap_or(&m.kind.model);
    let mut poses = anim::Poses::pooled();
    poses.reset(body);
    anim::setup(m, &anim::state(m, partial), &mut poses, &game.res.mob_data().animations);
    let base = render::base_transform(m, partial, camera);
    let white = game.gui.atlas.font.white();
    let uv = [white.u0, white.v0, white.u1, white.v1];
    // Bare skin (the thigh above a stocking): one texel of the wearer's own cheek.
    let skin_uv = m
        .skin
        .as_deref()
        .and_then(|s| game.gui.atlas.sprites.get(s))
        .or_else(|| game.gui.atlas.sprites.get(&m.kind.texture))
        .map(|r| {
            let (du, dv) = (r.u1 - r.u0, r.v1 - r.v0);
            [r.u0 + du * 12.3 / 64.0, r.v0 + dv * 15.3 / 64.0, r.u0 + du * 12.7 / 64.0, r.v0 + dv * 15.7 / 64.0]
        });
    let red = m.hurt_time > 0 || m.death_time > 0;
    let overlay = horizon_client::overlay::living(0.0, red);
    let pose_of = |p: &ModelPart| p.pose;
    let draw_uv = |piece: &ModelPart, bone: glam::Mat4, uv: [f32; 4], rgb: u32, bright: bool, out: &mut Vec<horizon_client::mesher::Vertex>| {
        let start = out.len();
        horizon_client::mobs::render_model(piece, bone, &pose_of, uv, light, overlay, out);
        for v in &mut out[start..] {
            v.color = tint(v.color, rgb);
            if bright {
                v.light = 240 | 240 << 8;
                v.normal = [0; 4];
            }
        }
    };
    let draw = |piece: &ModelPart, bone: glam::Mat4, rgb: u32, bright: bool, out: &mut Vec<horizon_client::mesher::Vertex>| {
        draw_uv(piece, bone, uv, rgb, bright, out);
    };
    // Skin: the cheek's texel tinted, lit softly (the shade of its faces raised from 0.4..1 to 0.8..1).
    let draw_skin = |piece: &ModelPart, bone: glam::Mat4, rgb: u32, out: &mut Vec<horizon_client::mesher::Vertex>| {
        let start = out.len();
        let (suv, base) = skin_uv.map_or((uv, C_SKIN), |s| (s, 0xFF_FFFF));
        horizon_client::mobs::render_model(piece, bone, &pose_of, suv, light, overlay, out);
        let l0 = glam::Vec3::new(0.2, 1.0, -0.7).normalize();
        let l1 = glam::Vec3::new(-0.2, 1.0, 0.7).normalize();
        for v in &mut out[start..] {
            let shade = if v.normal[3] == 0 {
                1.0
            } else {
                let n = glam::Vec3::new(f32::from(v.normal[0]), f32::from(v.normal[1]), f32::from(v.normal[2])) / 127.0;
                ((l0.dot(n).max(0.0) + l1.dot(n).max(0.0)) * 0.6 + 0.4).min(1.0)
            };
            let k = 0.8 + 0.2 * shade;
            let c = |shift: u32| ((((rgb >> shift) & 0xFF) as f32 * k) as u32).min(255);
            v.color = tint(tint(v.color, base), c(16) << 16 | c(8) << 8 | c(0));
            v.normal = [0; 4];
        }
    };
    let paint = |pieces: Vec<(ModelPart, Paint)>, bone: glam::Mat4, out: &mut Vec<horizon_client::mesher::Vertex>| {
        for (piece, how) in &pieces {
            match *how {
                Paint::Lit(rgb) => draw(piece, bone, rgb, false, out),
                Paint::Glow(rgb) => draw(piece, bone, rgb, true, out),
                Paint::Skin(rgb) => draw_skin(piece, bone, rgb, out),
            }
        }
    };
    let springs = (want_hat || want_dress).then(|| skirt_pose(m, key, partial, &t));
    let (back, side, flare, nod) = springs.unwrap_or_default();
    if want_hat
        && !poses.is_hidden_named("head")
        && let Some(bone) = render::part_matrix(body, base, &poses, "head", 0.0)
    {
        let head_x = poses.get("head")[3];
        paint(hat_parts(nod, side, head_x), bone, out);
    }
    if want_dress
        && !poses.is_hidden_named("body")
        && let Some(bone) = render::part_matrix(body, base, &poses, "body", 0.0)
    {
        use horizon_client::kotost_body as kb;
        // kotost_body's hourglass body under it: the corset and the skirt follow its waist and hips.
        let shape = if kb::shaped(m.id) { kb::waist_hips() } else { None };
        let hw = |y: f32| shape.map_or(4.0, |(waist, hips)| kb::width_at(y, waist, hips));
        let legs = legs_in_body(&mut poses);
        let (cloth, lining, gold) = skirt_part(back, side, flare, &t, &legs, slit_left, &hw);
        draw(&cloth, bone, C_DRESS, false, out);
        draw(&lining, bone, C_LINING, false, out);
        draw(&gold, bone, C_GOLD, false, out);
        // A bust (kotost_body's, from the skin or the item): the dress takes it over and hugs its shape.
        if kb::bust_on(m.id) {
            kb::cover(key);
            paint(dress_bust(kb::jiggle(m, own, partial)), bone, out);
        }
        paint(bodice_parts(&hw), bone, out);
        // Sleeves on both arms (slim arms are three pixels wide, wide ones four).
        let w = if m.slim { 3.0 } else { 4.0 };
        for (arm, x0, x1) in [("right_arm", -w + 1.0, 1.0f32), ("left_arm", -1.0, w - 1.0)] {
            if fp || poses.is_hidden_named(arm) {
                continue;
            }
            if let Some(bone) = render::part_matrix(body, base, &poses, arm, 0.0) {
                paint(sleeve_parts(x0, x1), bone, out);
            }
        }
        if t.stockings {
            for (leg, outer) in [("right_leg", -1.0f32), ("left_leg", 1.0)] {
                if poses.is_hidden_named(leg) {
                    continue;
                }
                if let Some(bone) = render::part_matrix(body, base, &poses, leg, 0.0) {
                    paint(leg_parts(outer), bone, out);
                }
            }
        }
    }
    let left = staff_left.unwrap_or(false);
    let arm = if left { "left_arm" } else { "right_arm" };
    if want_staff
        && !poses.is_hidden_named(arm)
        && let Some(bone) = render::part_matrix(body, base, &poses, arm, 0.0)
    {
        let spin = (m.age as f32 + partial) * 3.0;
        // Third person: the arm hangs a little forward (kotost_mage_pose::arms), the staff is turned in the hand
        // by the arm's whole turn so it stands as a walking staff (top `STAFF_FWD` forward, `STAFF_OUT` out).
        // First person: as before, the arm hanging 18 degrees forward (`ArmPose::Item`).
        let a = *poses.get(arm);
        let arm_x = a[3].to_degrees();
        let sx = if left { -1.0f32 } else { 1.0 };
        let arm_q = glam::Quat::from_euler(glam::EulerRot::ZYX, a[5], a[4], a[3]);
        let upright = glam::Quat::from_euler(
            glam::EulerRot::ZYX,
            (-sx * STAFF_OUT).to_radians(),
            0.0,
            STAFF_FWD.to_radians(),
        );
        let rest = if fp {
            glam::Quat::from_euler(glam::EulerRot::ZYX, (-9.0 * sx).to_radians(), 0.0, (62.0 + STAFF_HOLD).to_radians())
        } else {
            arm_q.inverse() * upright
        };
        // While casting: turned in the hand about x only, so the crystal points where the cast wants it
        // whatever the arm does (the swing reads as before); the crystal swells for a moment.
        let (turn, flash) = match horizon_client::kotost_mage_pose::staff_lean(m.id) {
            Some((goal, w, _)) => {
                let want = if fp {
                    glam::Quat::from_euler(glam::EulerRot::ZYX, (-9.0 * sx).to_radians(), 0.0, (70.0 - arm_x).to_radians())
                } else {
                    glam::Quat::from_rotation_x((goal - arm_x).to_radians())
                };
                // Gathering, the crystal swells little by little; gone off, it flashes.
                let flash = horizon_client::kotost_mage_pose::now(m.id).map_or(0.0, |(c, s, _)| {
                    if s < c.windup {
                        0.6 * (s / c.windup.max(1e-3)).clamp(0.0, 1.0)
                    } else {
                        (1.0 - (s - c.windup) / 0.35).clamp(0.0, 1.0)
                    }
                });
                (rest.slerp(want, w.clamp(0.0, 1.0)), flash)
            }
            None => (rest, 0.0),
        };
        let (tz, ty, tx) = turn.to_euler(glam::EulerRot::ZYX);
        let (wood, gold, head) = staff_parts(left, spin, [tx.to_degrees(), ty.to_degrees(), tz.to_degrees()], flash);
        draw(&wood, bone, C_WOOD, false, out);
        draw(&gold, bone, C_GOLD, false, out);
        let start = out.len();
        let pulse = 0.8 + 0.2 * ((m.age as f32 + partial) * 0.25).sin() + 0.3 * flash;
        let glow = {
            let c = |v: f32| ((v * pulse).clamp(0.0, 255.0)) as u32;
            c(200.0) << 16 | c(140.0) << 8 | c(255.0)
        };
        draw(&head, bone, glow, true, out);
        let drawn = &out[start..];
        if !drawn.is_empty() {
            let mut sum = [0.0f64; 3];
            for v in drawn {
                for i in 0..3 {
                    sum[i] += f64::from(v.pos[i]);
                }
            }
            let n = drawn.len() as f64;
            let at = [camera.x + sum[0] / n, camera.y + sum[1] / n, camera.z + sum[2] / n];
            client(|c| {
                if c.crystals.len() > 32 {
                    c.crystals.clear();
                }
                c.crystals.insert(key, (at, Instant::now()));
            });
        }
    }
}
