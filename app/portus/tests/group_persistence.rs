use portus_core::session::Protocol;
use portus_lib::commands::{delete_group, save_group, save_session, set_group_collapsed, AuthInput};
use uuid::Uuid;

/// Exercises save/rename/collapse/delete against the real config.json in
/// this machine's Portus config dir, same as saved_session_persistence.rs.
/// Passes explicit ids the test generates itself and looks rows up by id, so
/// a stale row from an earlier failed run can't be mistaken for this run's.
#[test]
fn save_rename_collapse_and_delete_a_folder_round_trip() {
    let id = Uuid::new_v4();
    let config = save_group(Some(id), "test folder".to_string(), None).expect("save_group failed");
    let saved = config.groups.iter().find(|g| g.id == id).expect("saved group missing").clone();
    assert_eq!(saved.name, "test folder");
    assert!(!saved.collapsed);

    let config = save_group(Some(id), "renamed folder".to_string(), None).expect("save_group (rename) failed");
    let renamed = config.groups.iter().find(|g| g.id == id).expect("renamed group missing");
    assert_eq!(renamed.name, "renamed folder");

    let config = set_group_collapsed(id, true).expect("set_group_collapsed failed");
    let collapsed = config.groups.iter().find(|g| g.id == id).expect("group missing after collapse");
    assert!(collapsed.collapsed);

    let config = delete_group(id).expect("delete_group failed");
    assert!(config.groups.iter().all(|g| g.id != id), "group still present after delete");
}

/// Deleting a folder must not delete the saved sessions inside it — they
/// move to the root (group_id: None) instead of vanishing.
#[test]
fn deleting_a_folder_unparents_its_saved_sessions_instead_of_deleting_them() {
    let group_id = Uuid::new_v4();
    save_group(Some(group_id), "temp folder for saved-session unparenting test".to_string(), None)
        .expect("save_group failed");

    let saved_session_id = Uuid::new_v4();
    let config = save_session(
        Some(saved_session_id),
        "test session in folder".to_string(),
        Some(group_id),
        Protocol::Ssh,
        "127.0.0.1".to_string(),
        Some(2222),
        Some("testuser".to_string()),
        None,
        AuthInput::None,
        None,
        None,
    )
    .expect("save_session failed");
    let saved = config.sessions.iter().find(|s| s.id == saved_session_id).expect("saved session missing");
    assert_eq!(saved.group_id, Some(group_id));

    let config = delete_group(group_id).expect("delete_group failed");
    assert!(config.groups.iter().all(|g| g.id != group_id), "group still present after delete");
    let unparented = config.sessions.iter().find(|s| s.id == saved_session_id).expect("saved session vanished after folder delete");
    assert_eq!(unparented.group_id, None, "saved session should be unparented, not deleted, when its folder is deleted");

    let config = portus_lib::commands::delete_session(saved_session_id).expect("cleanup delete_session failed");
    assert!(config.sessions.iter().all(|s| s.id != saved_session_id));
}
