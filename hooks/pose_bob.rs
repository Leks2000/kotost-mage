fn kotost_mage_pose_bob(p: &mut Poses, n: &str) {
    if n == "right_arm" {
        crate::kotost_mage_pose::legs(p);
        crate::kotost_mage_pose::arms(p);
    }
}
