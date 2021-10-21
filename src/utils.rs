use std::fs::DirEntry;

pub fn entry_not_hidden(entry: &DirEntry) -> bool {
    !entry
        .path()
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with(".")
}
