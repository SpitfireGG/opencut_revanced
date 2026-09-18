// SPDX-License-Identifier: AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Jareer and Concat contributors

//! What the window asks of the platform it runs on.
//!
//! Concat ships on Android. The Linux desktop build is kept for
//! development - the tests run on it, and at phone width it shows the phone
//! layout in seconds - and it differs from the phone in four things: how the
//! backend is chosen, how a file or folder is picked, whether the window
//! has a title strip of its own to drag, and whether a file dragged in from
//! outside the window can happen. Everything else in the crate is the same
//! tree, the same state and the same callbacks, so the differences live
//! here and nowhere else.
//!
//! Linux draws through the winit backend; Android through Slint's
//! android-activity backend, which the activity sets up before [`crate::run`]
//! is called. File dialogs are Linux's: on the phone a pick goes through
//! the system's picker. A drag in from outside is Linux's too, and X11's
//! only: winit reports no `DroppedFile` on Wayland.

use std::path::PathBuf;

use slint::PlatformError;
// The winit backend, and so these, exist everywhere but Android, which
// draws through Slint's android-activity backend and has no winit at all.
#[cfg(not(target_os = "android"))]
use slint::winit_030::winit::event::WindowEvent;
#[cfg(not(target_os = "android"))]
use slint::winit_030::winit::event_loop::ActiveEventLoop;
#[cfg(not(target_os = "android"))]
use slint::winit_030::winit::window::{Window as WinitWindow, WindowId};
#[cfg(not(target_os = "android"))]
use slint::winit_030::{CustomApplicationHandler, EventResult};

use crate::gpu::Gpu;

/// Collects the paths of a single OS drag as `DroppedFile` events deliver
/// them one at a time, then hands the whole batch to `on_dropped` once
/// winit says this pass over the event queue is done - the same shape a
/// picked-files dialog hands the caller, so the caller need not know drag
/// and drop split it up.
#[cfg(not(target_os = "android"))]
struct DropHandler {
    pending: Vec<PathBuf>,
    on_dropped: Box<dyn Fn(Vec<PathBuf>)>,
}

#[cfg(not(target_os = "android"))]
impl DropHandler {
    fn new(on_dropped: impl Fn(Vec<PathBuf>) + 'static) -> Self {
        Self {
            pending: Vec::new(),
            on_dropped: Box::new(on_dropped),
        }
    }
}

#[cfg(not(target_os = "android"))]
impl CustomApplicationHandler for DropHandler {
    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        _winit_window: Option<&WinitWindow>,
        _slint_window: Option<&slint::Window>,
        event: &WindowEvent,
    ) -> EventResult {
        if let WindowEvent::DroppedFile(path) = event {
            self.pending.push(path.clone());
        }
        EventResult::Propagate
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) -> EventResult {
        if !self.pending.is_empty() {
            (self.on_dropped)(std::mem::take(&mut self.pending));
        }
        EventResult::Propagate
    }
}

/// Chooses and installs the backend, and hands back the device the
/// renderer and the engine's compositor share, when there is one.
///
/// `on_files_dropped` fires on the event-loop thread with the paths of a
/// file (or several) dragged in from outside the window - a file manager -
/// batched into one call per drag. It is taken here,
/// before the window exists, because the backend - and the hook into its
/// event loop that OS drops arrive through - has to be selected before
/// anything is built on top of it; see [`DropHandler`].
#[cfg(not(target_os = "android"))]
pub fn select_backend(
    on_files_dropped: impl Fn(Vec<PathBuf>) + 'static,
) -> Result<Option<Gpu>, PlatformError> {
    // The device the renderer and the monitor share. Taken first, because
    // the backend is selected with it.
    let gpu = Gpu::acquire();
    if gpu.is_none() {
        log::warn!("no GPU adapter; the monitor composites on the CPU");
    }

    let mut selector = slint::BackendSelector::new()
        .backend_name("winit".into())
        .with_winit_custom_application_handler(DropHandler::new(on_files_dropped));
    selector = match &gpu {
        Some(gpu) => selector.require_wgpu_29(gpu.configuration()),
        None => selector,
    };

    // The custom title bar: the window draws its own strip, with its own
    // minimise, maximise and close, so the platform's decorations go.
    selector =
        selector.with_winit_window_attributes_hook(|attributes| attributes.with_decorations(false));
    selector.select()?;
    Ok(gpu)
}

/// Minimises the window: the strip's first button.
pub fn minimize(window: &slint::Window) {
    #[cfg(not(target_os = "android"))]
    {
        use slint::winit_030::WinitWindowAccessor;
        window.with_winit_window(|window| {
            window.set_minimized(true);
        });
    }
    #[cfg(target_os = "android")]
    let _ = window;
}

/// Whether the window is currently maximised, for the strip to pick the
/// maximise or the restore glyph. False where there is no such state.
pub fn is_maximized(window: &slint::Window) -> bool {
    #[cfg(not(target_os = "android"))]
    {
        use slint::winit_030::WinitWindowAccessor;
        window
            .with_winit_window(|window| window.is_maximized())
            .unwrap_or(false)
    }
    #[cfg(target_os = "android")]
    {
        let _ = window;
        false
    }
}

/// On Android the activity installed the backend before calling in. The
/// shared device is opened here all the same - the monitor composites on it,
/// as on the desktop, and a frame is a texture the window samples instead of
/// pixels the CPU blended - and `run` hands it to the window once the window
/// exists. Android has no OS drag to wire up - a file arrives through the
/// picker instead - so `on_files_dropped` is taken only to keep the
/// signature the same as the desktop's and is never called.
#[cfg(target_os = "android")]
pub fn select_backend(
    on_files_dropped: impl Fn(Vec<PathBuf>) + 'static,
) -> Result<Option<Gpu>, PlatformError> {
    let _ = on_files_dropped;
    let gpu = Gpu::acquire();
    log::info!(
        "preview: {}",
        if gpu.is_some() {
            "composited on the GPU"
        } else {
            "no GPU device; composited on the CPU"
        }
    );
    Ok(gpu)
}

/// Starts a window drag from the title strip.
pub fn begin_drag(window: &slint::Window) {
    #[cfg(not(target_os = "android"))]
    {
        use slint::winit_030::WinitWindowAccessor;
        window.with_winit_window(|window| {
            let _ = window.drag_window();
        });
    }
    #[cfg(target_os = "android")]
    let _ = window;
}

/// Maximises the window, or restores it: the title strip's double-click.
pub fn toggle_maximize(window: &slint::Window) {
    #[cfg(not(target_os = "android"))]
    {
        use slint::winit_030::WinitWindowAccessor;
        window.with_winit_window(|window| {
            window.set_maximized(!window.is_maximized());
        });
    }
    #[cfg(target_os = "android")]
    let _ = window;
}

/// Asks for a folder, starting at `start` when there is one.
pub fn pick_folder(title: &str, start: &str) -> Option<PathBuf> {
    #[cfg(not(target_os = "android"))]
    {
        let mut dialog = rfd::FileDialog::new().set_title(title);
        if !start.is_empty() {
            dialog = dialog.set_directory(start);
        }
        dialog.pick_folder()
    }
    #[cfg(target_os = "android")]
    {
        let _ = (title, start);
        None
    }
}

/// What a phone does when asked for files: shows the system's picker and
/// calls back, later, with what was chosen - an empty list for nothing.
/// Installed by the phone's own crate before the window runs; see
/// [`install_file_picker`]. The flag asks for the photo and video gallery
/// rather than the document picker.
#[cfg(target_os = "android")]
pub type FilePicker = Box<dyn Fn(bool, Box<dyn FnOnce(Vec<PathBuf>) + Send>) + Send + Sync>;

#[cfg(target_os = "android")]
static FILE_PICKER: std::sync::OnceLock<FilePicker> = std::sync::OnceLock::new();

/// A short tick from the phone's vibration motor, installed by the phone's
/// own crate; see [`install_haptic`].
#[cfg(target_os = "android")]
/// `true` asks for a firm one.
pub type Haptic = Box<dyn Fn(bool) + Send + Sync>;

#[cfg(target_os = "android")]
static HAPTIC: std::sync::OnceLock<Haptic> = std::sync::OnceLock::new();

/// Installs what [`haptic_tick`] does. Once; a second call is ignored.
#[cfg(target_os = "android")]
pub fn install_haptic(tick: Haptic) {
    let _ = HAPTIC.set(tick);
}

/// A tick you feel: a dragged picture has landed on the frame's centre.
/// Nothing on a desktop.
pub fn haptic_tick() {
    #[cfg(target_os = "android")]
    if let Some(tick) = HAPTIC.get() {
        tick(false);
    }
}

/// A firm buzz: the playhead has come to the end of the timeline and
/// stopped there. Nothing on a desktop.
pub fn haptic_firm() {
    #[cfg(target_os = "android")]
    if let Some(tick) = HAPTIC.get() {
        tick(true);
    }
}

/// Installs the picker a phone answers [`pick_files_async`] with. Once;
/// a second call is ignored.
#[cfg(target_os = "android")]
pub fn install_file_picker(picker: FilePicker) {
    let _ = FILE_PICKER.set(picker);
}

/// Asks for files and calls `on_picked` with them, on whichever thread the
/// platform answers from - the caller hops to the window's thread itself.
///
/// On a desktop the dialog blocks and the callback runs before this
/// returns. On a phone the system's picker is another screen: this returns
/// at once and the callback comes when the picker is dismissed, through
/// the picker the phone's crate installed. `filter` is the desktop
/// dialog's; a phone's picker offers every kind of media on its own.
pub fn pick_files_async(
    title: &str,
    filter: Option<(&str, &[&str])>,
    on_picked: impl FnOnce(Vec<PathBuf>) + Send + 'static,
) {
    pick_async(title, filter, false, on_picked);
}

/// [`pick_files_async`] for pictures and footage: on a phone, the system's
/// gallery picker. The same file dialog on a desktop.
pub fn pick_media_async(title: &str, on_picked: impl FnOnce(Vec<PathBuf>) + Send + 'static) {
    pick_async(title, None, true, on_picked);
}

fn pick_async(
    title: &str,
    filter: Option<(&str, &[&str])>,
    gallery: bool,
    on_picked: impl FnOnce(Vec<PathBuf>) + Send + 'static,
) {
    #[cfg(not(target_os = "android"))]
    {
        let _ = gallery;
        if let Some(paths) = pick_files(title, filter) {
            on_picked(paths);
        }
    }
    #[cfg(target_os = "android")]
    {
        let _ = (title, filter);
        match FILE_PICKER.get() {
            Some(picker) => picker(gallery, Box::new(on_picked)),
            None => log::warn!("no file picker on this platform yet"),
        }
    }
}

/// Asks for files. `filter` names a family and its extensions, and limits
/// the dialog to them.
pub fn pick_files(title: &str, filter: Option<(&str, &[&str])>) -> Option<Vec<PathBuf>> {
    #[cfg(not(target_os = "android"))]
    {
        let mut dialog = rfd::FileDialog::new().set_title(title);
        if let Some((name, extensions)) = filter {
            dialog = dialog.add_filter(name, extensions);
        }
        dialog.pick_files()
    }
    #[cfg(target_os = "android")]
    {
        let _ = (title, filter);
        None
    }
}

/// Shows a written file in the platform's file manager.
///
/// A phone has no file manager to hand a path to, and says so rather than
/// appearing to work: a control that silently does nothing is worse than one
/// that explains itself. The path is in the message, which is the part a
/// developer on a cable can still use.
pub fn reveal(path: &str) -> Result<(), String> {
    #[cfg(not(target_os = "android"))]
    {
        opener::reveal(path).map_err(|error| error.to_string())
    }
    #[cfg(target_os = "android")]
    {
        Err(format!(
            "this device has no file manager to open {path} with"
        ))
    }
}
