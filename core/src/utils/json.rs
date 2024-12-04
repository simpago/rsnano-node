pub fn as_nano_json(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}
