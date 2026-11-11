// Kotost Mage: vanilla sounds the mage's own spells set off, kept quiet (horizon_client_render::kotost_mage_mute).
// A blast the mod makes with the game's explosion still sends vanilla's bang; the mod plays its own instead,
// so the client mutes that one event near that spot for a moment (`mute`), heard in `Audio::play_seeded`'s
// head (`muted`).

use std::sync::Mutex;
use std::time::Instant;

struct Zone {
    event: &'static str,
    at: [f64; 3],
    radius: f64,
    until: Instant,
}

static ZONES: Mutex<Vec<Zone>> = Mutex::new(Vec::new());

/// Mutes `event` within `radius` blocks of `at` for `secs` seconds.
pub fn mute(event: &'static str, at: [f64; 3], radius: f64, secs: f32) {
    let mut z = ZONES.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let now = Instant::now();
    z.retain(|z| z.until > now);
    if z.len() < 64 {
        z.push(Zone {
            event,
            at,
            radius,
            until: now + std::time::Duration::from_secs_f32(secs.clamp(0.05, 10.0)),
        });
    }
}

/// Whether `event` at `at` is one to keep quiet now.
pub fn muted(event: &str, at: Option<[f64; 3]>) -> bool {
    let Some(at) = at else { return false };
    let z = ZONES.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if z.is_empty() {
        return false;
    }
    let now = Instant::now();
    let name = event.strip_prefix("minecraft:").unwrap_or(event);
    z.iter().any(|z| {
        z.until > now
            && z.event.strip_prefix("minecraft:").unwrap_or(z.event) == name
            && (0..3).map(|i| (z.at[i] - at[i]).powi(2)).sum::<f64>() <= z.radius * z.radius
    })
}
