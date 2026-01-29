use crate::{Action, Achievements, Material};

#[test]
fn core_counts() {
    assert_eq!(Action::classic_actions().len(), 17);
    assert_eq!(Achievements::all_names().len(), 22);
}

#[test]
fn material_indices_cover_classic() {
    for idx in 0u8..=11 {
        assert!(Material::from_index(idx).is_some());
    }
}
