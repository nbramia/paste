use std::process::Command;
use log::debug;
use super::{Injector, InjectorError};

/// Injects text via xdotool (X11).
pub struct XdotoolInjector;

impl Injector for XdotoolInjector {
    fn inject_text(&self, text: &str) -> Result<(), InjectorError> {
        debug!("xdotool: injecting {} chars", text.len());

        let status = Command::new("xdotool")
            .args(["type", "--clearmodifiers", "--", text])
            .status()?;

        if !status.success() {
            return Err(InjectorError::Failed(format!(
                "xdotool type exited with {}",
                status
            )));
        }

        Ok(())
    }

    fn inject_via_clipboard(&self, text: &str) -> Result<(), InjectorError> {
        use super::clipboard_inject::clipboard_inject_x11;
        clipboard_inject_x11(text)
    }

    fn send_backspaces(&self, count: usize) -> Result<(), InjectorError> {
        if count == 0 {
            return Ok(());
        }

        debug!("xdotool: sending {} backspaces", count);

        for _ in 0..count {
            let status = Command::new("xdotool")
                .args(["key", "--clearmodifiers", "BackSpace"])
                .status()?;

            if !status.success() {
                return Err(InjectorError::Failed("xdotool key BackSpace failed".into()));
            }
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        "xdotool"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xdotool_name() {
        assert_eq!(XdotoolInjector.name(), "xdotool");
    }
}
