//! Explicit, camera-free import of a selected calibration into provider settings.
//! Publish a new private configuration; never replace an adopted profile/bundle.
use crate::{profile_store, provider};
use serde_json::{json, Value};
use std::{
    error::Error,
    ffi::CString,
    fs::{self, DirBuilder, OpenOptions},
    io::{self, Write},
    os::unix::{ffi::OsStrExt, fs::DirBuilderExt, fs::OpenOptionsExt},
    path::Path,
};

fn object(path: &Path) -> Result<Value, Box<dyn Error>> {
    if fs::metadata(path)?.len() > 8192 {
        return Err("provider configuration exceeds8KiB".into());
    }
    let value: Value = serde_json::from_slice(&fs::read(path)?)?;
    if !value.is_object() {
        return Err("expected a provider configuration object".into());
    }
    Ok(value)
}

fn write_new(path: &Path, value: &Value) -> io::Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    serde_json::to_writer_pretty(&mut file, value)?;
    file.write_all(b"\n")?;
    file.sync_all()
}

/// Preserve a destination that appears during preparation, including an empty
/// directory. Linux rename() alone may replace that directory despite a check.
fn publish_new(stage: &Path, output: &Path) -> io::Result<()> {
    let stage = CString::new(stage.as_os_str().as_bytes())?;
    let output = CString::new(output.as_os_str().as_bytes())?;
    let result = unsafe {
        libc::renameat2(
            libc::AT_FDCWD,
            stage.as_ptr(),
            libc::AT_FDCWD,
            output.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

pub fn prepare(
    session: &Path,
    settings: &Path,
    profile: &Path,
    output: &Path,
) -> Result<(), Box<dyn Error>> {
    if !output.is_absolute() || output.file_name().is_none() {
        return Err("choose an absolute new configuration directory".into());
    }
    if output.try_exists()? || output.is_symlink() {
        return Err("preserve the existing configuration; choose a new directory".into());
    }
    // Reuse exactly the live provider's admission, including matching gesture,
    // cursor, workspace and visual-macro policies. This initializes no inference.
    let (config, _, _) = provider::configured(session, settings, None)?;
    if !config.provider_absolute_pointer {
        return Err("calibration import requires the absolute provider".into());
    }
    let selected = profile_store::load(profile)?;
    if config
        .pointer_output
        .is_some_and(|size| size != selected.output)
    {
        return Err("calibration output differs from the selected output".into());
    }
    let mut session_value = object(session)?;
    let mut settings_value = object(settings)?;
    let workspace = serde_json::to_value(selected.workspace)?;
    session_value["touch_workspace"] = workspace.clone();
    session_value["pointer_output"] = json!(selected.output);
    settings_value["workspace"] = workspace;
    let mut name = output.as_os_str().to_os_string();
    name.push(format!(".new-{}", std::process::id()));
    let stage = std::path::PathBuf::from(name);
    DirBuilder::new().mode(0o700).create(&stage)?;
    let result = (|| {
        write_new(&stage.join("session.json"), &session_value)?;
        write_new(&stage.join("settings.json"), &settings_value)?;
        write_new(
            &stage.join("calibration.json"),
            &serde_json::to_value(selected)?,
        )?;
        // Validate the actual serialized pair, not just the input templates.
        provider::configured(
            &stage.join("session.json"),
            &stage.join("settings.json"),
            None,
        )?;
        let report = json!({"schema":"wigglecontrol.provider-setup/v1",
            "prepared":true,"camera_opened":false,"system_input":false,
            "inference_initialized":false,"installed":false,
            "profile":"calibration.json","session":"session.json","settings":"settings.json",
            "output":selected.output,"workspace":selected.workspace});
        write_new(&stage.join("setup.json"), &report)?;
        fs::File::open(&stage)?.sync_all()?;
        publish_new(&stage, output)?;
        fs::File::open(output.parent().unwrap())?.sync_all()?;
        // No local paths, camera identifiers or recording locations in stdout.
        println!("{report}");
        Ok::<(), Box<dyn Error>>(())
    })();
    if stage.exists() {
        fs::remove_dir_all(stage)?;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publication_preserves_a_destination_created_after_preparation() {
        let root =
            std::env::temp_dir().join(format!("wiggle-setup-publish-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        let stage = root.join("stage");
        let output = root.join("output");
        fs::create_dir(&stage).unwrap();
        fs::write(stage.join("session.json"), "complete").unwrap();
        fs::create_dir(&output).unwrap();
        assert_eq!(
            publish_new(&stage, &output).unwrap_err().kind(),
            io::ErrorKind::AlreadyExists
        );
        assert!(stage.join("session.json").exists());
        assert!(fs::read_dir(output).unwrap().next().is_none());
        fs::remove_dir_all(root).unwrap();
    }
}
