//! Legacy outbox reading/deletion only. This fork never queues new reports.
use std::path::{Path, PathBuf};

use crate::{Consent, Item};

pub const MAX_FILES: usize = 200;
pub const MAX_BYTES: u64 = 2 * 1024 * 1024;

pub fn dir(home: &Path) -> PathBuf {
    crate::dir(home).join("outbox")
}

/// Reports are never queued, regardless of saved consent.
pub fn push(_home: &Path, _item: &Item) -> std::io::Result<bool> {
    Ok(false)
}

fn files(dir: &Path) -> Vec<(PathBuf, u64)> {
    let mut files: Vec<(PathBuf, u64)> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
        .map(|e| (e.path(), e.metadata().map_or(0, |m| m.len())))
        .collect();
    files.sort();
    files
}

/// The items waiting, oldest first, with their files. Unreadable files are removed.
pub fn pending(home: &Path) -> Vec<(PathBuf, Item)> {
    files(&dir(home))
        .into_iter()
        .filter_map(|(path, _)| {
            let item = std::fs::read(&path)
                .ok()
                .and_then(|b| serde_json::from_slice::<Item>(&b).ok());
            if item.is_none() {
                let _ = std::fs::remove_file(&path);
            }
            item.map(|i| (path, i))
        })
        .collect()
}

/// The waiting items the player still allows, oldest first; the others are deleted, never sent.
pub fn retain_allowed(home: &Path) -> Vec<(PathBuf, Item)> {
    let consent = Consent::load(home);
    pending(home)
        .into_iter()
        .filter(|(path, item)| {
            let allowed = consent.allows(item.kind());
            if !allowed {
                let _ = std::fs::remove_file(path);
            }
            allowed
        })
        .collect()
}

/// Removes every waiting item ("Delete my data", or everything turned off).
pub fn clear(home: &Path) -> std::io::Result<()> {
    match std::fs::remove_dir_all(dir(home)) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::payload::{Crash, Payload, Source};
    use crate::{Choices, test_home};

    #[test]
    fn saved_consent_cannot_reenable_collection() {
        let home = test_home::new("clean-outbox");
        let mut consent = Consent::load(&home);
        consent.answer(Choices::ALL);
        consent.save(&home).unwrap();
        let item = Item::new(Source::Game, "clean", Payload::Crash(Crash::default()));
        assert!(!push(&home, &item).unwrap());
        assert!(pending(&home).is_empty());
        assert!(!dir(&home).exists());
        std::fs::remove_dir_all(home).unwrap();
    }
}
