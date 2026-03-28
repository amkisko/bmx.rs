pub(crate) fn debug_enabled() -> bool {
    std::env::var("BMX_DEBUG")
        .map(|value| {
            matches!(
                value.to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

/// Show git, build tool, and other subprocess output instead of quiet mode / spinner.
pub(crate) fn subprocess_output_visible(cli_verbose: bool) -> bool {
    cli_verbose || debug_enabled()
}

pub(crate) fn debug_log(message: &str) {
    if debug_enabled() {
        eprintln!("[bmx][debug] {message}");
    }
}
