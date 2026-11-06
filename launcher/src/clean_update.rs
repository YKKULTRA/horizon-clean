//! Horizon Clean from source (FORK.md, "Updates"): when the player presses Update, the newest
//! commit of this fork's `main` on GitHub is downloaded as GitHub's source snapshot, compiled
//! with the launcher's own Rust (`rust_toolchain`, the one native mods build with) in the release
//! profile, installed as its own version `clean-<commit>` and selected. Nothing checks or builds
//! by itself.
//!
//! The version folder holds what a release folder holds (the four programs, and the sources
//! xtask's `package` archives beside them) and [`RECORD`], naming the commit. The resources
//! prepared for another installed version are reused when its preparer makes the same ones
//! (`backend::install_folder`). Native mods download this repository's source and build on
//! these commits too (`native_mods::revision_of`).
use crate::backend;
use crate::instances;
use crate::native_mods::{self, Plan, Profile, Step};
use crate::rust_toolchain;
use std::fs;
use std::path::Path;
use std::process::Command;

/// This fork on GitHub: Update builds its [`BRANCH`], and native mods download its source.
pub const REPO: &str = "YKKULTRA/horizon-clean";
pub const BRANCH: &str = "main";
const PREFIX: &str = "clean-";
/// In a version folder Update made: what it was built from.
pub const RECORD: &str = "horizon-clean.json";
/// The programs of a release folder, built together (game-build.yml).
const PROGRAMS: [(&str, &str); 4] = [
    ("horizon-client", "horizon-client"),
    ("horizon-client", "horizon-ctl"),
    ("horizon-prepare", "horizon-prepare"),
    ("horizon-server", "horizon-server"),
];
/// The sources a release folder takes beside the programs (xtask's `ARCHIVED`), and the notices.
const ARCHIVED: [&str; 6] = [
    "tools",
    "mods/horizon/src/test/java",
    "crates/horizon-prepare",
    "research-lock.json",
    "LICENSE",
    "THIRD_PARTY_NOTICES.md",
];

/// How an Update ended: the version now selected, built or already there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    UpToDate(String),
    Built(String),
}

impl Outcome {
    pub fn version(&self) -> &str {
        match self {
            Self::UpToDate(v) | Self::Built(v) => v,
        }
    }
}

/// Whether `version` is a Horizon Clean build (Update's, or the portable package's).
pub fn is_clean(version: &str) -> bool {
    version.starts_with(PREFIX)
}

/// The version a build of `commit` installs as.
pub fn version_for(commit: &str) -> String {
    format!("{PREFIX}{}", &commit[..commit.len().min(12)])
}

fn is_commit(text: &str) -> bool {
    text.len() == 40 && text.bytes().all(|b| b.is_ascii_hexdigit())
}

/// The commit installed version `version` was built from (see [`commit_in`]).
pub fn commit_of(version: &str) -> Option<String> {
    commit_in(&backend::version_dir(version))
}

/// The commit a version folder was built from: its [`RECORD`], else, for the portable package
/// (PACKAGE_CLEAN.sh), the `clean fork <commit>` its `.installed` says.
pub fn commit_in(dir: &Path) -> Option<String> {
    let record = fs::read_to_string(dir.join(RECORD))
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok());
    if let Some(commit) = record
        .as_ref()
        .and_then(|r| r["commit"].as_str())
        .filter(|c| is_commit(c))
    {
        return Some(commit.to_owned());
    }
    let installed = fs::read_to_string(dir.join(".installed")).ok()?;
    let commit = installed
        .split("clean fork ")
        .nth(1)?
        .split_whitespace()
        .next()?;
    is_commit(commit).then(|| commit.to_owned())
}

/// The clean build installed last (its `.installed` is the newest), of `installed`.
pub fn newest_installed(installed: &[String]) -> Option<&String> {
    installed.iter().filter(|v| is_clean(v)).max_by_key(|v| {
        fs::metadata(backend::version_dir(v).join(".installed"))
            .and_then(|m| m.modified())
            .ok()
    })
}

/// The newest commit of [`BRANCH`] on GitHub.
pub fn latest() -> Result<String, String> {
    let url = format!("https://api.github.com/repos/{REPO}/commits/{BRANCH}");
    let commit: serde_json::Value = backend::get(&url)?.into_json().map_err(|e| e.to_string())?;
    commit["sha"]
        .as_str()
        .filter(|s| is_commit(s))
        .map(str::to_owned)
        .ok_or_else(|| format!("GitHub didn't name the newest commit of {REPO} {BRANCH}."))
}

/// Update: the newest commit of [`BRANCH`] built (once) and selected, with every instance that
/// played a clean build moved to it. `progress` hears each step.
pub fn update(profile: Profile, progress: impl Fn(Step)) -> Result<Outcome, String> {
    let started = std::time::Instant::now();
    let commit = latest()?;
    if let Some(have) = backend::installed()
        .into_iter()
        .find(|v| is_clean(v) && commit_of(v).as_deref() == Some(commit.as_str()))
    {
        follow(&have)?;
        return Ok(Outcome::UpToDate(have));
    }
    let version = version_for(&commit);
    fs::create_dir_all(native_mods::root()).map_err(|e| e.to_string())?;
    let _ = fs::write(native_mods::log_path(), "");
    rust_toolchain::linker()?;
    let rust = rust_toolchain::install(|f| progress(Step::Rust(f)))?;
    progress(Step::Source(commit.clone()));
    let source = native_mods::source(&commit)?;
    // One folder for every update, written only where the source changed: Cargo then rebuilds
    // only what the new commit changed.
    let stage = native_mods::root().join("clean");
    let before = commit_in(&stage);
    native_mods::sync(&source, &stage)?;
    let plan = match &before {
        Some(old) => Plan::Incremental(vec![format!("{} → {}", &old[..7], &commit[..7])]),
        None => Plan::First,
    };
    progress(Step::Plan(plan.clone(), profile));
    let target = native_mods::root().join("target");
    let (mut crates, mut total, mut name) = (0, None, String::new());
    native_mods::run(
        compile(&rust, &stage, &target, profile),
        "Compiling the game",
        |line| {
            for event in native_mods::parse_cargo(line) {
                match event {
                    native_mods::Cargo::Compiling(n) => {
                        crates += 1;
                        name = n;
                    }
                    native_mods::Cargo::Progress(done, all) => {
                        crates = crates.max(done);
                        total = Some(all);
                    }
                }
                if !name.is_empty() {
                    progress(Step::Compile {
                        done: crates,
                        total,
                        name: name.clone(),
                        profile,
                    });
                }
            }
        },
    )?;
    // The stage now says which commit it holds, for the next update's plan.
    fs::write(
        stage.join(RECORD),
        serde_json::json!({ "commit": commit }).to_string(),
    )
    .map_err(|e| e.to_string())?;
    progress(Step::Install);
    let folder = backend::version_dir(&format!(".{version}.partial"));
    let _ = fs::remove_dir_all(&folder);
    fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    let built = target.join(profile.name());
    for (_, program) in PROGRAMS {
        let file = format!("{program}{}", std::env::consts::EXE_SUFFIX);
        copy_program(&built.join(&file), &folder.join(&file))?;
    }
    for path in ARCHIVED {
        let from = stage.join(path);
        let to = folder.join(path);
        if from.is_dir() {
            backend::copy_dir(&from, &to).map_err(|e| format!("{path}: {e}"))?;
        } else if from.is_file() {
            if let Some(parent) = to.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            fs::copy(&from, &to).map_err(|e| format!("{path}: {e}"))?;
        }
    }
    let record = serde_json::json!({
        "repo": REPO,
        "branch": BRANCH,
        "commit": commit,
        "profile": profile.name(),
        "rust": rust_toolchain::VERSION,
    });
    fs::write(folder.join(RECORD), record.to_string()).map_err(|e| e.to_string())?;
    if let Err(e) = backend::install_folder(&version, &folder) {
        let _ = fs::remove_dir_all(&folder);
        return Err(e);
    }
    follow(&version)?;
    prune(&version);
    // The last update's snapshot is in the stage now; a native build needing it downloads it again.
    if let Some(old) = before.filter(|old| *old != commit) {
        let _ = fs::remove_dir_all(native_mods::root().join("source").join(old));
    }
    progress(Step::Done {
        secs: started.elapsed().as_secs(),
        profile,
        plan,
    });
    Ok(Outcome::Built(version))
}

/// The release folder's programs, compiled together into `target`.
fn compile(
    rust: &rust_toolchain::Toolchain,
    stage: &Path,
    target: &Path,
    profile: Profile,
) -> Command {
    let mut cmd = native_mods::cargo(rust, stage, "build");
    cmd.env("CARGO_TARGET_DIR", target)
        .env("CARGO_TERM_PROGRESS_WHEN", "always")
        .env("CARGO_TERM_PROGRESS_WIDTH", "100");
    cmd.args(["--locked", "--profile", profile.name()]);
    let mut package = "";
    for (crate_name, program) in PROGRAMS {
        if crate_name != package {
            cmd.args(["-p", crate_name]);
            package = crate_name;
        }
        cmd.args(["--bin", program]);
    }
    cmd
}

/// Copies a built program into the version folder, keeping it executable.
fn copy_program(from: &Path, to: &Path) -> Result<(), String> {
    fs::copy(from, to).map_err(|e| format!("The built {}: {e}", from.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(to, fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// `version` becomes the selected version, and every instance pinned to a clean build plays it.
/// Instances on a release, a Nightly or a native build keep theirs.
fn follow(version: &str) -> Result<(), String> {
    let mut settings = backend::Settings::load();
    settings.selected = Some(version.to_owned());
    settings.save();
    for instance in instances::list() {
        if instance
            .version
            .as_deref()
            .is_some_and(|v| is_clean(v) && v != version)
        {
            instances::edit(&instance.id, |meta| meta.version = Some(version.to_owned()))?;
        }
    }
    Ok(())
}

/// After an Update, unless the player turned pruning off: the clean builds but `keep` and the
/// one installed before it (to go back to) are deleted, except those an instance plays and the
/// portable package's own (a link to its folder, never deleted from here).
fn prune(keep: &str) {
    let settings = backend::Settings::load();
    if !settings.auto_prune() {
        return;
    }
    let installed = backend::installed();
    let mut clean: Vec<&String> = installed.iter().filter(|v| is_clean(v)).collect();
    clean.sort_by_key(|v| {
        std::cmp::Reverse(
            fs::metadata(backend::version_dir(v).join(".installed"))
                .and_then(|m| m.modified())
                .ok(),
        )
    });
    let pinned: Vec<String> = instances::list()
        .into_iter()
        .filter_map(|i| i.version)
        .collect();
    let previous = clean.iter().find(|v| v.as_str() != keep).cloned();
    for version in clean {
        let linked = fs::symlink_metadata(backend::version_dir(version))
            .is_ok_and(|m| m.file_type().is_symlink());
        if version == keep || Some(version) == previous || linked || pinned.contains(version) {
            continue;
        }
        match backend::remove(version) {
            Ok(()) => eprintln!("Removed old Horizon Clean build {version}."),
            Err(e) => eprintln!("Could not remove old Horizon Clean build {version}: {e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_name_their_commit() {
        let commit = "160f5ae5c96e6b79fba79d6b3c6937fd4e1c2f69";
        assert_eq!(version_for(commit), "clean-160f5ae5c96e");
        assert!(is_clean(&version_for(commit)));
        assert!(is_clean("clean-4aa6854"));
        assert!(!is_clean("nightly-2026-10-28-0c02067"));
        assert!(!is_clean("v0.2.97"));
        assert!(!is_clean("native-abc"));
    }

    #[test]
    fn a_folder_says_its_commit_by_record_or_by_the_packages_line() {
        let dir = std::env::temp_dir().join(format!(
            "horizon-clean-commit-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        assert_eq!(commit_in(&dir), None);
        let commit = "160f5ae5c96e6b79fba79d6b3c6937fd4e1c2f69";
        // The portable package (PACKAGE_CLEAN.sh): no record, the line in `.installed`.
        fs::write(
            dir.join(".installed"),
            format!("Source base: 4aa6854a886183440256f4f4a4a9d1ed853227a1; clean fork {commit}\n"),
        )
        .unwrap();
        assert_eq!(commit_in(&dir).as_deref(), Some(commit));
        // Update's record wins; a record without a full commit is ignored.
        let other = "5bcef93771529d45596a766bdd6702b2c23a6342";
        fs::write(dir.join(RECORD), format!(r#"{{"commit":"{other}"}}"#)).unwrap();
        assert_eq!(commit_in(&dir).as_deref(), Some(other));
        fs::write(dir.join(RECORD), r#"{"commit":"5bcef93"}"#).unwrap();
        assert_eq!(commit_in(&dir).as_deref(), Some(commit));
        // An empty `.installed` (a release's) names nothing.
        fs::remove_file(dir.join(RECORD)).unwrap();
        fs::write(dir.join(".installed"), "").unwrap();
        assert_eq!(commit_in(&dir), None);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_build_compiles_the_release_folders_programs() {
        let rust = rust_toolchain::Toolchain {
            dir: std::path::PathBuf::from("/rust"),
        };
        let cmd = compile(
            &rust,
            Path::new("/stage"),
            Path::new("/target"),
            Profile::Release,
        );
        let args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        let line = args.join(" ");
        assert!(
            line.ends_with(
                "build --manifest-path /stage/Cargo.toml --locked --profile release \
                 -p horizon-client --bin horizon-client --bin horizon-ctl \
                 -p horizon-prepare --bin horizon-prepare -p horizon-server --bin horizon-server"
            ),
            "{line}"
        );
        assert!(cmd
            .get_envs()
            .any(|(k, v)| k == "CARGO_TARGET_DIR" && v == Some("/target".as_ref())));
    }
}
