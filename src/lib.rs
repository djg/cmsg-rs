extern crate bytes;
extern crate libc;

use bytes::BufMut;
use libc::cmsghdr;
use std::{convert, fmt, mem, ops, result, slice};
use std::os::unix::io::RawFd;
use std::time::Duration;

#[derive(Clone, Copy)]
pub struct CMsg<'a> {
    kind: libc::c_int,
    level: libc::c_int,
    data: &'a [u8],
}

impl<'a> CMsg<'a> {
    pub fn kind(&self) -> libc::c_int {
        self.kind
    }

    pub fn level(&self) -> libc::c_int {
        self.level
    }
}

impl<'a> convert::AsRef<[u8]> for CMsg<'a> {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.data
    }
}

impl<'a> fmt::Debug for CMsg<'a> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let len = self.data.len();
        fmt.debug_struct("ControlMsg")
            .field("len", &len)
            .field("level", &self.level)
            .field("type", &self.kind)
            .finish()
    }
}

impl<'a> ops::Deref for CMsg<'a> {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &[u8] {
        self.as_ref()
    }
}

pub enum ControlMsg<'a> {
    Raw(CMsg<'a>),
    Rights(&'a [RawFd]),
    Timestamp(Duration),
    Credentials(libc::ucred),
}

pub struct ControlMsgIter<'a> {
    control: &'a [u8],
}

pub fn iterator<'a>(c: &'a [u8]) -> ControlMsgIter<'a> {
    ControlMsgIter { control: c }
}

impl<'a> Iterator for ControlMsgIter<'a> {
    type Item = ControlMsg<'a>;

    // This follows the logic in __cmsg_nxthdr from glibc
    // /usr/include/bits/socket.h
    fn next(&mut self) -> Option<Self::Item> {
        use ControlMsg::*;

        let control = self.control;
        let cmsghdr_len = align(mem::size_of::<cmsghdr>());

        if control.len() < cmsghdr_len {
            // No more entries---not enough data in `control` for a
            // complete message.
            return None;
        }

        let cmsg: &cmsghdr = unsafe { &*(control.as_ptr() as *const _) };
        // The offset to the next cmsghdr in control.  This must be
        // aligned to a boundary that matches the type used to
        // represent the length of the message.
        let cmsg_len = cmsg.cmsg_len;
        let next_cmsghdr = align(cmsg_len);
        self.control = if next_cmsghdr > control.len() {
            // No more entries---not enough data in `control` for a
            // complete message.
            &[]
        } else {
            &control[next_cmsghdr..]
        };

        let data = &control[cmsghdr_len..cmsg_len];
        match (cmsg.cmsg_level, cmsg.cmsg_type) {
            (libc::SOL_SOCKET, libc::SCM_RIGHTS) => {
                let ptr = data.as_ptr() as *const _;
                let count = data.len() / mem::size_of::<RawFd>();
                let fds = unsafe { slice::from_raw_parts(ptr, count) };
                Some(Rights(fds))
            }
            (libc::SOL_SOCKET, libc::SCM_CREDENTIALS) => {
                assert_eq!(data.len(), mem::size_of::<libc::ucred>());
                let creds = unsafe { &*(data.as_ptr() as *const libc::ucred) };
                Some(Credentials(*creds))
            }
            (libc::SOL_SOCKET, libc::SCM_TIMESTAMP) => {
                assert_eq!(data.len(), mem::size_of::<libc::timeval>());
                let tv = unsafe { &*(data.as_ptr() as *const libc::timeval) };
                assert!(tv.tv_sec > 0);
                assert!(tv.tv_usec > 0);
                assert!(tv.tv_usec * 1000 < u32::max_value() as i64);
                let timestamp = Duration::new(tv.tv_sec as u64, tv.tv_usec as u32 * 1000);
                Some(Timestamp(timestamp))
            }
            (libc::SOL_SOCKET, libc::SCM_TIMESTAMPNS) => {
                assert_eq!(data.len(), mem::size_of::<libc::timespec>());
                let ts = unsafe { &*(data.as_ptr() as *const libc::timespec) };
                assert!(ts.tv_sec > 0);
                assert!(ts.tv_nsec > 0);
                assert!(ts.tv_nsec < u32::max_value() as i64);
                let timestamp = Duration::new(ts.tv_sec as u64, ts.tv_nsec as u32);
                Some(Timestamp(timestamp))
            }
            (level, kind) => Some(Raw(CMsg {
                kind: kind,
                level: level,
                data: data,
            })),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Error {
    /// The passed storage object is not correctly aligned to start a
    /// control message.
    Alignment,
    /// Not enough space in storage to insert control mesage.
    NoSpace,
}

pub type Result = result::Result<(), Error>;

#[must_use]
pub struct ControlMsgBuilder<'a, B: 'a> {
    buf: &'a mut B,
    result: Result,
}

pub fn builder<'a, B: BufMut>(buf: &'a mut B) -> ControlMsgBuilder<'a, B> {
    let result = aligned(buf);
    ControlMsgBuilder {
        buf: buf,
        result: result,
    }
}

impl<'a, B> ControlMsgBuilder<'a, B>
where
    B: BufMut,
{
    pub fn msg(
        &mut self,
        level: libc::c_int,
        kind: libc::c_int,
        msg: &[u8],
    ) -> &mut ControlMsgBuilder<'a, B> {
        self.result = self.result.and_then(|_| {
            try!(self.align_buf());
            let cmsg_len = len(msg.len());
            if self.buf.remaining_mut() < cmsg_len {
                return Err(Error::NoSpace);
            }

            let cmsghdr = cmsghdr {
                cmsg_len: cmsg_len,
                cmsg_level: level,
                cmsg_type: kind,
            };

            let cmsghdr = unsafe {
                slice::from_raw_parts(&cmsghdr as *const _ as *const _, mem::size_of::<cmsghdr>())
            };
            self.buf.put_slice(cmsghdr);
            try!(self.align_buf());
            self.buf.put_slice(msg);

            Ok(())
        });

        self
    }

    pub fn rights(&mut self, fds: &[RawFd]) -> &mut ControlMsgBuilder<'a, B> {
        self.msg(libc::SOL_SOCKET, libc::SCM_RIGHTS, fds.as_bytes())
    }

    pub fn finish(&mut self) -> Result {
        self.result
    }

    fn align_buf(&mut self) -> Result {
        let offset = unsafe { self.buf.bytes_mut().as_ptr() } as usize;
        let adjust = align(offset) - offset;
        if self.buf.remaining_mut() < adjust {
            return Err(Error::NoSpace);
        }

        for _ in 0..adjust {
            self.buf.put_u8(0);
        }

        Ok(())
    }
}

fn aligned<B>(b: &mut B) -> Result
where
    B: BufMut,
{
    let cmsghdr_align = mem::align_of::<cmsghdr>();
    if (unsafe { b.bytes_mut().as_ptr() } as usize) & (cmsghdr_align - 1) == 0 {
        Ok(())
    } else {
        Err(Error::Alignment)
    }
}

fn align(len: usize) -> usize {
    let cmsghdr_align = mem::align_of::<cmsghdr>();
    (len + cmsghdr_align - 1) & !(cmsghdr_align - 1)
}

fn len(len: usize) -> usize {
    align(mem::size_of::<cmsghdr>()) + len
}

pub trait AsBytes {
    fn as_bytes(&self) -> &[u8];
}

impl<'a, T: Sized> AsBytes for &'a [T] {
    fn as_bytes(&self) -> &[u8] {
        // TODO: This should account for the alignment of T
        let byte_count = self.len() * mem::size_of::<T>();
        unsafe { slice::from_raw_parts(self.as_ptr() as *const _, byte_count) }
    }
}

pub trait AsBytesMut {
    fn as_bytes_mut(&mut self) -> &mut [u8];
}

impl<'a, T: Sized> AsBytesMut for &'a mut [T] {
    fn as_bytes_mut(&mut self) -> &mut [u8] {
        // TODO: This should account for the alignment of T
        let byte_count = self.len() * mem::size_of::<T>();
        unsafe { slice::from_raw_parts_mut(self.as_ptr() as *mut _, byte_count) }
    }
}

macro_rules! array_impls {
    ($($N:expr)+) => {
        $(
            impl<T> AsBytes for [T; $N] {
                fn as_bytes(&self) -> &[u8] {
                    let ptr = self.as_ptr();
                    unsafe { slice::from_raw_parts(ptr as *const _, mem::size_of::<T>() * $N) }
                }
            }

            impl<T> AsBytesMut for [T; $N] {
                fn as_bytes_mut(&mut self) -> &mut [u8] {
                    let ptr = self.as_ptr();
                    unsafe { slice::from_raw_parts_mut(ptr as *mut _, mem::size_of::<T>() * $N) }
                }
            }
        )+
    }
}

array_impls! {
     0  1  2  3  4  5  6  7  8  9
    10 11 12 13 14 15 16 17 18 19
    20 21 22 23 24 25 26 27 28 29
    30 31 32
}
