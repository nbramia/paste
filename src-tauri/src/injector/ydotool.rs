use super::{Injector, InjectorError};
use log::debug;
use std::process::Command;

/// Injects text via ydotool (Wayland -- universal, requires ydotoold).
pub struct YdotoolInjector;

impl Injector for YdotoolInjector {
    fn inject_text(&self, text: &str) -> Result<(), InjectorError> {
        // The persistent virtual keyboard types in milliseconds; spawning
        // ydotool takes hundreds, long enough for the user's next keys to
        // land inside an expansion. ydotool remains the fallback.
        if super::virtual_keyboard::type_text(text) {
            debug!("ydotool: typed {} chars via virtual keyboard", text.len());
            return Ok(());
        }

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

        // Prefer the persistent virtual keyboard: it reaches native Wayland
        // windows. xdotool is the fallback when uinput is unavailable; it
        // reaches XWayland windows only.
        if super::virtual_keyboard::backspaces(count) {
            debug!("ydotool: sent {} backspaces via virtual keyboard", count);
            return Ok(());
        }

        debug!("ydotool: sending {} backspaces via xdotool", count);
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

    fn inject_rich(
        &self,
        content: &super::clipboard_inject::RichContent,
    ) -> Result<(), InjectorError> {
        use super::clipboard_inject::clipboard_inject_rich_wayland;
        clipboard_inject_rich_wayland(content, "ydotool")
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
