// Kotost Mage: the mage's cast pose, in horizon-client-lib's root (horizon_client::kotost_mage_pose) so both the
// model's animation (anim::setup, anim::bob hooks) and the game's kotost_mage module reach it.
// The game's module says who casts what (`start`, from the server's cast message); `setup`'s head names the
// mob being posed (`begin`); `bob`'s head, near the end of a humanoid's pose, turns its arms (`arms`).

use std::cell::Cell;
use std::collections::HashMap;
use std::f32::consts::PI;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

/// The pose a spell is cast with.
pub const RAISE: u8 = 1; // the staff held high over the head (meteor, supernova)
pub const THRUST: u8 = 2; // the staff and the other hand pushed at the aim (fireball)
pub const SLAM: u8 = 3; // the staff raised, then struck down in front (absolute zero)
pub const PALM: u8 = 4; // the staff up, the free hand flung at the aim (chain lightning)
pub const SWEEP: u8 = 5; // a sweep of the staff across (shadow step)

#[derive(Clone, Copy, Debug)]
pub struct Cast {
    pub kind: u8,
    pub at: Instant,
    /// How long the pose holds, seconds (rise and fall included).
    pub secs: f32,
    /// The staff is in the left hand.
    pub left: bool,
    /// Seconds it gathers (the arms draw the power in) before it goes off.
    pub windup: f32,
}

static CASTS: Mutex<Option<HashMap<u32, Cast>>> = Mutex::new(None);
static FIRST_PERSON_ID: Mutex<Option<u32>> = Mutex::new(None);
static FIRST_PERSON: AtomicBool = AtomicBool::new(false);
/// Who wears the dress (the outfit says so every frame it draws it): when, the slit over the left leg,
/// how far the idle stance goes (0..1.5).
static DRESSED: Mutex<Option<HashMap<u32, (Instant, bool, f32)>>> = Mutex::new(None);

thread_local! {
    /// The mob posed now on this thread, how still it stands (1: idle, 0: walking or not upright), and the
    /// hand holding the mage's staff (`Some(true)`: the left).
    static CURRENT: Cell<Option<(u32, f32, Option<bool>)>> = const { Cell::new(None) };
}

/// The mage's staff (the content mod's item).
const STAFF: &str = "kotost_mage_items:mage_staff";
/// The staff arm while it holds the staff: a little forward (radians about x, the game's arm hangs at 0)
/// and out from the hip, the hand by the hip's front gripping the staff like a walking staff (its foot on
/// the ground by the foot); how much of the walk's swing is left to it.
const HOLD_X: f32 = -0.3;
const HOLD_IN: f32 = 0.08;
const HOLD_OUT: f32 = 0.06;
const HOLD_SWING: f32 = 0.3;

/// The arm holding the staff, the main hand first: `Some(true)` for the left one.
fn staff_hand(m: &crate::mobs::Mob) -> Option<bool> {
    let left = m.main_arm_left();
    if m.extra.main_hand.as_deref() == Some(STAFF) {
        Some(left)
    } else if m.extra.off_hand.as_deref() == Some(STAFF) {
        Some(!left)
    } else {
        None
    }
}

/// Mob `id` wears the dress: the leg under the slit (`slit_left`) steps forward when it stands, by `stance`.
pub fn dressed(id: u32, slit_left: bool, stance: f32) {
    let mut g = lock(&DRESSED);
    let all = g.get_or_insert_with(HashMap::new);
    if all.len() > 64 {
        all.retain(|_, d| d.0.elapsed().as_secs_f32() < 2.0);
    }
    all.insert(id, (Instant::now(), slit_left, stance));
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Mob `id` casts now: pose `kind` for `secs`, the first `windup` of them gathering.
pub fn start(id: u32, kind: u8, secs: f32, left: bool, windup: f32) {
    let mut g = lock(&CASTS);
    let all = g.get_or_insert_with(HashMap::new);
    if all.len() > 64 {
        all.retain(|_, c| c.at.elapsed().as_secs_f32() < c.secs);
    }
    let secs = secs.clamp(0.2, 12.0);
    all.insert(
        id,
        Cast {
            kind,
            at: Instant::now(),
            secs,
            left,
            windup: windup.clamp(0.0, secs),
        },
    );
}

/// The server says mob `id`'s spell went off now: the release runs from this moment.
pub fn release(id: u32) {
    if let Some(c) = lock(&CASTS).as_mut().and_then(|all| all.get_mut(&id)) {
        let t = c.at.elapsed().as_secs_f32();
        if t < c.secs && (t - c.windup).abs() < 0.4 {
            let rest = (c.secs - c.windup).max(0.3);
            c.windup = t;
            c.secs = t + rest;
        }
    }
}

/// Mob `id`'s spell fizzled: the arms fall back without going off.
pub fn cancel(id: u32) {
    if let Some(c) = lock(&CASTS).as_mut().and_then(|all| all.get_mut(&id)) {
        let t = c.at.elapsed().as_secs_f32();
        c.windup = t + 60.0;
        c.secs = (t + 0.3).min(c.secs.max(t));
    }
}

/// The own model's id and whether the camera is in first person (its arms then keep in view).
pub fn set_own(id: Option<u32>, first_person: bool) {
    *lock(&FIRST_PERSON_ID) = id;
    FIRST_PERSON.store(first_person, Ordering::Relaxed);
}

/// Mob `id`'s cast now: the cast, seconds into it, and its weight (0..1: rising, held, falling).
pub fn now(id: u32) -> Option<(Cast, f32, f32)> {
    let c = *lock(&CASTS).as_ref()?.get(&id)?;
    let t = c.at.elapsed().as_secs_f32();
    if t >= c.secs {
        return None;
    }
    let rise = (t / 0.11).min(1.0);
    let fall = ((c.secs - t) / 0.3).min(1.0);
    let w = smooth(rise.min(fall));
    Some((c, t, w))
}

fn smooth(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// `anim::setup`'s head: the mob posed next on this thread, and how still it stands.
pub fn begin(m: &crate::mobs::Mob) {
    let upright = m.alive()
        && !m.extra.passenger
        && !m.extra.crouching
        && m.extra.avatar_pose == crate::pose::Pose::Standing;
    let speed = m.limb_speed.max(m.previous_limb_speed);
    let still = if upright { 1.0 - smooth((speed - 0.02) / 0.3) } else { 0.0 };
    CURRENT.with(|c| c.set(Some((m.id, still, staff_hand(m)))));
}

/// `anim::bob`'s head for the right arm: a dressed mob standing still puts the leg under the slit forward
/// and a little out, toes turned out, the other leg back a touch (the walk's swing fades into it).
pub fn legs(p: &mut crate::mobs::anim::Poses) {
    let Some((id, still, _)) = CURRENT.with(Cell::get) else {
        return;
    };
    if still <= 0.0 || !p.has("right_leg") || !p.has("left_leg") {
        return;
    }
    let Some((slit_left, stance)) = lock(&DRESSED)
        .as_ref()
        .and_then(|all| all.get(&id))
        .filter(|d| d.0.elapsed().as_secs_f32() < 1.0)
        .map(|d| (d.1, d.2))
    else {
        return;
    };
    let w = still * stance;
    if w <= 0.0 {
        return;
    }
    // Right leg's numbers; the left leg's turn about y and z is mirrored.
    let (front, back, sign_front) = if slit_left {
        ("left_leg", "right_leg", -1.0)
    } else {
        ("right_leg", "left_leg", 1.0)
    };
    for (leg, x, y, z, s) in [(front, -0.34f32, 0.3f32, 0.09f32, sign_front), (back, 0.1, 0.12, 0.05, -sign_front)] {
        let q = p.get(leg);
        q[3] += x * w;
        q[4] += y * s * w;
        q[5] += z * s * w;
        // The foot kept on the ground: the hip lowered by what the swing lifts it.
        q[1] += 12.0 * (1.0 - (x * w).cos() * (z * w).cos());
    }
}

fn lerp3(a: [f32; 3], b: [f32; 3], k: f32) -> [f32; 3] {
    [a[0] + (b[0] - a[0]) * k, a[1] + (b[1] - a[1]) * k, a[2] + (b[2] - a[2]) * k]
}

/// Target angles of the staff arm and the free arm while the spell gathers (`g`: 0..1 of the way), the
/// head's pitch and turn given. Right-handed; mirrored by the caller. Each ends where its release starts.
fn gathering(kind: u8, g: f32, head_x: f32, head_y: f32, first_person: bool) -> ([f32; 3], [f32; 3]) {
    let aim = head_x - PI / 2.0;
    let e = smooth(g);
    if first_person {
        // In view: the staff comes up before the eyes, the free hand reaches in toward it.
        return (
            [aim - 0.1 - 0.4 * e, head_y * 0.6, 0.05 + 0.12 * e],
            [aim + 0.15 - 0.25 * e, head_y + 0.35 + 0.25 * e, -0.1],
        );
    }
    match kind {
        // The staff rising slowly overhead, the free hand opening up beside it.
        RAISE => (lerp3([-0.9, 0.1, 0.15], [-2.95, 0.0, 0.18], e), lerp3([-0.5, 0.3, -0.25], [aim + 0.35, 0.45, -0.35], e)),
        // The staff drawn back across the body, the free hand cupping the fire before the chest.
        THRUST => (lerp3([-0.6, 0.1, 0.1], [-0.75, 0.5, 0.3], e), lerp3([-0.6, 0.3, -0.1], [aim + 0.35, 0.7, -0.15], e)),
        // Up over the head, gathering the cold to strike down.
        SLAM => (lerp3([-1.0, 0.0, 0.1], [-2.85, 0.0, 0.12], e), lerp3([-0.5, 0.2, -0.3], [aim + 0.5, 0.3, -0.45], e)),
        // The staff up, the free hand drawn back by the shoulder to fling the bolt.
        PALM => (lerp3([-1.0, 0.15, 0.2], [-2.2, 0.15, 0.25], e), lerp3([-0.4, 0.2, -0.2], [0.55, -0.15, -0.35], e)),
        // Wound up across the body for the sweep.
        _ => (lerp3([-0.6, 0.0, 0.1], [aim + 0.4, -0.9, 0.2], e), lerp3([-0.4, 0.2, -0.2], [aim + 0.6, 0.3, -0.3], e)),
    }
}

/// Target angles of the staff arm and the free arm (x, y, z radians) for a cast `t` seconds after it went
/// off, the head's pitch and turn given. Right-handed; mirrored by the caller.
fn targets(kind: u8, t: f32, head_x: f32, head_y: f32, first_person: bool) -> ([f32; 3], [f32; 3]) {
    let aim = head_x - PI / 2.0;
    if first_person {
        // In view: the staff pushed forward and up, the hand a little behind it.
        let lift = match kind {
            RAISE => 0.55,
            SLAM if t < 0.2 => 0.6,
            SLAM => -0.1,
            _ => 0.25,
        };
        return ([aim - lift, head_y * 0.6, 0.05], [aim + 0.15, head_y + 0.35, -0.1]);
    }
    match kind {
        RAISE => ([-2.95, 0.0, 0.18], [aim + 0.35, 0.45, -0.35]),
        THRUST => ([aim + 0.05, head_y - 0.08, 0.0], [aim + 0.1, head_y + 0.32, -0.08]),
        SLAM => {
            // Struck down in front at once (it was raised while it gathered).
            let k = smooth((t - 0.02) / 0.1);
            ([-2.85 + 2.1 * k, 0.0, 0.12], [aim + 0.5, 0.3, -0.45])
        }
        PALM => ([-2.2, 0.15, 0.25], [aim, head_y + 0.18, -0.05]),
        _ => {
            // A sweep from across the body out to the side.
            let k = smooth(t / 0.3);
            ([aim + 0.4, -0.9 + 1.8 * k, 0.2 + 0.5 * k], [aim + 0.6, 0.3, -0.3])
        }
    }
}

/// `anim::bob`'s head for the right arm: the mob posed now (`begin`), if it casts, turns its arms toward
/// the cast's pose by the cast's weight.
pub fn arms(p: &mut crate::mobs::anim::Poses) {
    let Some((id, _, staff)) = CURRENT.with(Cell::get) else {
        return;
    };
    if !p.has("right_arm") || !p.has("left_arm") {
        return;
    }
    let fp = FIRST_PERSON.load(Ordering::Relaxed) && *lock(&FIRST_PERSON_ID) == Some(id);
    // Holding the staff (third person): the arm a little forward and out, the hand before the hip; the walk
    // swings it only a little. The staff itself is stood upright in the hand by the outfit.
    if let Some(left) = staff
        && !fp
    {
        let q = p.get(if left { "left_arm" } else { "right_arm" });
        q[3] = HOLD_X + q[3] * HOLD_SWING;
        q[4] += if left { HOLD_IN } else { -HOLD_IN };
        q[5] += if left { -HOLD_OUT } else { HOLD_OUT };
    }
    let Some((c, t, w)) = now(id) else {
        return;
    };
    if w <= 0.0 {
        return;
    }
    let head = *p.get("head");
    let (hx, hy) = (head[3], head[4]);
    let tr = t - c.windup;
    let (staff, free) = if tr < 0.0 {
        // Gathering: the arms draw it in, trembling more as it fills.
        let g = if c.windup > 0.0 { t / c.windup } else { 1.0 };
        let (mut s, mut f) = gathering(c.kind, g, hx, hy, fp);
        let shake = 0.035 * smooth(g) * (t * 47.0).sin();
        s[0] += shake;
        f[0] -= shake * 0.8;
        (s, f)
    } else {
        // Gone off: from where the gathering ended into the release, in a snap.
        let (s1, f1) = targets(c.kind, tr, hx, hy, fp);
        if c.windup > 0.0 {
            let (s0, f0) = gathering(c.kind, 1.0, hx, hy, fp);
            let k = smooth(tr / 0.08);
            (lerp3(s0, s1, k), lerp3(f0, f1, k))
        } else {
            (s1, f1)
        }
    };
    let (staff_arm, free_arm, sign) = if c.left {
        ("left_arm", "right_arm", -1.0)
    } else {
        ("right_arm", "left_arm", 1.0)
    };
    // The free hand moves a little after the staff (with no gathering before).
    let w_free = if c.kind == RAISE || c.kind == SLAM || c.windup > 0.0 { w } else { smooth((t - 0.04) / 0.11).min(w) };
    for (arm, goal, k) in [(staff_arm, staff, w), (free_arm, free, w_free)] {
        let q = p.get(arm);
        let goal = [goal[0], goal[1] * sign, goal[2] * sign];
        for i in 0..3 {
            q[3 + i] += (goal[i] - q[3 + i]) * k;
        }
    }
}

/// The staff's own turn in the hand while casting (degrees about x), to keep its crystal up while the arm
/// swings: the angle the staff should make with the vertical (`lean`), given the arm's turn now.
pub fn staff_lean(id: u32) -> Option<(f32, f32, u8)> {
    let (c, t, w) = now(id)?;
    let gathering = t < c.windup;
    let lean = match c.kind {
        RAISE => 4.0,
        THRUST if gathering => 20.0,
        THRUST => 70.0,
        SLAM => 2.0,
        PALM => 10.0,
        _ if gathering => 35.0,
        _ => 55.0,
    };
    Some((lean, w, c.kind))
}
