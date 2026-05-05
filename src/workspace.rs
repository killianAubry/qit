// Workspace folder picker (native) and Wasm stub.
//
// `rfd` only makes sense on native targets; Wasm builds get a no-op with a
// status message.
//
// Folder picker still uses sync `rfd::FileDialog`; `.qasm` open uses
// `rfd::AsyncFileDialog` in `app.rs` (polled each frame) so we never block the
// egui/winit loop — blocking sync dialogs broke ⌘O after the first open on macOS.

use crate::state::{AppState, StatusKind};

#[cfg(not(target_arch = "wasm32"))]
pub fn pick_workspace_folder(state: &mut AppState) {
    let dialog = rfd::FileDialog::new()
        .set_title("qsim — workspace folder")
        .set_directory(state.workspace_dir.clone());

    if let Some(path) = dialog.pick_folder() {
        state.workspace_dir = path;

        let circuit_path = state.circuit_file_path();
        if circuit_path.is_file() {
            match state.load_circuit_file() {
                Ok(()) => {
                    let fname = circuit_path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("circuit");
                    state.ui.flash(format!("loaded {fname}"), StatusKind::Ok);
                }
                Err(e) => state.ui.flash(e, StatusKind::Err),
            }
        } else {
            let fname = circuit_path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("circuit");
            state.ui.flash(
                format!(
                    "workspace: {}  (⌘S saves {fname})",
                    state.workspace_dir.display(),
                ),
                StatusKind::Info,
            );
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub fn pick_workspace_folder(state: &mut AppState) {
    state.ui.flash(
        "workspace folder: available on desktop builds only",
        StatusKind::Err,
    );
}
