use uuid::Uuid;

pub fn rich_edit(contents: Option<&str>) -> Option<String> {
    let editor = std::env::var("EDITOR");

    if editor.is_err() {
        return None;
    }

    let mut dir = std::env::temp_dir();
    let file_name = Uuid::new_v4().to_string();
    dir.push(&file_name);
    let file_path = dir.to_str().unwrap();

    std::fs::write(file_path, contents.unwrap_or("")).unwrap();

    let result = match std::process::Command::new(editor.unwrap())
        .arg(&file_path)
        .status()
    {
        Ok(status) => {
            if status.success() {
                match std::fs::read_to_string(&file_path) {
                    Ok(content) => Some(content),
                    Err(_) => None,
                }
            } else {
                None
            }
        }
        Err(_) => None,
    };

    // Cleanup
    std::fs::remove_file(file_path).unwrap();

    result
}
