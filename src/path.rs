pub(crate) fn filename(name: &str) -> &str {
    let byte_position = name.rfind(|c| c == '/').map(|it| it + 1).unwrap_or(0);
    &name[byte_position..]
}

pub(crate) fn extension(filename: &str) -> &str {
    let byte_position = filename.rfind(|c| c == '.').map(|it| it + 1).unwrap_or(0);
    &filename[byte_position..]
}
