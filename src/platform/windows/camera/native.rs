//! Media Foundation calls stay in the explicit camera child. Tests never call them.
use super::{invalid, Format, Spec, MAX_JPEG_BYTES};
use ::windows::{
    core::{GUID, PWSTR},
    Win32::{Media::MediaFoundation::*, System::Com::*},
};
use std::{cell::Cell, io, marker::PhantomData, ptr, rc::Rc};
fn error(e: impl std::fmt::Display) -> io::Error {
    io::Error::other(format!("Media Foundation: {e}"))
}
struct Context(PhantomData<Rc<()>>);
impl Context {
    fn new() -> io::Result<Self> {
        // SAFETY: child/metadata calling thread owns its balanced MTA and MF scope.
        unsafe {
            CoInitializeEx(None, COINIT_MULTITHREADED)
                .ok()
                .map_err(error)?;
            if let Err(e) = MFStartup(MF_VERSION, MFSTARTUP_FULL) {
                CoUninitialize();
                return Err(error(e));
            }
        }
        Ok(Self(PhantomData))
    }
}
impl Drop for Context {
    fn drop(&mut self) {
        unsafe {
            let _ = MFShutdown();
            CoUninitialize();
        }
    }
}
fn attributes() -> io::Result<IMFAttributes> {
    let mut value = None;
    unsafe {
        MFCreateAttributes(&mut value, 4).map_err(error)?;
    }
    value.ok_or_else(|| invalid("MF returned no attributes"))
}
struct Allocated(PWSTR);
impl Drop for Allocated {
    fn drop(&mut self) {
        unsafe {
            CoTaskMemFree(Some(self.0 .0.cast()));
        }
    }
}
fn string(attributes: &IMFAttributes, key: &GUID) -> io::Result<String> {
    let mut pointer = PWSTR::null();
    let mut length = 0;
    let result = unsafe { attributes.GetAllocatedString(key, &mut pointer, &mut length) };
    let value = Allocated(pointer);
    result.map_err(error)?;
    if value.0.is_null() || length == 0 || length > 4096 {
        return Err(invalid("invalid camera metadata string"));
    }
    // SAFETY: MF allocated at least length UTF16 characters; owner frees afterward.
    String::from_utf16(unsafe { std::slice::from_raw_parts(value.0 .0, length as usize) })
        .map_err(error)
}
struct Activations {
    pointer: *mut Option<IMFActivate>,
    length: u32,
}
impl Drop for Activations {
    fn drop(&mut self) {
        if !self.pointer.is_null() {
            // SAFETY: array/count are returned together by MFEnumDeviceSources.
            unsafe {
                for item in std::slice::from_raw_parts_mut(self.pointer, self.length as usize) {
                    item.take();
                }
                CoTaskMemFree(Some(self.pointer.cast()));
            }
        }
    }
}
fn activations() -> io::Result<Activations> {
    let attrs = attributes()?;
    unsafe {
        attrs
            .SetGUID(
                &MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE,
                &MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_GUID,
            )
            .map_err(error)?;
    }
    let mut list = Activations {
        pointer: ptr::null_mut(),
        length: 0,
    };
    unsafe {
        MFEnumDeviceSources(&attrs, &mut list.pointer, &mut list.length).map_err(error)?;
    }
    if list.length > 64 || (list.length > 0 && list.pointer.is_null()) {
        return Err(invalid("invalid/oversized camera activation list"));
    }
    Ok(list)
}
impl Activations {
    fn items(&self) -> &[Option<IMFActivate>] {
        if self.length == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(self.pointer, self.length as usize) }
        }
    }
}
#[derive(serde::Serialize)]
pub struct Device {
    pub id: String,
    pub name: String,
}
/// Metadata enumeration only: never ActivateObject, ReadSample or render frames.
pub fn devices() -> io::Result<Vec<Device>> {
    let _context = Context::new()?;
    let list = activations()?;
    let mut result = Vec::new();
    for activate in list.items().iter().flatten() {
        result.push(Device {
            id: string(
                activate,
                &MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_SYMBOLIC_LINK,
            )?,
            name: string(activate, &MF_DEVSOURCE_ATTRIBUTE_FRIENDLY_NAME)?,
        });
    }
    Ok(result)
}
fn format(media: &IMFMediaType) -> io::Result<Option<Format>> {
    unsafe {
        if media.GetGUID(&MF_MT_SUBTYPE).map_err(error)? != MFVideoFormat_MJPG {
            return Ok(None);
        }
        let size = media.GetUINT64(&MF_MT_FRAME_SIZE).map_err(error)?;
        let rate = media.GetUINT64(&MF_MT_FRAME_RATE).map_err(error)?;
        let value = Format {
            resolution: [(size >> 32) as u32, size as u32],
            frame_rate: [(rate >> 32) as u32, rate as u32],
        };
        Ok(value.validate().ok().map(|_| value))
    }
}
struct Source {
    media: IMFMediaSource,
    closed: Cell<bool>,
}
impl Source {
    fn close(&self) -> io::Result<()> {
        if !self.closed.get() {
            unsafe {
                self.media.Shutdown().map_err(error)?;
            }
            self.closed.set(true);
        }
        Ok(())
    }
}
impl Drop for Source {
    fn drop(&mut self) {
        let _ = self.close();
    }
}
pub(super) struct Reader {
    reader: IMFSourceReader,
    source: Source,
    pub(super) format: Format,
    _context: Context,
}
impl Reader {
    pub(super) fn open(spec: &Spec) -> io::Result<Self> {
        let context = Context::new()?;
        let list = activations()?;
        let mut selected = None;
        for activate in list.items().iter().flatten() {
            if string(
                activate,
                &MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_SYMBOLIC_LINK,
            )? == spec.camera
            {
                selected = Some(activate.clone());
                break;
            }
        }
        let activate = selected
            .ok_or_else(|| invalid("selected camera is unavailable; no replacement was opened"))?;
        // Only this explicit Start boundary activates the chosen camera source.
        let source = Source {
            media: unsafe { activate.ActivateObject::<IMFMediaSource>().map_err(error)? },
            closed: Cell::new(false),
        };
        let attrs = attributes()?;
        unsafe {
            attrs
                .SetUINT32(&MF_READWRITE_DISABLE_CONVERTERS, 1)
                .map_err(error)?;
            attrs.SetUINT32(&MF_LOW_LATENCY, 1).map_err(error)?;
        }
        let reader =
            unsafe { MFCreateSourceReaderFromMediaSource(&source.media, &attrs).map_err(error)? };
        let video = MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32;
        let mut chosen = None;
        for index in 0..256 {
            let media = match unsafe { reader.GetNativeMediaType(video, index) } {
                Ok(value) => value,
                Err(e) if e.code() == MF_E_NO_MORE_TYPES => break,
                Err(e) => return Err(error(e)),
            };
            if format(&media)?.is_some_and(|f| f.matches(&spec.format)) {
                chosen = Some(media);
                break;
            }
        }
        let media=chosen.ok_or_else(||invalid("selected native MJPEG resolution/frame rate is unavailable; no crop or conversion enabled"))?;
        unsafe {
            reader
                .SetStreamSelection(MF_SOURCE_READER_ALL_STREAMS.0 as u32, false)
                .map_err(error)?;
            reader.SetStreamSelection(video, true).map_err(error)?;
            reader
                .SetCurrentMediaType(video, None, &media)
                .map_err(error)?;
        }
        let actual = format(&unsafe { reader.GetCurrentMediaType(video).map_err(error)? })?
            .ok_or_else(|| invalid("camera did not retain native MJPEG"))?;
        if !actual.matches(&spec.format) {
            return Err(invalid("camera changed the requested native format"));
        }
        Ok(Self {
            reader,
            source,
            format: actual,
            _context: context,
        })
    }
    pub(super) fn next(&mut self) -> io::Result<Option<(u64, Vec<u8>)>> {
        let mut sample = None;
        let mut flags = 0;
        unsafe {
            self.reader
                .ReadSample(
                    MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32,
                    0,
                    None,
                    Some(&mut flags),
                    None,
                    Some(&mut sample),
                )
                .map_err(error)?;
        }
        let fatal = MF_SOURCE_READERF_ERROR.0
            | MF_SOURCE_READERF_ENDOFSTREAM.0
            | MF_SOURCE_READERF_NEWSTREAM.0
            | MF_SOURCE_READERF_NATIVEMEDIATYPECHANGED.0
            | MF_SOURCE_READERF_CURRENTMEDIATYPECHANGED.0;
        if flags & fatal as u32 != 0 {
            return Err(invalid(format!(
                "camera ended or changed format; source flags={flags}"
            )));
        }
        let Some(sample) = sample else {
            return Ok(None);
        };
        // MF's driver timestamp shares the QPC epoch and is expressed in100ns.
        // GetSampleTime is stream-relative and cannot replace this attribute.
        let stamp = unsafe {
            sample
                .GetUINT64(&MFSampleExtension_DeviceTimestamp)
                .map_err(|e| error(format!("camera has no driver/QPC timestamp: {e}")))?
        };
        let buffer = unsafe { sample.ConvertToContiguousBuffer().map_err(error)? };
        let mut pointer = ptr::null_mut();
        let mut length = 0;
        unsafe {
            buffer
                .Lock(&mut pointer, None, Some(&mut length))
                .map_err(error)?;
        }
        let bytes = if pointer.is_null() || !(4..=MAX_JPEG_BYTES as u32).contains(&length) {
            Err(invalid("invalid native MJPEG buffer length"))
        } else {
            // SAFETY: successful Lock returns length readable bytes until Unlock.
            Ok(unsafe { std::slice::from_raw_parts(pointer, length as usize) }.to_vec())
        };
        let unlock = unsafe { buffer.Unlock().map_err(error) };
        let bytes = bytes?;
        unlock?;
        Ok(Some((stamp, bytes)))
    }
    pub(super) fn close(&self) -> io::Result<()> {
        self.source.close()
    }
}
