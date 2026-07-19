//! Process exit codes for script-friendly error handling.

use std::process::ExitCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AppExit {
    Success = 0,
    General = 1,
    Usage = 2,
    Trust = 3,
    Network = 4,
    Io = 5,
}

impl AppExit {
    pub fn code(self) -> ExitCode {
        ExitCode::from(self as u8)
    }
}

impl From<AppExit> for ExitCode {
    fn from(value: AppExit) -> Self {
        value.code()
    }
}

pub fn exit_for_message(message: &str) -> AppExit {
    let lower = message.to_lowercase();
    if lower.contains("trust")
        || lower.contains("untrusted")
        || lower.contains("blocked by trust")
        || lower.contains("signer")
    {
        AppExit::Trust
    } else if lower.contains("http ")
        || lower.contains("http request")
        || lower.contains("search api")
        || lower.contains("network")
        || lower.contains("timed out")
        || lower.contains("timeout")
    {
        AppExit::Network
    } else if lower.contains("usage")
        || lower.contains("invalid")
        || lower.contains("unknown")
        || lower.contains("must be")
        || lower.contains("must not")
        || lower.contains("requires")
        || lower.contains("expected one of")
        || lower.starts_with("`bmx ")
    {
        AppExit::Usage
    } else if lower.contains("failed reading")
        || lower.contains("failed writing")
        || lower.contains("failed to create")
        || lower.contains("failed to write")
        || lower.contains("no such file")
        || lower.contains("permission denied")
    {
        AppExit::Io
    } else {
        AppExit::General
    }
}

#[cfg(test)]
mod tests {
    use super::{AppExit, exit_for_message};

    #[test]
    fn exit_for_message_classifies_common_failures() {
        assert_eq!(exit_for_message("blocked by trust policy"), AppExit::Trust);
        assert_eq!(
            exit_for_message("HTTP request failed for https://example"),
            AppExit::Network
        );
        assert_eq!(exit_for_message("`bmx exec` requires APP"), AppExit::Usage);
        assert_eq!(exit_for_message("failed reading /tmp/x"), AppExit::Io);
        assert_eq!(exit_for_message("something else"), AppExit::General);
    }
}
