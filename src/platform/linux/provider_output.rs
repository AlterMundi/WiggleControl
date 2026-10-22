//! Existing independent Linux controller writer and exit policy.
use crate::session_config::SessionConfig;
use std::{
    io,
    process::{Command, Stdio},
};
pub(crate) fn controller_writer(
    config: &SessionConfig,
    seconds: Option<u64>,
) -> io::Result<std::process::Child> {
    let mut args = match seconds {
        Some(seconds) => vec!["controller-stream".to_owned(), seconds.to_string()],
        None => vec!["controller-stream-until-stop".to_owned()],
    };
    if config.provider_absolute_pointer {
        args[0] = if seconds.is_some() {
            "controller-absolute-stream"
        } else {
            "controller-absolute-stream-until-stop"
        }
        .into();
    }
    args.extend([config.gain.to_string(), config.freshness_ms.to_string()]);
    Command::new(std::env::current_exe()?)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
}
pub(crate) fn check_writer_exit(
    status: Option<std::process::ExitStatus>,
    interrupted: bool,
) -> io::Result<()> {
    use std::os::unix::process::ExitStatusExt;
    match status {
        None => Ok(()),
        Some(status)
            if status.success()
                || (interrupted
                    && matches!(status.signal(), Some(libc::SIGINT | libc::SIGTERM))) =>
        {
            Ok(())
        }
        Some(status) => Err(io::Error::other(format!("device writer exited: {status}"))),
    }
}
