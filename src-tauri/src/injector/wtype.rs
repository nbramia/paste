use std::process::Command;
use log::debug;
use super::{Injector, InjectorError};

/// Injects text via wtype (Wayland -- wlroots compositors only: Sway, Hyprland, etc.).
pub struct WtypeInjector;

impl Injector for WtypeInjector {
    fn inject_text(&self, text: &str) -> Result<(), InjectorError> {
        debug!("wtype: injecting {} chars", text.len());

        let status = Command::new("wtype")
            .args(["--", text])
            .status()?;

        if !status.success() {
            return Err(InjectorError::Failed(format!(
                "wtype exited with {}",
                status
            )));
        }

        Ok(())
    }

    fn inject_via_clipboard(&self, text: &str) -> Result<(), InjectorError> {
        use super::clipboard_inject::clipboard_inject_wayland;
        clipboard_inject_wayland(text, "wtype")
    }

    fn send_backspaces(&self, count: usize) -> Result<(), InjectorError> {
        if count == 0 {
            return Ok(());
        }

        debug!("wtype: sending {} backspaces", count);

        for _ in 0..count {
            let status = Command::new("wtype")
                .args(["-k", "BackSpace"])
                .status()?;

            if !status.success() {
                return Err(InjectorError::Failed("wtype -k BackSpace failed".into()));
            }
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        "wtype"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wtype_name() {
        assert_eq!(WtypeInjector.name(), "wtype");
    }
}
