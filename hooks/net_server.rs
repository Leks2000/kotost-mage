fn kotost_mage_net_from_server(game: &mut crate::game::Game<'_>, channel: &str, data: &[u8]) {
    crate::game::kotost_mage::net_from_server(game, channel, data);
}
