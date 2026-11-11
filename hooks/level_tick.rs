fn kotost_mage_level_tick(game: &mut crate::game::Game<'_>, dimension: &str) {
    let dimension = dimension.to_owned();
    crate::game::kotost_mage::level_tick(game, &dimension);
}
