use crate::errors::*;
use console::Emoji;
use indicatif::{ProgressBar, ProgressStyle};
use std::io::Write;
use std::path::Path;
use std::process::Command;

// Some useful emojis
pub static SUCCESS: Emoji<'_, '_> = Emoji("✅", "success!");
pub static FAILURE: Emoji<'_, '_> = Emoji("❌", "failed!");

/// Runs the given command conveniently, returning the exit code. Notably, this parses the given command by separating it on spaces.
/// Returns the command's output and the exit code.
pub fn run_cmd(cmd: String, dir: &Path, pre_dump: impl Fn()) -> Result<(String, String, i32)> {
    // let mut cmd_args: Vec<&str> = raw_cmd.split(' ').collect();
    // let cmd = cmd_args.remove(0);

    // We run the command in a shell so that NPM/Yarn binaries can be recognized (see #5)
    #[cfg(unix)]
    let shell_exec = "sh";
    #[cfg(windows)]
    let shell_exec = "powershell";
    #[cfg(unix)]
    let shell_param = "-c";
    #[cfg(windows)]
    let shell_param = "-command";

    // This will NOT pipe output/errors to the console
    let output = Command::new(shell_exec)
        .args([shell_param, &cmd])
        .current_dir(dir)
        .output()
        .map_err(|err| ErrorKind::CmdExecFailed(cmd.clone(), err.to_string()))?;

    let exit_code = match output.status.code() {
        Some(exit_code) => exit_code,         // If we have an exit code, use it
        None if output.status.success() => 0, // If we don't, but we know the command succeeded, return 0 (success code)
        None => 1, // If we don't know an exit code but we know that the command failed, return 1 (general error code)
    };

    // Print `stderr` only if there's something therein and the exit code is non-zero
    if !output.stderr.is_empty() && exit_code != 0 {
        pre_dump();
        std::io::stderr().write_all(&output.stderr).unwrap();
    }

    Ok((
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
        exit_code,
    ))
}

/// Runs a series of commands and provides a nice spinner with a custom message. Returns the last command's output and an appropriate exit
/// code (0 if everything worked, otherwise the exit code of the one that failed).
pub fn run_stage(cmds: Vec<&str>, target: &Path, message: String) -> Result<(String, String, i32)> {
    // Tell the user about the stage with a nice progress bar
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(ProgressStyle::default_spinner().tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ "));
    spinner.set_message(format!("{}...", message));
    // Tick the spinner every 50 milliseconds
    spinner.enable_steady_tick(50);

    let mut last_output = (String::new(), String::new());
    // Run the commands
    for cmd in cmds {
        // We make sure all commands run in the target directory ('.perseus/' itself)
        let (stdout, stderr, exit_code) = run_cmd(cmd.to_string(), target, || {
            // We're done, we'll write a more permanent version of the message
            spinner.finish_with_message(format!("{}...{}", message, FAILURE))
        })?;
        last_output = (stdout, stderr);
        // If we have a non-zero exit code, we should NOT continue (stderr has been written to the console already)
        if exit_code != 0 {
            return Ok((last_output.0, last_output.1, 1));
        }
    }

    // We're done, we'll write a more permanent version of the message
    spinner.finish_with_message(format!("{}...{}", message, SUCCESS));

    Ok((last_output.0, last_output.1, 0))
}
