use crafter_core as reference;
use crafter_core_modular as modular;
use serde::Serialize;

fn assert_json_eq<L: Serialize, R: Serialize>(label: &str, left: &L, right: &R) {
    let left_value = serde_json::to_value(left).expect("serialize left");
    let right_value = serde_json::to_value(right).expect("serialize right");
    assert_eq!(left_value, right_value, "mismatch in {label}");
}

fn config_pair() -> (reference::SessionConfig, modular::SessionConfig) {
    let mut config_ref = reference::SessionConfig::default();
    config_ref.seed = Some(123);
    config_ref.world_size = (16, 16);
    config_ref.view_radius = 4;

    let config_mod: modular::SessionConfig =
        serde_json::from_value(serde_json::to_value(&config_ref).expect("serialize config"))
            .expect("deserialize config");
    (config_ref, config_mod)
}

#[test]
fn session_steps_match_reference() {
    let (config_ref, config_mod) = config_pair();
    let mut session_ref = reference::Session::new(config_ref);
    let mut session_mod = modular::Session::new(config_mod);

    assert_json_eq("initial_state", &session_mod.get_state(), &session_ref.get_state());

    let action_indices = [
        3u8, 3, 2, 2, 5, 0, 4, 4, 1, 5, 7, 8, 10, 6, 11, 14,
    ];

    for (step_idx, action_index) in action_indices.iter().enumerate() {
        let action_ref = reference::Action::from_index(*action_index).expect("ref action");
        let action_mod = modular::Action::from_index(*action_index).expect("mod action");
        let result_ref = session_ref.step(action_ref);
        let result_mod = session_mod.step(action_mod);

        assert_json_eq(
            &format!("step_{step_idx}_result"),
            &result_mod,
            &result_ref,
        );
    }
}
