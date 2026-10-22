//! Windows profile-file mechanics; validation/adoption/rollback are common policy.
use std::{
    fs::{File, OpenOptions},
    io,
    os::windows::{
        ffi::OsStrExt,
        fs::{MetadataExt, OpenOptionsExt},
    },
    path::Path,
};
use windows_sys::Win32::Storage::FileSystem::{
    MoveFileExW, FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_OPEN_REPARSE_POINT,
    MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
};
pub(crate) fn open_read(path: &Path) -> io::Result<File> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)?;
    if file.metadata()?.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(io::Error::other("profile reparse points are not supported"));
    }
    Ok(file)
}
pub(crate) fn create_new(path: &Path) -> io::Result<File> {
    // Receiving state belongs in the current user's local application-data directory;
    // new files inherit its ACL. No Unix modes or ownership values are fabricated.
    OpenOptions::new().write(true).create_new(true).open(path)
}
fn wide(path: &Path) -> io::Result<Vec<u16>> {
    let mut value: Vec<u16> = path.as_os_str().encode_wide().collect();
    if value.contains(&0) {
        return Err(io::Error::other("profile path contains NUL"));
    }
    value.push(0);
    Ok(value)
}
pub(crate) fn replace(source: &Path, destination: &Path) -> io::Result<()> {
    let source = wide(source)?;
    let destination = wide(destination)?;
    // SAFETY: both arguments are live NUL-terminated UTF-16 buffers. The temp
    // file is in the same directory; COPY_ALLOWED is intentionally absent.
    if unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
pub(crate) fn sync_parent(_path: &Path) -> io::Result<()> {
    // The file was flushed/closed before MoveFileExW(WRITE_THROUGH). Windows
    // does not expose the Unix directory-fsync operation used by the Linux adapter.
    Ok(())
}
