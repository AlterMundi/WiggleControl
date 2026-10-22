//! Windows physical display discovery and SendInput virtual-desktop coordinates.
use crate::Point;
use serde::{Deserialize, Serialize};
use std::{io, mem::size_of};
use windows_sys::Win32::{
    Foundation::{BOOL, LPARAM, RECT},
    Graphics::Gdi::{EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFOEXW},
    UI::{
        HiDpi::{
            SetThreadDpiAwarenessContext, DPI_AWARENESS_CONTEXT,
            DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        },
        WindowsAndMessaging::{
            GetSystemMetrics, MONITORINFOF_PRIMARY, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN,
            SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
        },
    },
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub width: u32,
    pub height: u32,
}
impl Rect {
    pub fn validate(self) -> io::Result<()> {
        if !(1..=65_536).contains(&self.width)
            || !(1..=65_536).contains(&self.height)
            || i64::from(self.left) + i64::from(self.width) - 1 > i64::from(i32::MAX)
            || i64::from(self.top) + i64::from(self.height) - 1 > i64::from(i32::MAX)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid physical display bounds",
            ));
        }
        Ok(())
    }
    fn contains(self, other: Self) -> bool {
        other.left >= self.left
            && other.top >= self.top
            && i64::from(other.left) + i64::from(other.width)
                <= i64::from(self.left) + i64::from(self.width)
            && i64::from(other.top) + i64::from(other.height)
                <= i64::from(self.top) + i64::from(self.height)
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Monitor {
    pub id: String,
    pub bounds: Rect,
    pub primary: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Desktop {
    pub bounds: Rect,
    pub monitors: Vec<Monitor>,
}
#[derive(Clone, Copy, Debug)]
pub struct Mapping {
    desktop: Rect,
    target: Rect,
}
impl Mapping {
    pub fn new(desktop: Rect, target: Rect) -> io::Result<Self> {
        desktop.validate()?;
        target.validate()?;
        if !desktop.contains(target) {
            return Err(io::Error::other(
                "target monitor is outside virtual desktop",
            ));
        }
        Ok(Self { desktop, target })
    }
    pub fn output_size(self) -> [u32; 2] {
        [self.target.width, self.target.height]
    }
    pub fn absolute(self, point: Point) -> io::Result<(i32, i32)> {
        if !point.valid() {
            return Err(io::Error::other(
                "invalid normalized Windows pointer position",
            ));
        }
        let axis = |p: f64, start: i32, span: u32, desktop_start: i32, desktop_span: u32| {
            let physical = f64::from(start) + p * f64::from(span - 1);
            ((physical - f64::from(desktop_start)) * 65535.0 / f64::from((desktop_span - 1).max(1)))
                .round() as i32
        };
        Ok((
            axis(
                point.x,
                self.target.left,
                self.target.width,
                self.desktop.left,
                self.desktop.width,
            ),
            axis(
                point.y,
                self.target.top,
                self.target.height,
                self.desktop.top,
                self.desktop.height,
            ),
        ))
    }
}
impl Desktop {
    pub fn mapping(&self, id: &str) -> io::Result<Mapping> {
        let selected = self
            .monitors
            .iter()
            .find(|m| m.id == id)
            .ok_or_else(|| io::Error::other("selected Windows monitor is unavailable"))?;
        Mapping::new(self.bounds, selected.bounds)
    }
    /// Read only. Temporarily use physical DPI coordinates on this thread and
    /// restore its prior context; never change layout, resolution or scaling.
    pub fn read() -> io::Result<Self> {
        struct Dpi(DPI_AWARENESS_CONTEXT);
        impl Drop for Dpi {
            fn drop(&mut self) {
                unsafe {
                    SetThreadDpiAwarenessContext(self.0);
                }
            }
        }
        // SAFETY: supported Windows11 thread context; the prior handle is restored.
        let previous =
            unsafe { SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
        if previous.is_null() {
            return Err(io::Error::last_os_error());
        }
        let _context = Dpi(previous);
        let bounds = Rect {
            left: unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) },
            top: unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) },
            width: unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) }
                .try_into()
                .map_err(|_| io::Error::other("invalid virtual desktop width"))?,
            height: unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) }
                .try_into()
                .map_err(|_| io::Error::other("invalid virtual desktop height"))?,
        };
        bounds.validate()?;
        struct Collector {
            monitors: Vec<Monitor>,
            failure: Option<io::Error>,
        }
        unsafe extern "system" fn collect(
            monitor: HMONITOR,
            _dc: HDC,
            _rect: *mut RECT,
            data: LPARAM,
        ) -> BOOL {
            // SAFETY: EnumDisplayMonitors invokes synchronously with our live collector.
            let output = unsafe { &mut *(data as *mut Collector) };
            if output.monitors.len() >= 32 {
                output.failure = Some(io::Error::other("too many displays"));
                return 0;
            }
            let mut info: MONITORINFOEXW = unsafe { std::mem::zeroed() };
            info.monitorInfo.cbSize = size_of::<MONITORINFOEXW>() as u32;
            if unsafe { GetMonitorInfoW(monitor, (&mut info as *mut MONITORINFOEXW).cast()) } == 0 {
                output.failure = Some(io::Error::last_os_error());
                return 0;
            }
            let rect = info.monitorInfo.rcMonitor;
            let convert = || -> io::Result<Rect> {
                let rect = Rect {
                    left: rect.left,
                    top: rect.top,
                    width: (i64::from(rect.right) - i64::from(rect.left))
                        .try_into()
                        .map_err(|_| io::Error::other("invalid monitor width"))?,
                    height: (i64::from(rect.bottom) - i64::from(rect.top))
                        .try_into()
                        .map_err(|_| io::Error::other("invalid monitor height"))?,
                };
                rect.validate()?;
                Ok(rect)
            };
            match convert() {
                Ok(bounds) => output.monitors.push(Monitor {
                    id: String::from_utf16_lossy(
                        &info.szDevice[..info
                            .szDevice
                            .iter()
                            .position(|c| *c == 0)
                            .unwrap_or(info.szDevice.len())],
                    ),
                    bounds,
                    primary: info.monitorInfo.dwFlags & MONITORINFOF_PRIMARY != 0,
                }),
                Err(error) => {
                    output.failure = Some(error);
                    return 0;
                }
            }
            1
        }
        let mut output = Collector {
            monitors: Vec::new(),
            failure: None,
        };
        // SAFETY: no HDC/clip, a synchronous callback, and a live collector pointer.
        let success = unsafe {
            EnumDisplayMonitors(
                std::ptr::null_mut(),
                std::ptr::null(),
                Some(collect),
                &mut output as *mut Collector as LPARAM,
            )
        };
        if let Some(error) = output.failure {
            return Err(error);
        }
        if success == 0 {
            return Err(io::Error::last_os_error());
        }
        if output.monitors.is_empty() {
            return Err(io::Error::other("no active Windows display"));
        }
        let desktop = Self {
            bounds,
            monitors: output.monitors,
        };
        for monitor in &desktop.monitors {
            desktop.mapping(&monitor.id)?;
        }
        Ok(desktop)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn maps_the_selected_display_in_a_negative_origin_desktop() {
        let map = Mapping::new(
            Rect {
                left: -100,
                top: 0,
                width: 200,
                height: 100,
            },
            Rect {
                left: 0,
                top: 0,
                width: 100,
                height: 100,
            },
        )
        .unwrap();
        assert_eq!(map.absolute(Point { x: 0.0, y: 0.0 }).unwrap(), (32932, 0));
        assert_eq!(
            map.absolute(Point { x: 1.0, y: 1.0 }).unwrap(),
            (65535, 65535)
        );
        assert_eq!(
            map.absolute(Point { x: 0.5, y: 0.5 }).unwrap(),
            (49234, 32768)
        );
        assert_eq!(map.output_size(), [100, 100]);
    }
    #[test]
    fn rejects_unavailable_targets_bad_bounds_and_nonfinite_points() {
        let bounds = Rect {
            left: 0,
            top: 0,
            width: 1920,
            height: 1080,
        };
        assert!(Mapping::new(bounds, Rect { left: -1, ..bounds }).is_err());
        assert!(Mapping::new(bounds, Rect { width: 0, ..bounds }).is_err());
        assert!(Mapping::new(
            bounds,
            Rect {
                left: i32::MAX,
                width: 2,
                ..bounds
            }
        )
        .is_err());
        let map = Mapping::new(bounds, bounds).unwrap();
        for point in [
            Point {
                x: f64::NAN,
                y: 0.0,
            },
            Point { x: 1.1, y: 0.5 },
        ] {
            assert!(map.absolute(point).is_err());
        }
        assert_eq!(map.absolute(Point { x: 0.0, y: 0.0 }).unwrap(), (0, 0));
        assert_eq!(
            map.absolute(Point { x: 1.0, y: 1.0 }).unwrap(),
            (65535, 65535)
        );
    }
}
