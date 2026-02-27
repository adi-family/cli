use std::path::Path;
use std::process::{Child, Command, Stdio};

/// Run `tailwindcss` once (production build with minification).
/// Returns `false` if the binary is not found.
pub fn build(crate_dir: &Path) -> anyhow::Result<bool> {
    let mut cmd = tailwind_cmd(crate_dir);
    cmd.arg("--minify");

    match cmd.status() {
        Ok(status) => {
            anyhow::ensure!(status.success(), "tailwindcss build failed (exit {status})");
            tracing::info!("Tailwind CSS built");
            Ok(true)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            tracing::warn!("tailwindcss not found — skipping CSS build");
            Ok(false)
        }
        Err(e) => Err(e.into()),
    }
}

/// Spawn `tailwindcss --watch` as a background child process.
/// Returns `None` if the binary is not found.
pub fn watch(crate_dir: &Path) -> anyhow::Result<Option<Child>> {
    let mut cmd = tailwind_cmd(crate_dir);
    cmd.arg("--watch");

    match cmd.spawn() {
        Ok(child) => {
            tracing::info!("Tailwind CSS watcher started (pid {})", child.id());
            Ok(Some(child))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            tracing::warn!("tailwindcss not found — skipping CSS watcher");
            Ok(None)
        }
        Err(e) => Err(e.into()),
    }
}

fn tailwind_cmd(crate_dir: &Path) -> Command {
    let input = crate_dir.join("static/input.css");
    let output = crate_dir.join("static/style.css");

    let mut cmd = Command::new("tailwindcss");
    cmd.current_dir(crate_dir)
        .arg("-i").arg(input)
        .arg("-o").arg(output)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    cmd
}
