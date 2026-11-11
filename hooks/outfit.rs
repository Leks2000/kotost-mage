fn kotost_mage_outfit(
    game: &crate::game::Game<'_>,
    m: &Mob,
    camera: glam::DVec3,
    partial: f32,
    light: u32,
    out: &mut Vec<horizon_client::mesher::Vertex>,
) {
    crate::game::kotost_mage::outfit(game, m, camera, partial, light, out);
}
