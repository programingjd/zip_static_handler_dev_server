pub(crate) fn file_path(name: &str) -> String {
    let name = name.strip_prefix('/').unwrap_or(name);
    let name = name.strip_suffix('/').unwrap_or(name);
    if name.is_empty() {
        return "index.html".to_string();
    }
    name.to_string()
}

pub(crate) fn filename(name: &str) -> &str {
    let byte_position = name.rfind(|c| c == '/').map(|it| it + 1).unwrap_or(0);
    &name[byte_position..]
}

pub(crate) fn extension(filename: &str) -> &str {
    let byte_position = filename.rfind(|c| c == '.').map(|it| it + 1).unwrap_or(0);
    &filename[byte_position..]
}
