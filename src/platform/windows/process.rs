//! Owned worker-process shutdown; never a host-wide process operation.
use std::{
    io,
    process::Child,
    thread,
    time::{Duration, Instant},
};
pub(crate) const CREATE_NO_WINDOW: u32 = 0x0800_0000;
pub(crate) fn terminate(child: &mut Child) -> io::Result<()> {
    let kill = child.kill();
    let start = Instant::now();
    loop {
        if child.try_wait()?.is_some() {
            return Ok(());
        }
        if let Err(error) = &kill {
            return Err(io::Error::other(format!(
                "owned child kill failed: {error}"
            )));
        }
        if start.elapsed() >= Duration::from_secs(1) {
            return Err(io::Error::other(
                "owned child exit not observed within one second",
            ));
        }
        thread::sleep(Duration::from_millis(20));
    }
}
