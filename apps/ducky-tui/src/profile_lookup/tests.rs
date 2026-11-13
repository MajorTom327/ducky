use ducky_core::ProfileKind;

use super::*;

fn profile(label: &str, project: Option<&str>, kind: ProfileKind) -> Profile {
    Profile {
        label: label.to_owned(),
        url: "postgres://h/db".to_owned(),
        kind,
        project: project.map(str::to_owned),
        tags: Vec::new(),
    }
}

fn fixture() -> Vec<Profile> {
    vec![
        profile("Local", None, ProfileKind::Local),
        profile("", Some("shop"), ProfileKind::Prod),
        profile("Main", Some("shop"), ProfileKind::Staging),
        profile("Main", Some("blog"), ProfileKind::Staging),
        profile("", Some("blog"), ProfileKind::Prod),
    ]
}

#[test]
fn resolves_label() {
    assert_eq!(resolve_profile(&fixture(), "Local"), Ok(0));
}

#[test]
fn resolves_project_and_label() {
    assert_eq!(resolve_profile(&fixture(), "blog/Main"), Ok(3));
}

#[test]
fn resolves_project_and_type() {
    assert_eq!(resolve_profile(&fixture(), "shop/prod"), Ok(1));
    assert_eq!(resolve_profile(&fixture(), "shop/staging"), Ok(2));
}

#[test]
fn unknown_query_is_not_found() {
    assert_eq!(
        resolve_profile(&fixture(), "nope"),
        Err(ProfileLookupError::NotFound)
    );
    assert_eq!(
        resolve_profile(&fixture(), ""),
        Err(ProfileLookupError::NotFound)
    );
}

#[test]
fn shared_label_is_ambiguous() {
    assert_eq!(
        resolve_profile(&fixture(), "Main"),
        Err(ProfileLookupError::Ambiguous {
            matching_profile_indexes: vec![2, 3]
        })
    );
}
