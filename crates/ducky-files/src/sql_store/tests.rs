use super::*;

fn store() -> (tempfile::TempDir, SqlStore) {
    let temp_dir = tempfile::tempdir().unwrap();
    let store = SqlStore::new(temp_dir.path().join("sql"));
    (temp_dir, store)
}

#[test]
fn save_list_load_round_trip() {
    let (_temp_dir, store) = store();
    let saved_entry = store.save("b", "select 2;", false).unwrap();
    assert_eq!(saved_entry.name, "b.sql");
    assert_eq!(saved_entry.size_bytes, 9);
    store.save("a.sql", "select 1;", false).unwrap();
    let names: Vec<String> = store
        .list()
        .unwrap()
        .into_iter()
        .map(|entry| entry.name)
        .collect();
    assert_eq!(names, vec!["a.sql", "b.sql"]);
    assert_eq!(store.load("b.sql").unwrap(), "select 2;");
}

#[test]
fn list_ignores_other_extensions() {
    let (temp_dir, store) = store();
    store.save("a", "x", false).unwrap();
    std::fs::write(temp_dir.path().join("sql/notes.txt"), "x").unwrap();
    assert_eq!(store.list().unwrap().len(), 1);
}

#[test]
fn save_without_overwrite_fails_when_existing() {
    let (_temp_dir, store) = store();
    store.save("a", "one", false).unwrap();
    assert_eq!(
        store.save("a", "two", false),
        Err(FilesError::AlreadyExists("a.sql".to_owned()))
    );
    store.save("a", "two", true).unwrap();
    assert_eq!(store.load("a").unwrap(), "two");
}

#[test]
fn missing_directory_lists_empty() {
    let (_temp_dir, store) = store();
    assert_eq!(store.list().unwrap(), Vec::new());
}

#[test]
fn load_missing_file_is_not_found() {
    let (_temp_dir, store) = store();
    assert_eq!(
        store.load("nope"),
        Err(FilesError::NotFound("nope.sql".to_owned()))
    );
}

#[test]
fn load_rejects_too_large_file() {
    let (temp_dir, store) = store();
    std::fs::create_dir_all(temp_dir.path().join("sql")).unwrap();
    let file = std::fs::File::create(temp_dir.path().join("sql/big.sql")).unwrap();
    file.set_len(MAX_EDITOR_FILE_BYTES + 1).unwrap();
    assert_eq!(
        store.load("big.sql"),
        Err(FilesError::TooLarge {
            name: "big.sql".to_owned(),
            size_bytes: MAX_EDITOR_FILE_BYTES + 1
        })
    );
}

#[test]
fn preview_truncates_at_line_limit() {
    let (_temp_dir, store) = store();
    let content: String = (0..250).map(|index| format!("select {index};\n")).collect();
    store.save("long", &content, false).unwrap();
    let preview = store.preview("long").unwrap();
    assert_eq!(preview.lines().count(), PREVIEW_LINE_LIMIT);
    assert_eq!(preview.lines().last(), Some("select 199;"));
}
