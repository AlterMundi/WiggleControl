// SPDX-License-Identifier: MIT
// Adapted from v4l 0.14.0, upstream revision
// 867d9b122fde603ff7f1bfcdf19de48845c9e1f7, src/io/mmap/{stream,arena}.rs.
// Copyright (c) 2020 Christopher N. Hesse.
// Original permission notice: packaging/v4l-mmap-MIT.txt.
//! Capture-only MMAP adapter: retry poll timeouts without double-QBUF.
mod arena;
use std::convert::TryInto;
use std::time::Duration;
use std::{io, mem, sync::Arc};

use self::arena::Arena;
use v4l::buffer::{Metadata, Type};
use v4l::device::{Device, Handle};
use v4l::io::traits::{CaptureStream, Stream as StreamTrait};
use v4l::memory::Memory;
use v4l::v4l2;
use v4l::v4l_sys::*;

/// Stream of mapped buffers
///
/// An arena instance is used internally for buffer handling.
pub struct Stream<'a> {
    handle: Arc<Handle>,
    arena: Arena<'a>,
    arena_index: usize,
    buf_type: Type,
    buf_meta: Vec<Metadata>,
    timeout: Option<i32>,

    active: bool,
    // True only after DQBUF returned this arena_index successfully.
    capture_dequeued: bool,
}

impl Stream<'_> {
    pub fn with_buffers(dev: &Device, buf_type: Type, buf_count: u32) -> io::Result<Self> {
        let mut arena = Arena::new(dev.handle(), buf_type);
        let count = arena.allocate(buf_count)?;
        let mut buf_meta = Vec::new();
        buf_meta.resize(count as usize, Metadata::default());

        Ok(Stream {
            handle: dev.handle(),
            arena,
            arena_index: 0,
            buf_type,
            buf_meta,
            active: false,
            capture_dequeued: false,
            timeout: None,
        })
    }

    /// Sets a timeout of the v4l file handle.
    pub fn set_timeout(&mut self, duration: Duration) {
        self.timeout = Some(duration.as_millis().try_into().unwrap());
    }

    fn buffer_desc(&self) -> v4l2_buffer {
        v4l2_buffer {
            type_: self.buf_type as u32,
            memory: Memory::Mmap as u32,
            ..unsafe { mem::zeroed() }
        }
    }
}

impl Drop for Stream<'_> {
    fn drop(&mut self) {
        if !self.active {
            return;
        }
        if let Err(e) = self.stop() {
            if let Some(code) = e.raw_os_error() {
                // ENODEV means the file descriptor wrapped in the handle became invalid, most
                // likely because the device was unplugged or the connection (USB, PCI, ..)
                // broke down. Handle this case gracefully by ignoring it.
                if code == 19 {
                    /* ignore */
                    return;
                }
            }

            panic!("{:?}", e)
        }
    }
}

impl StreamTrait for Stream<'_> {
    type Item = [u8];

    fn start(&mut self) -> io::Result<()> {
        unsafe {
            let mut typ = self.buf_type as u32;
            v4l2::ioctl(
                self.handle.fd(),
                v4l2::vidioc::VIDIOC_STREAMON,
                &mut typ as *mut _ as *mut std::os::raw::c_void,
            )?;
        }

        self.active = true;
        Ok(())
    }

    fn stop(&mut self) -> io::Result<()> {
        unsafe {
            let mut typ = self.buf_type as u32;
            v4l2::ioctl(
                self.handle.fd(),
                v4l2::vidioc::VIDIOC_STREAMOFF,
                &mut typ as *mut _ as *mut std::os::raw::c_void,
            )?;
        }

        self.active = false;
        Ok(())
    }
}

impl<'b> CaptureStream<'b> for Stream<'_> {
    fn queue(&mut self, index: usize) -> io::Result<()> {
        let mut v4l2_buf = v4l2_buffer {
            index: index as u32,
            ..self.buffer_desc()
        };

        unsafe {
            v4l2::ioctl(
                self.handle.fd(),
                v4l2::vidioc::VIDIOC_QBUF,
                &mut v4l2_buf as *mut _ as *mut std::os::raw::c_void,
            )?;
        }

        if index == self.arena_index {
            self.capture_dequeued = false;
        }
        Ok(())
    }

    fn dequeue(&mut self) -> io::Result<usize> {
        let mut v4l2_buf = self.buffer_desc();

        if self.handle.poll(libc::POLLIN, self.timeout.unwrap_or(-1))? == 0 {
            // This condition can only happen if there was a timeout.
            // A timeout is only possible if the `timeout` value is non-zero, meaning we should
            // propagate it to the caller.
            return Err(io::Error::new(io::ErrorKind::TimedOut, "VIDIOC_DQBUF"));
        }

        unsafe {
            v4l2::ioctl(
                self.handle.fd(),
                v4l2::vidioc::VIDIOC_DQBUF,
                &mut v4l2_buf as *mut _ as *mut std::os::raw::c_void,
            )?;
        }
        self.arena_index = v4l2_buf.index as usize;
        if self.arena_index >= self.buf_meta.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "camera buffer index out of range",
            ));
        }
        self.capture_dequeued = true;

        self.buf_meta[self.arena_index] = Metadata {
            bytesused: v4l2_buf.bytesused,
            flags: v4l2_buf.flags.into(),
            field: v4l2_buf.field,
            timestamp: v4l2_buf.timestamp.into(),
            sequence: v4l2_buf.sequence,
        };

        Ok(self.arena_index)
    }

    fn next(&'b mut self) -> io::Result<(&'b Self::Item, &'b Metadata)> {
        if !self.active {
            // Enqueue all buffers once on stream start
            for index in 0..self.arena.bufs.len() {
                CaptureStream::queue(self, index)?;
            }

            self.start()?;
        } else if self.capture_dequeued {
            CaptureStream::queue(self, self.arena_index)?;
        }

        self.arena_index = CaptureStream::dequeue(self)?;

        // The index used to access the buffer elements is given to us by v4l2, so we assume it
        // will always be valid.
        let bytes = &self.arena.bufs[self.arena_index];
        let meta = &self.buf_meta[self.arena_index];
        Ok((bytes, meta))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

    // A real pipe gives bounded poll timeouts and rejects video ioctls. No
    // camera, mmap buffer or fake syscall is involved. This recreates the
    // state after STREAMON, before the first successful DQBUF (or after a
    // previous buffer was requeued). The next operation must be poll, not QBUF.
    struct WaitingStream {
        stream: Stream<'static>,
        _writer: OwnedFd,
    }
    impl WaitingStream {
        fn new(index: usize) -> Self {
            let mut fds = [-1; 2];
            assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
            let reader = unsafe { OwnedFd::from_raw_fd(fds[0]) };
            let writer = unsafe { OwnedFd::from_raw_fd(fds[1]) };
            let handle = Device::with_path(format!("/proc/self/fd/{}", reader.as_raw_fd()))
                .unwrap()
                .handle();
            Self {
                stream: Stream {
                    handle: handle.clone(),
                    arena: Arena::new(handle, Type::VideoCapture),
                    arena_index: index,
                    buf_type: Type::VideoCapture,
                    buf_meta: vec![Metadata::default(); 2],
                    timeout: Some(0),
                    active: true,
                    capture_dequeued: false,
                },
                _writer: writer,
            }
        }
    }
    impl Drop for WaitingStream {
        fn drop(&mut self) {
            // The fixture never started an actual video stream. Its empty arena
            // and both real pipe descriptors are released by normal field Drop.
            self.stream.active = false;
        }
    }
    #[test]
    fn repeated_poll_timeouts_never_requeue_a_driver_owned_buffer() {
        for index in [0, 1] {
            let mut fixture = WaitingStream::new(index);
            for _ in 0..3 {
                let error = match fixture.stream.next() {
                    Err(error) => error,
                    Ok(_) => panic!("An empty pipe cannot supply a camera frame"),
                };
                assert_eq!(error.kind(), io::ErrorKind::TimedOut);
                assert!(!fixture.stream.capture_dequeued);
                assert!(fixture.stream.active);
            }
        }
    }
    #[test]
    fn a_dequeued_buffer_requires_queue_and_failed_queue_keeps_ownership() {
        let mut fixture = WaitingStream::new(1);
        fixture.stream.capture_dequeued = true;
        let error = match fixture.stream.next() {
            Err(error) => error,
            Ok(_) => panic!("A pipe cannot accept a video buffer"),
        };
        assert_eq!(error.raw_os_error(), Some(libc::ENOTTY));
        assert!(fixture.stream.capture_dequeued);
    }
}
