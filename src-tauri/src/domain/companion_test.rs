use crate::domain::companion::{CompanionSkin, MotionSettings, SkinSource, VisualPreset};

#[test]
fn serializes_companion_skin_for_the_frontend() {
    let skin = CompanionSkin::builtin(VisualPreset::QuietAurora);

    assert_eq!(serde_json::to_value(skin).unwrap()["id"], "quiet-aurora");
    assert_eq!(
        serde_json::to_value(CompanionSkin::builtin(VisualPreset::QuietAurora)).unwrap()
            ["visualPreset"],
        "quiet_aurora"
    );
}

#[test]
fn serializes_distinct_image_and_package_skin_sources() {
    assert_eq!(serde_json::to_value(SkinSource::Image).unwrap(), "image");
    assert_eq!(
        serde_json::to_value(SkinSource::Package).unwrap(),
        "package"
    );
}

#[test]
fn clamps_motion_settings_to_safe_ranges() {
    let settings = MotionSettings::new(9.0, -2.0);

    assert_eq!(settings.flow_speed, 2.0);
    assert_eq!(settings.flow_intensity, 0.0);
    assert_eq!(MotionSettings::new(-1.0, 0.5).flow_speed, 0.5);
}

#[test]
fn defines_the_locked_builtin_visual_palettes() {
    let expected = [
        (VisualPreset::QuietAurora, vec!["#BD9FFF", "#FFF4DC"]),
        (VisualPreset::PorcelainPearl, vec!["#E3BD7E", "#FFF8EA"]),
        (VisualPreset::DeepInk, vec!["#49D9CF", "#D4FFF8"]),
    ];

    for (preset, colors) in expected {
        let skin = CompanionSkin::builtin(preset);
        assert_eq!(skin.flow_colors, colors);
        assert_eq!(skin.flow_speed, 1.0);
        assert_eq!(skin.flow_intensity, 0.7);
    }
}
