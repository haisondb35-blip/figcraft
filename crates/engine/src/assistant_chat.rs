//! Save chat with file: the desktop app changes the bytes of a `.figcraft` file about to be
//! written, to put the assistant's conversation about it in (or take it out). Without a hook
//! (automation, figcraft-cli, the web app) a file is written as it is.

use std::sync::OnceLock;

/// The bytes to write for a file at a path instead of the ones given, or None to keep them.
pub type SaveHook = Box<dyn Fn(&str, &[u8]) -> Option<Vec<u8>> + Send + Sync>;

static HOOK: OnceLock<SaveHook> = OnceLock::new();

/// Sets the [`SaveHook`] (once per run).
pub fn set_hook(hook: SaveHook) {
    let _ = HOOK.set(hook);
}

/// `bytes` as the hook has them written to `path`.
pub(crate) fn hooked(path: &str, bytes: &[u8]) -> Option<Vec<u8>> {
    HOOK.get().and_then(|hook| hook(path, bytes))
}
