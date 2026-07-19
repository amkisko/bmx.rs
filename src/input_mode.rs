use std::sync::atomic::{AtomicBool, Ordering};

static NO_INPUT: AtomicBool = AtomicBool::new(false);

pub(crate) fn set_no_input(enabled: bool) {
    NO_INPUT.store(enabled, Ordering::Relaxed);
}

pub(crate) fn no_input() -> bool {
    NO_INPUT.load(Ordering::Relaxed)
}

/// Auto-consent for trust prompts: env only (not implied by `--no-input`).
pub(crate) fn trust_assume_yes() -> bool {
    std::env::var("BMX_TRUST_ASSUME_YES")
        .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::{no_input, set_no_input};

    #[test]
    fn no_input_flag_is_process_local() {
        set_no_input(false);
        assert!(!no_input());
        set_no_input(true);
        assert!(no_input());
        set_no_input(false);
        assert!(!no_input());
    }
}
