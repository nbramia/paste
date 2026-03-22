use std::process::Command;
use log::debug;
use super::{Injector, InjectorError};

/// Injects text via ydotool (Wayland -- universal, requires ydotoold).
pub struct YdotoolInjector;

impl Injector for YdotoolInjector {
    fn inject_text(&self, text: &str) -> Result<(), InjectorError> {
        debug!("ydotool: injecting {} chars", text.len());

        let status = Command::new("ydotool")
            .args(["type", "--", text])
            .status()?;

        if !status.success() {
            return Err(InjectorError::Failed(format!(
                "ydotool type exited with {}. Is ydotoold running?",
                status
            )));
        }

        Ok(())
    }

    fn inject_via_clipboard(&self, text: &str) -> Result<(), InjectorError> {
        use super::clipboard_inject::clipboard_inject_wayland;
        clipboard_inject_wayland(text, "ydotool")
    }

    fn send_backspaces(&self, count: usize) -> Result<(), InjectorError> {
        if count == 0 {
            return Ok(());
        }

        debug!("ydotool: sending {} backspaces", count);

        for _ in 0..count {
            let status = Command::new("ydotool")
                .args(["key", "14:1", "14:0"]) // KEY_BACKSPACE press and release
                .status()?;

            if !status.success() {
                return Err(InjectorError::Failed("ydotool key backspace failed".into()));
            }
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        "ydotool"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ydotool_name() {
        assert_eq!(YdotoolInjector.name(), "ydotool");
    }
}
