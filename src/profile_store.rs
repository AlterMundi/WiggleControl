//! Receiving-local profile adoption with an atomic previous-profile rollback.
use crate::calibration::Profile;
use std::{
    fs::{self, OpenOptions},
    io::{self, Read, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
};
fn previous(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(".previous");
    PathBuf::from(name)
}
pub fn load(path: &Path) -> io::Result<Profile> {
    let mut file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)?;
    let meta = file.metadata()?;
    if !meta.is_file() || meta.len() > 4096 || meta.uid() != unsafe { libc::geteuid() } {
        return Err(io::Error::other("invalid profile ownership/type/size"));
    }
    let mut bytes = Vec::new();
    Read::by_ref(&mut file).take(4097).read_to_end(&mut bytes)?;
    if bytes.len() > 4096 {
        return Err(io::Error::other("oversized profile"));
    }
    let profile: Profile = serde_json::from_slice(&bytes)?;
    profile.validate().map_err(io::Error::other)?;
    Ok(profile)
}
fn atomic(path: &Path, profile: Profile) -> io::Result<()> {
    profile.validate().map_err(io::Error::other)?;
    let mut temp = path.as_os_str().to_os_string();
    temp.push(format!(".new-{}", std::process::id()));
    let temp = PathBuf::from(temp);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(&temp)?;
    let result = (|| {
        serde_json::to_writer(&mut file, &profile)?;
        file.write_all(b"\n")?;
        file.sync_all()?;
        fs::rename(&temp, path)?;
        fs::File::open(
            path.parent()
                .filter(|parent| !parent.as_os_str().is_empty())
                .unwrap_or(Path::new(".")),
        )?
        .sync_all()
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}
pub fn adopt(path: &Path, profile: Profile) -> io::Result<()> {
    profile.validate().map_err(io::Error::other)?;
    match load(path) {
        Ok(old) => atomic(&previous(path), old)?,
        Err(e) if e.kind() == io::ErrorKind::NotFound => (),
        Err(e) => return Err(e),
    }
    atomic(path, profile)
}
pub fn previous_profile(path: &Path) -> io::Result<Profile> {
    load(&previous(path))
}
pub fn rollback(path: &Path) -> io::Result<Profile> {
    let profile = load(&previous(path))?;
    adopt(path, profile)?;
    Ok(profile)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{touch_gestures::Workspace, Point};
    use std::os::unix::fs::{symlink, PermissionsExt};
    fn profile(min: f64) -> Profile {
        Profile {
            version: 1,
            workspace: Workspace {
                min: Point { x: min, y: min },
                max: Point { x: 0.9, y: 0.9 },
            },
            output: [1920, 1080],
        }
    }
    #[test]
    fn adoption_rollback_invalid_profiles_and_symlinks_preserve_originals() {
        let dir = std::env::temp_dir().join(format!("wiggle-profile-test-{}", std::process::id()));
        fs::create_dir(&dir).unwrap();
        let path = dir.join("profile.json");
        adopt(&path, profile(0.1)).unwrap();
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        adopt(&path, profile(0.2)).unwrap();
        assert_eq!(load(&previous(&path)).unwrap().workspace.min.x, 0.1);
        let mut bad = profile(0.3);
        bad.version = 9;
        assert!(adopt(&path, bad).is_err());
        assert_eq!(load(&path).unwrap().workspace.min.x, 0.2);
        assert_eq!(rollback(&path).unwrap().workspace.min.x, 0.1);
        assert_eq!(load(&previous(&path)).unwrap().workspace.min.x, 0.2);
        let link = dir.join("link.json");
        symlink(&path, &link).unwrap();
        assert!(adopt(&link, profile(0.3)).is_err());
        assert_eq!(load(&path).unwrap().workspace.min.x, 0.1);
        fs::write(&path, b"broken").unwrap();
        assert!(adopt(&path, profile(0.3)).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"broken");
        fs::remove_dir_all(dir).unwrap();
    }
}
