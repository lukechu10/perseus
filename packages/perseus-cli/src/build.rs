use crate::cmd::run_stage;
use crate::errors::*;
use console::{style, Emoji};
use std::env;
use std::fs;
use std::path::PathBuf;

// Emojis for stages
static GENERATING: Emoji<'_, '_> = Emoji("🔨", "");
static BUILDING: Emoji<'_, '_> = Emoji("🏗️ ", ""); // Yes, there's a space here, for some reason it's needed...
static FINALIZING: Emoji<'_, '_> = Emoji("📦", "");

/// Returns the exit code if it's non-zero.
macro_rules! handle_exit_code {
    ($code:expr) => {
        let (_, _, code) = $code;
        if code != 0 {
            return Ok(code);
        }
    };
}

/// Actually builds the user's code, program arguments having been interpreted. This needs to know how many steps there are in total
/// because the serving logic also uses it.
pub fn build_internal(dir: PathBuf, num_steps: u8) -> Result<i32> {
    let mut target = dir;
    target.extend([".perseus"]);

    // Static generation
    handle_exit_code!(run_stage(
        vec![&format!(
            "{} run",
            env::var("PERSEUS_CARGO_PATH").unwrap_or_else(|_| "cargo".to_string())
        )],
        &target,
        format!(
            "{} {} Generating your app",
            style(format!("[1/{}]", num_steps)).bold().dim(),
            GENERATING
        )
    )?);
    // WASM building
    handle_exit_code!(run_stage(
        vec![&format!(
            "{} build --target web",
            env::var("PERSEUS_WASM_PACK_PATH").unwrap_or_else(|_| "wasm-pack".to_string())
        )],
        &target,
        format!(
            "{} {} Building your app to WASM",
            style(format!("[2/{}]", num_steps)).bold().dim(),
            BUILDING
        )
    )?);
    // Move the `pkg/` directory into `dist/pkg/`
    let pkg_dir = target.join("dist/pkg");
    if pkg_dir.exists() {
        if let Err(err) = fs::remove_dir_all(&pkg_dir) {
            bail!(ErrorKind::MovePkgDirFailed(err.to_string()));
        }
    }
    // The `fs::rename()` function will fail on Windows if the destination already exists, so this should work (we've just deleted it as per https://github.com/rust-lang/rust/issues/31301#issuecomment-177117325)
    if let Err(err) = fs::rename(target.join("pkg"), target.join("dist/pkg")) {
        bail!(ErrorKind::MovePkgDirFailed(err.to_string()));
    }
    // JS bundle generation
    handle_exit_code!(run_stage(
        vec![&format!(
            "{} main.js --format iife --file dist/pkg/bundle.js",
            env::var("PERSEUS_ROLLUP_PATH").unwrap_or_else(|_| "rollup".to_string())
        )],
        &target,
        format!(
            "{} {} Finalizing bundle",
            style(format!("[3/{}]", num_steps)).bold().dim(),
            FINALIZING
        )
    )?);

    Ok(0)
}

/// Builds the subcrates to get a directory that we can serve. Returns an exit code.
pub fn build(dir: PathBuf, prog_args: &[String]) -> Result<i32> {
    // TODO support watching files
    // If we should watch for file changes, do so
    let should_watch = prog_args.get(1);
    let dflt_watch_path = ".".to_string();
    let _watch_path = prog_args.get(2).unwrap_or(&dflt_watch_path);
    if should_watch == Some(&"-w".to_string()) || should_watch == Some(&"--watch".to_string()) {
        todo!("watching not yet supported, try a tool like 'entr'");
    }
    let exit_code = build_internal(dir.clone(), 3)?;

    Ok(exit_code)
}
