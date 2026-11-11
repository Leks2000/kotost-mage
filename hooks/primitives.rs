fn kotost_mage_primitives(
    game: &crate::game::Game<'_>,
    partial: f32,
    camera: [f64; 3],
    primitives: &mut WorldPrimitives,
) {
    crate::game::kotost_mage::primitives(game, partial, camera, primitives);
}
