use std::{ffi::OsStr, process::Command};

pub fn run<I, S>(color: bool, args: I) -> Result<String, std::io::Error>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut cmd = Command::new("task");
    cmd.arg("rc.verbose:no");
    cmd.arg("rc._forcecolor:on");

    if !color {
        cmd.arg("rc.color.active=none");
    }

    let output = cmd.arg("rc.detection:off").args(args).output()?;

    let output_string = String::from_utf8_lossy(&output.stdout);
    Ok(String::from(output_string.trim()))
}

pub fn task_count(report: &String) -> usize {
    return report.lines().count() - 1;
}
