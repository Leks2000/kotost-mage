fn kotost_mage_mute_sound(event: &str, at: Option<[f64; 3]>) -> std::ops::ControlFlow<()> {
    if crate::kotost_mage_mute::muted(event, at) {
        std::ops::ControlFlow::Break(())
    } else {
        std::ops::ControlFlow::Continue(())
    }
}
