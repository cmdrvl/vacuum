use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[allow(dead_code)]
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);
#[allow(dead_code)]
static WRAPPER_BINARY: OnceLock<String> = OnceLock::new();

#[allow(dead_code)]
pub fn vacuum_command(label: &str) -> Command {
    let root = unique_root(label);
    let home = root.join("home");
    write_healthy_guard_hooks(&home);

    let mut command = Command::new(env!("CARGO_BIN_EXE_vacuum"));
    command.env("HOME", &home);
    command.env("USERPROFILE", root.join("profile"));
    command
}

#[allow(dead_code)]
pub fn vacuum_binary() -> &'static str {
    WRAPPER_BINARY.get_or_init(create_vacuum_wrapper).as_str()
}

pub fn write_guard_hooks(home: &Path, dcg_command: &str) {
    let settings_path = home.join(".claude/settings.json");
    let veil_path = home.join("bin/veil");
    create_executable(&veil_path);
    if let Some(parent) = settings_path.parent() {
        fs::create_dir_all(parent).expect("settings parent should be creatable");
    }
    fs::write(
        settings_path,
        serde_json::json!({
            "hooks": {
                "PreToolUse": [
                    { "matcher": "Read", "hooks": [{ "type": "command", "command": veil_path.display().to_string() }] },
                    { "matcher": "Grep", "hooks": [{ "type": "command", "command": veil_path.display().to_string() }] },
                    {
                        "matcher": "Bash",
                        "hooks": [
                            { "type": "command", "command": veil_path.display().to_string() },
                            { "type": "command", "command": dcg_command }
                        ]
                    }
                ]
            }
        })
        .to_string(),
    )
    .expect("settings should be writable");
}

pub fn write_healthy_guard_hooks(home: &Path) {
    let dcg_path = home.join("bin/dcg");
    create_executable(&dcg_path);
    write_guard_hooks(home, &dcg_path.display().to_string());
}

#[allow(dead_code)]
fn unique_root(label: &str) -> PathBuf {
    let id = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "vacuum-integration-{label}-{}-{id}",
        std::process::id()
    ));
    fs::create_dir_all(&root).expect("temporary root should be creatable");
    root
}

#[allow(dead_code)]
fn create_vacuum_wrapper() -> String {
    let root = unique_root("golden-rules");
    let home = root.join("home");
    write_healthy_guard_hooks(&home);

    let wrapper_path = root.join("vacuum-wrapper");
    fs::write(
        &wrapper_path,
        format!(
            "#!/bin/sh\nHOME={} USERPROFILE={} exec {} \"$@\"\n",
            shell_quote(&home.display().to_string()),
            shell_quote(&root.join("profile").display().to_string()),
            shell_quote(env!("CARGO_BIN_EXE_vacuum")),
        ),
    )
    .expect("wrapper should be writable");
    make_executable(&wrapper_path);
    wrapper_path.display().to_string()
}

fn create_executable(path: &Path) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("executable parent should be creatable");
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .expect("test executable should be writable");
    file.write_all(b"#!/bin/sh\nexit 0\n")
        .expect("test executable should be writable");
    make_executable(path);
}

fn make_executable(path: &Path) {
    #[cfg(unix)]
    {
        let mut permissions = fs::metadata(path)
            .expect("test executable should exist")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions)
            .expect("test executable permissions should be writable");
    }
}

#[allow(dead_code)]
fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r#"'\''"#))
}
