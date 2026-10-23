//! The size of the app's window when it opens (P-057).
//!
//! The window opens at 1280 × 800 (`tauri.conf.json`), wide enough for the
//! menu with its names and for the dense tables. On a screen where that
//! does not fit (a laptop at 125 % or 150 %), it opens maximized instead,
//! and if the person restores it, it comes back to a size that fits the
//! screen, never one larger than it. The app does not remember the size a
//! person left yet: each start is the same.

use tauri::{AppHandle, Manager, PhysicalSize, Runtime};

/// The inner size the window takes on a screen too small for the one it
/// opens with: the work area of the screen without the window's frame,
/// never below the smallest size the window allows. `None`: it fits as it
/// is. Every size in physical pixels.
pub(crate) fn fitted_size(
    inner: (u32, u32),
    outer: (u32, u32),
    area: (u32, u32),
    smallest: (u32, u32),
) -> Option<(u32, u32)> {
    if outer.0 <= area.0 && outer.1 <= area.1 {
        return None;
    }
    let frame = (outer.0.saturating_sub(inner.0), outer.1.saturating_sub(inner.1));
    let width = area.0.saturating_sub(frame.0).min(inner.0).max(smallest.0);
    let height = area.1.saturating_sub(frame.1).min(inner.1).max(smallest.1);
    Some((width, height))
}

/// The smallest inner size of the window, as the configuration says
/// (logical pixels): `minWidth` and `minHeight` of `tauri.conf.json`.
const SMALLEST: (f64, f64) = (800.0, 500.0);

/// Opens the main window maximized when its size does not fit the screen it
/// is on, with a size that fits for when the person restores it.
pub(crate) fn fit_main_window<R: Runtime>(app: &AppHandle<R>) {
    // `get_window`: with the view of a judicial portal inside it, the main
    // window has two webviews.
    let Some(window) = app.get_window("main") else {
        return;
    };
    let (Ok(Some(monitor)), Ok(inner), Ok(outer)) =
        (window.current_monitor(), window.inner_size(), window.outer_size())
    else {
        return;
    };
    let area = monitor.work_area().size;
    let scale = monitor.scale_factor();
    let smallest = ((SMALLEST.0 * scale).round() as u32, (SMALLEST.1 * scale).round() as u32);
    let Some((width, height)) = fitted_size(
        (inner.width, inner.height),
        (outer.width, outer.height),
        (area.width, area.height),
        smallest,
    ) else {
        return;
    };
    let _ = window.set_size(PhysicalSize::new(width, height));
    let _ = window.center();
    let _ = window.maximize();
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn a_window_that_fits_the_screen_keeps_its_size() {
        assert_eq!(fitted_size((1280, 800), (1296, 839), (1920, 1032), (800, 500)), None);
    }

    #[test]
    fn a_window_larger_than_the_screen_takes_its_work_area_without_the_frame() {
        // 1920 × 1080 at 150 %: a work area of 1920 × 1032 physical pixels,
        // the window of 1280 × 800 logical is 1920 × 1200 inside.
        let fitted = fitted_size((1920, 1200), (1944, 1259), (1920, 1032), (1200, 750));
        assert_eq!(fitted, Some((1896, 973)));
    }

    #[test]
    fn a_fitted_window_is_never_smaller_than_the_smallest_size() {
        assert_eq!(fitted_size((1280, 800), (1296, 839), (1024, 520), (800, 500)), Some((1008, 500)));
    }

    fn first_window(config: &str) -> serde_json::Map<String, Value> {
        let config: Value = serde_json::from_str(config).expect("the configuration is JSON");
        config["app"]["windows"][0].as_object().expect("a window").clone()
    }

    /// The configuration for Windows replaces the whole list of windows of
    /// the general one (that is how Tauri joins them): without its own size
    /// the window opened at Tauri's 800 × 600.
    #[test]
    fn the_window_opens_with_the_same_size_on_every_system() {
        let general = first_window(include_str!("../tauri.conf.json"));
        let windows = first_window(include_str!("../tauri.windows.conf.json"));
        for key in ["width", "height", "minWidth", "minHeight", "center"] {
            assert!(general.contains_key(key), "the general window has no {key}");
            assert_eq!(general.get(key), windows.get(key), "{key} differs on Windows");
        }
        assert_eq!(general["width"], 1280);
        assert_eq!(general["height"], 800);
        assert_eq!(general["minWidth"].as_f64(), Some(SMALLEST.0));
        assert_eq!(general["minHeight"].as_f64(), Some(SMALLEST.1));
    }
}
