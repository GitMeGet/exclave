//! Provide helpers for making ioctl system calls.
//!
//! This library is pretty low-level and messy. `ioctl` is not fun.
//!
//! What is an `ioctl`?
//! ===================
//!
//! The `ioctl` syscall is the grab-bag syscall on POSIX systems. Don't want to add a new
//! syscall? Make it an `ioctl`! `ioctl` refers to both the syscall, and the commands that can be
//! sent with it. `ioctl` stands for "IO control", and the commands are always sent to a file
//! descriptor.
//!
//! It is common to see `ioctl`s used for the following purposes:
//!
//!   * Provide read/write access to out-of-band data related to a device such as configuration
//!     (for instance, setting serial port options)
//!   * Provide a mechanism for performing full-duplex data transfers (for instance, xfer on SPI
//!     devices).
//!   * Provide access to control functions on a device (for example, on Linux you can send
//!     commands like pause, resume, and eject to the CDROM device.
//!   * Do whatever else the device driver creator thought made most sense.
//!
//! `ioctl`s are synchronous system calls and are similar to read and write calls in that regard.
//! They operate on file descriptors and have an identifier that specifies what the ioctl is.
//! Additionally they may read or write data and therefore need to pass along a data pointer.
//! Besides the semantics of the ioctls being confusing, the generation of this identifer can also
//! be difficult.
//!
//! Historically `ioctl` numbers were arbitrary hard-coded values. In Linux (before 2.6) and some
//! unices this has changed to a more-ordered system where the ioctl numbers are partitioned into
//! subcomponents (For linux this is documented in
//! [`Documentation/ioctl/ioctl-number.txt`](http://elixir.free-electrons.com/linux/latest/source/Documentation/ioctl/ioctl-number.txt)):
//!
//!   * Number: The actual ioctl ID
//!   * Type: A grouping of ioctls for a common purpose or driver
//!   * Size: The size in bytes of the data that will be transferred
//!   * Direction: Whether there is any data and if it's read, write, or both
//!
//! Newer drivers should not generate complete integer identifiers for their `ioctl`s instead
//! preferring to use the 4 components above to generate the final ioctl identifier. Because of
//! how old `ioctl`s are, however, there are many hard-coded `ioctl` identifiers. These are
//! commonly referred to as "bad" in `ioctl` documentation.
//!
//! Defining ioctls
//! ===============
//!
//! This library provides the `ioctl!` macro, for binding `ioctl`s. This macro generates public
//! unsafe functions that can then be used for calling the ioctl. This macro has a few different
//! ways it can be used depending on the specific ioctl you're working with.
//!
//! A simple `ioctl` is `SPI_IOC_RD_MODE`. This ioctl works with the SPI interface on Linux. This
//! specific `ioctl` reads the mode of the SPI device as a `u8`. It's declared in
//! `/include/uapi/linux/spi/spidev.h` as `_IOR(SPI_IOC_MAGIC, 1, __u8)`. Since it uses the `_IOR`
//! macro, we know it's a `read` ioctl and can use the `ioctl!` macro as follows:
//!
//! ```
//! # #[macro_use] extern crate nix;
//! const SPI_IOC_MAGIC: u8 = b'k'; // Defined in linux/spi/spidev.h
//! const SPI_IOC_TYPE_MODE: u8 = 1;
//! ioctl!(read spi_read_mode with SPI_IOC_MAGIC, SPI_IOC_TYPE_MODE; u8);
//! # fn main() {}
//! ```
//!
//! This generates the function:
//!
//! ```
//! # #[macro_use] extern crate nix;
//! # use std::mem;
//! # use nix::{Errno, libc, Result};
//! # use nix::libc::c_int as c_int;
//! # const SPI_IOC_MAGIC: u8 = b'k'; // Defined in linux/spi/spidev.h
//! # const SPI_IOC_TYPE_MODE: u8 = 1;
//! pub unsafe fn spi_read_mode(fd: c_int, data: *mut u8) -> Result<c_int> {
//!     let res = libc::ioctl(fd, ior!(SPI_IOC_MAGIC, SPI_IOC_TYPE_MODE, mem::size_of::<u8>()), data);
//!     Errno::result(res)
//! }
//! # fn main() {}
//! ```
//!
//! The return value for `ioctl` functions generated by the `ioctl!` macro are `nix::Error`s.
//! These are generated by assuming the return value of the ioctl is `-1` on error and everything
//! else is a valid return value. If this is not the case, `Result::map` can be used to map some
//! of the range of "good" values (-Inf..-2, 0..Inf) into a smaller range in a helper function.
//!
//! Writing `ioctl`s generally use pointers as their data source and these should use the
//! `write_ptr` variant. But in some cases an `int` is passed directly. For these `ioctl`s use the
//! `write_int` variant of the `ioctl!` macro. This variant does not take a type as the last argument:
//!
//! ```
//! # #[macro_use] extern crate nix;
//! const HCI_IOC_MAGIC: u8 = b'k';
//! const HCI_IOC_HCIDEVUP: u8 = 1;
//! ioctl!(write_int hci_dev_up with HCI_IOC_MAGIC, HCI_IOC_HCIDEVUP);
//! # fn main() {}
//! ```
//!
//! Some `ioctl`s don't transfer any data, and those should use the `none` variant. This variant
//! doesn't take a type and so it is declared similar to the `write_int` variant shown above.
//!
//! The mode for a given `ioctl` should be clear from the documentation if it has good
//! documentation. Otherwise it will be clear based on the macro used to generate the `ioctl`
//! number where `_IO`, `_IOR`, `_IOW`, and `_IORW` map to "none", "read", "write_*", and "readwrite"
//! respectively. To determine the specific `write_` variant to use you'll need to find
//! what the argument type is supposed to be. If it's an `int`, then `write_int` should be used,
//! otherwise it should be a pointer and `write_ptr` should be used. On Linux the
//! [`ioctl_list` man page](http://man7.org/linux/man-pages/man2/ioctl_list.2.html) describes a
//! large number of `ioctl`s and describes their argument data type.
//!
//! More examples on using `ioctl!` can be found in the [rust-spidev crate](https://github.com/rust-embedded/rust-spidev).
//!
//! Using hard-coded ioctl numbers
//! ------------------------------
//!
//! As mentioned earlier, there are many old `ioctl`s that do not use the newer method of
//! generating `ioctl` numbers and instead use hardcoded values. These can be used with the `bad *`
//! variants of the `ioctl!` macro. This naming comes from the Linux kernel which refers to these
//! `ioctl`s as "bad". These are a different variant as they bypass calling the macro that generates
//! the ioctl number and instead use the defined value directly.
//!
//! For example the `TCGETS` `ioctl` reads a `termios` data structure for a given file descriptor.
//! It's defined as `0x5401` in `ioctls.h` on Linux and can be implemented as:
//!
//! ```
//! # #[macro_use] extern crate nix;
//! # #[cfg(any(target_os = "android", target_os = "linux"))]
//! # use nix::libc::TCGETS as TCGETS;
//! # #[cfg(any(target_os = "android", target_os = "linux"))]
//! # use nix::libc::termios as termios;
//! # #[cfg(any(target_os = "android", target_os = "linux"))]
//! ioctl!(bad read tcgets with TCGETS; termios);
//! # fn main() {}
//! ```
//!
//! The generated function has the same form as that generated by `read`:
//!
//! ```text
//! pub unsafe fn tcgets(fd: c_int, data: *mut termios) -> Result<c_int>;
//! ```
//!
//! There is also a `bad none`, `bad write_int`/`bad write_ptr`, and `bad readwrite` variant that work
//! similar to the standard `none`, `write_int`/`write_ptr`, and `readwrite` variants.
//!
//! Working with arrays
//! --------------------
//!
//! Some `ioctl`s work with entire arrays of elements. These are supported by the `*_buf` variants in
//! the `ioctl!` macro which can be used by specifying `read_buf`, `write_buf`, and
//! `readwrite_buf`. Note that there are no "bad" versions for working with buffers. The generated
//! functions include a `len` argument to specify the number of elements (where the type of each
//! element is specified in the macro).
//!
//! Again looking to the SPI `ioctl`s on Linux for an example, there is a `SPI_IOC_MESSAGE` `ioctl`
//! that queues up multiple SPI messages by writing an entire array of `spi_ioc_transfer` structs.
//! `linux/spi/spidev.h` defines a macro to calculate the `ioctl` number like:
//!
//! ```C
//! #define SPI_IOC_MAGIC 'k'
//! #define SPI_MSGSIZE(N) ...
//! #define SPI_IOC_MESSAGE(N) _IOW(SPI_IOC_MAGIC, 0, char[SPI_MSGSIZE(N)])
//! ```
//!
//! The `SPI_MSGSIZE(N)` calculation is already handled by the `ioctl!` macro, so all that's
//! needed to define this `ioctl` is:
//!
//! ```
//! # #[macro_use] extern crate nix;
//! const SPI_IOC_MAGIC: u8 = b'k'; // Defined in linux/spi/spidev.h
//! const SPI_IOC_TYPE_MESSAGE: u8 = 0;
//! # pub struct spi_ioc_transfer(u64);
//! ioctl!(write_buf spi_transfer with SPI_IOC_MAGIC, SPI_IOC_TYPE_MESSAGE; spi_ioc_transfer);
//! # fn main() {}
//! ```
//!
//! This generates a function like:
//!
//! ```
//! # #[macro_use] extern crate nix;
//! # use std::mem;
//! # use nix::{Errno, libc, Result};
//! # use nix::libc::c_int as c_int;
//! # const SPI_IOC_MAGIC: u8 = b'k';
//! # const SPI_IOC_TYPE_MESSAGE: u8 = 0;
//! # pub struct spi_ioc_transfer(u64);
//! pub unsafe fn spi_message(fd: c_int, data: &mut [spi_ioc_transfer]) -> Result<c_int> {
//!     let res = libc::ioctl(fd,
//!                           iow!(SPI_IOC_MAGIC, SPI_IOC_TYPE_MESSAGE, data.len() * mem::size_of::<spi_ioc_transfer>()),
//!                           data);
//!     Errno::result(res)
//! }
//! # fn main() {}
//! ```
//!
//! Finding ioctl documentation
//! ---------------------------
//!
//! For Linux, look at your system's headers. For example, `/usr/include/linux/input.h` has a lot
//! of lines defining macros which use `_IO`, `_IOR`, `_IOW`, `_IOC`, and `_IORW`. Some `ioctl`s are
//! documented directly in the headers defining their constants, but others have more extensive
//! documentation in man pages (like termios' `ioctl`s which are in `tty_ioctl(4)`).
//!
//! Documenting the generated functions
//! ===================================
//!
//! In many cases, users will wish for the functions generated by the `ioctl`
//! macro to be public and documented. For this reason, the generated functions
//! are public by default. If you wish to hide the ioctl, you will need to put
//! them in a private module.
//!
//! For documentation, it is possible to use doc comments inside the `ioctl!`
//! macro. Here is an example :
//!
//! ```
//! # #[macro_use] extern crate nix;
//! # use nix::libc::c_int;
//! ioctl! {
//!     /// Make the given terminal the controlling terminal of the calling process. The calling
//!     /// process must be a session leader and not have a controlling terminal already. If the
//!     /// terminal is already the controlling terminal of a different session group then the
//!     /// ioctl will fail with **EPERM**, unless the caller is root (more precisely: has the
//!     /// **CAP_SYS_ADMIN** capability) and arg equals 1, in which case the terminal is stolen
//!     /// and all processes that had it as controlling terminal lose it.
//!     read tiocsctty with b't', 19; c_int
//! }
//!
//! # fn main() {}
//! ```
//!
#[cfg(any(target_os = "linux", target_os = "android"))]
#[path = "platform/linux.rs"]
#[macro_use]
mod platform;

#[cfg(any(target_os = "macos",
          target_os = "ios",
          target_os = "netbsd",
          target_os = "openbsd",
          target_os = "freebsd",
          target_os = "dragonfly"))]
#[path = "platform/bsd.rs"]
#[macro_use]
mod platform;

pub use self::platform::*;

/// Convert raw ioctl return value to a Nix result
#[macro_export]
#[doc(hidden)]
macro_rules! convert_ioctl_res {
    ($w:expr) => (
        {
            $crate::Errno::result($w)
        }
    );
}

/// Generates ioctl functions. See [::sys::ioctl](sys/ioctl/index.html).
#[macro_export]
macro_rules! ioctl {
    ($(#[$attr:meta])* bad none $name:ident with $nr:expr) => (
        $(#[$attr])*
        pub unsafe fn $name(fd: $crate::libc::c_int)
                            -> $crate::Result<$crate::libc::c_int> {
            convert_ioctl_res!($crate::libc::ioctl(fd, $nr as $crate::sys::ioctl::ioctl_num_type))
        }
        );
    ($(#[$attr:meta])* bad read $name:ident with $nr:expr; $ty:ty) => (
        $(#[$attr])*
        pub unsafe fn $name(fd: $crate::libc::c_int,
                            data: *mut $ty)
                            -> $crate::Result<$crate::libc::c_int> {
            convert_ioctl_res!($crate::libc::ioctl(fd, $nr as $crate::sys::ioctl::ioctl_num_type, data))
        }
        );
    ($(#[$attr:meta])* bad write_ptr $name:ident with $nr:expr; $ty:ty) => (
        $(#[$attr])*
        pub unsafe fn $name(fd: $crate::libc::c_int,
                            data: *const $ty)
                            -> $crate::Result<$crate::libc::c_int> {
            convert_ioctl_res!($crate::libc::ioctl(fd, $nr as $crate::sys::ioctl::ioctl_num_type, data))
        }
        );
    ($(#[$attr:meta])* bad write_int $name:ident with $nr:expr) => (
        $(#[$attr])*
        pub unsafe fn $name(fd: $crate::libc::c_int,
                            data: $crate::libc::c_int)
                            -> $crate::Result<$crate::libc::c_int> {
            convert_ioctl_res!($crate::libc::ioctl(fd, $nr as $crate::sys::ioctl::ioctl_num_type, data))
        }
        );
    ($(#[$attr:meta])* bad readwrite $name:ident with $nr:expr; $ty:ty) => (
        $(#[$attr])*
        pub unsafe fn $name(fd: $crate::libc::c_int,
                            data: *mut $ty)
                            -> $crate::Result<$crate::libc::c_int> {
            convert_ioctl_res!($crate::libc::ioctl(fd, $nr as $crate::sys::ioctl::ioctl_num_type, data))
        }
        );
    ($(#[$attr:meta])* none $name:ident with $ioty:expr, $nr:expr) => (
        $(#[$attr])*
        pub unsafe fn $name(fd: $crate::libc::c_int)
                            -> $crate::Result<$crate::libc::c_int> {
            convert_ioctl_res!($crate::libc::ioctl(fd, io!($ioty, $nr) as $crate::sys::ioctl::ioctl_num_type))
        }
        );
    ($(#[$attr:meta])* read $name:ident with $ioty:expr, $nr:expr; $ty:ty) => (
        $(#[$attr])*
        pub unsafe fn $name(fd: $crate::libc::c_int,
                            data: *mut $ty)
                            -> $crate::Result<$crate::libc::c_int> {
            convert_ioctl_res!($crate::libc::ioctl(fd, ior!($ioty, $nr, ::std::mem::size_of::<$ty>()) as $crate::sys::ioctl::ioctl_num_type, data))
        }
        );
    ($(#[$attr:meta])* write_ptr $name:ident with $ioty:expr, $nr:expr; $ty:ty) => (
        $(#[$attr])*
        pub unsafe fn $name(fd: $crate::libc::c_int,
                            data: *const $ty)
                            -> $crate::Result<$crate::libc::c_int> {
            convert_ioctl_res!($crate::libc::ioctl(fd, iow!($ioty, $nr, ::std::mem::size_of::<$ty>()) as $crate::sys::ioctl::ioctl_num_type, data))
        }
        );
    ($(#[$attr:meta])* write_int $name:ident with $ioty:expr, $nr:expr) => (
        $(#[$attr])*
        pub unsafe fn $name(fd: $crate::libc::c_int,
                            data: $crate::libc::c_int)
                            -> $crate::Result<$crate::libc::c_int> {
            convert_ioctl_res!($crate::libc::ioctl(fd, iow!($ioty, $nr, ::std::mem::size_of::<$crate::libc::c_int>()) as $crate::sys::ioctl::ioctl_num_type, data))
        }
        );
    ($(#[$attr:meta])* readwrite $name:ident with $ioty:expr, $nr:expr; $ty:ty) => (
        $(#[$attr])*
        pub unsafe fn $name(fd: $crate::libc::c_int,
                            data: *mut $ty)
                            -> $crate::Result<$crate::libc::c_int> {
            convert_ioctl_res!($crate::libc::ioctl(fd, iorw!($ioty, $nr, ::std::mem::size_of::<$ty>()) as $crate::sys::ioctl::ioctl_num_type, data))
        }
        );
    ($(#[$attr:meta])* read_buf $name:ident with $ioty:expr, $nr:expr; $ty:ty) => (
        $(#[$attr])*
        pub unsafe fn $name(fd: $crate::libc::c_int,
                            data: &mut [$ty])
                            -> $crate::Result<$crate::libc::c_int> {
            convert_ioctl_res!($crate::libc::ioctl(fd, ior!($ioty, $nr, data.len() * ::std::mem::size_of::<$ty>()) as $crate::sys::ioctl::ioctl_num_type, data))
        }
        );
    ($(#[$attr:meta])* write_buf $name:ident with $ioty:expr, $nr:expr; $ty:ty) => (
        $(#[$attr])*
        pub unsafe fn $name(fd: $crate::libc::c_int,
                            data: &[$ty])
                            -> $crate::Result<$crate::libc::c_int> {
            convert_ioctl_res!($crate::libc::ioctl(fd, iow!($ioty, $nr, data.len() * ::std::mem::size_of::<$ty>()) as $crate::sys::ioctl::ioctl_num_type, data))
        }
        );
    ($(#[$attr:meta])* readwrite_buf $name:ident with $ioty:expr, $nr:expr; $ty:ty) => (
        $(#[$attr])*
        pub unsafe fn $name(fd: $crate::libc::c_int,
                            data: &mut [$ty])
                            -> $crate::Result<$crate::libc::c_int> {
            convert_ioctl_res!($crate::libc::ioctl(fd, iorw!($ioty, $nr, data.len() * ::std::mem::size_of::<$ty>()) as $crate::sys::ioctl::ioctl_num_type, data))
        }
        );
}