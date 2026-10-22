//! Manual-session sink preparation shared by platform runtimes.
//! Call only after the owning runtime's explicit Start/preparation checks.
/// Recording and native input are independent opt-ins. A selected recorder
/// failure aborts setup; a writer failure drops any already-created recorder.
pub fn prepare_acquisition<R, W, E>(
    recording: bool,
    input: bool,
    record: impl FnOnce() -> Result<R, E>,
    writer: impl FnOnce() -> Result<W, E>,
) -> Result<(Option<R>, Option<W>), E> {
    let frames = if recording { Some(record()?) } else { None };
    let input_writer = if input { Some(writer()?) } else { None };
    Ok((frames, input_writer))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn acquisition_starts_input_independently_of_optional_recording() {
        use std::cell::Cell;
        for recording in [false, true] {
            for input in [false, true] {
                let records = Cell::new(0);
                let writers = Cell::new(0);
                let result = prepare_acquisition(
                    recording,
                    input,
                    || {
                        records.set(records.get() + 1);
                        Ok::<_, &str>("frames")
                    },
                    || {
                        writers.set(writers.get() + 1);
                        Ok::<_, &str>("writer")
                    },
                )
                .unwrap();
                assert_eq!(records.get(), usize::from(recording));
                assert_eq!(
                    writers.get(),
                    usize::from(input),
                    "recording={recording}, input={input}"
                );
                assert_eq!(result.0.is_some(), recording);
                assert_eq!(result.1.is_some(), input);
            }
        }
    }

    #[test]
    fn acquisition_failure_preserves_error_and_does_not_start_input_after_recorder_failure() {
        use std::cell::Cell;
        let writers = Cell::new(0);
        let failed_record = prepare_acquisition(
            true,
            true,
            || Err::<(), _>("record failed"),
            || {
                writers.set(writers.get() + 1);
                Ok(())
            },
        );
        assert_eq!(failed_record.unwrap_err(), "record failed");
        assert_eq!(writers.get(), 0);
        let failed_input = prepare_acquisition(
            false,
            true,
            || -> Result<(), &str> { panic!("disabled recording factory ran") },
            || Err::<(), _>("writer failed"),
        );
        assert_eq!(failed_input.unwrap_err(), "writer failed");
    }
}
