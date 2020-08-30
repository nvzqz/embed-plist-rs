//! Embed an [`Info.plist`] or [`launchd.plist`] file directly in your
//! executable binary, brought to you by [@NikolaiVazquez]!
//!
//! If you found this library useful, please consider
//! [sponsoring me on GitHub](https://github.com/sponsors/nvzqz). ❤️
//!
//! # Index
//!
//! 1. [Motivation](#motivation)
//! 2. [Usage](#usage)
//! 3. [Minimum Supported Rust Version](#minimum-supported-rust-version)
//! 4. [Multi-Target Considerations](#multi-target-considerations)
//! 5. [Accidental Reuse Protection](#accidental-reuse-protection)
//! 6. [Implementation](#implementation)
//! 7. [License](#license)
//! 8. [Macros](#macros)
//!
//! # Motivation
//!
//! Certain programs require an embedded `Info.plist` or `launchd.plist` file to
//! work correctly. For example:
//!
//! - `Info.plist` is needed to obtain certain permissions on macOS 10.15 and
//!   later.
//!
//! - `launchd.plist` is needed to make
//!   [`launchd` daemons and user agents](https://developer.apple.com/library/archive/documentation/MacOSX/Conceptual/BPSystemStartup/Chapters/CreatingLaunchdJobs.html).
//!
//! Doing this manually with [`include_bytes!`] is cumbersome. To understand
//! why, see the [implementation](#implementation). This library removes the
//! headache of doing this.
//!
//! # Usage
//!
//! This library is available [on crates.io][crate] and can be used by adding
//! the following to your project's [`Cargo.toml`]:
//!
//! ```toml
//! [dependencies]
//! embed_plist = "0"
//! ```
//!
//! ...and this to any source file in your crate:
//!
//! ```rust
//! embed_plist::embed_info_plist!("Info.plist");
//!
//! // If making a daemon:
//! embed_plist::embed_launchd_plist!("launchd.plist");
//! ```
//!
//! Done! It's that simple. 🙂
//!
//! See [implementation](#implementation) for details on this sorcery.
//!
//! # Minimum Supported Rust Version
//!
//! This library targets <b>1.39</b> as its minimum supported Rust version
//! (MSRV).
//!
//! Requiring a newer Rust version is considered a breaking change and will
//! result in a "major" library version update. In other words: `0.1.z` would
//! become `0.2.0`, or `1.y.z` would become `2.0.0`.
//!
//! # Multi-Target Considerations
//!
//! This library only works for [Mach-O](https://en.wikipedia.org/wiki/Mach-O)
//! binaries. When building a cross-platform program, these macro calls should
//! be placed behind a `#[cfg]` to prevent linker errors on other targets.
//!
//! ```rust
//! #[cfg(target_os = "macos")]
//! embed_plist::embed_info_plist!("Info.plist");
//! ```
//!
//! # Accidental Reuse Protection
//!
//! Only one copy of `Info.plist` or `launchd.plist` should exist in a binary.
//! Accidentally embedding them multiple times would break tools that read these
//! sections.
//!
//! Fortunately, this library makes reuse a compile-time error! This protection
//! works even if these macros are reused in different modules.
//!
//! ```compile_fail
//! # #[cfg(pass_reuse_doctest)]
//! # compile_error!("hack to force a compile error pre 1.43");
//! embed_plist::embed_info_plist!("Info.plist");
//! embed_plist::embed_info_plist!("Info.plist");
//! ```
//!
//! <p style="background:rgba(34, 133, 255, 0.1);padding:0.75em;">
//! <b>Note:</b> This check only works on Rust 1.43 and later.
//! </p>
//!
//! This example produces the following error:
//!
//! ```txt
//! error: symbol `_EMBED_INFO_PLIST` is already defined
//!  --> src/main.rs:4:1
//!   |
//! 4 | embed_plist::embed_info_plist!("Info.plist");
//!   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
//!   |
//!   = note: this error originates in a macro (in Nightly builds, run with -Z macro-backtrace for more info)
//!
//! error: aborting due to previous error
//! ```
//!
//! <p style="background:rgba(255, 181, 77, 0.16);padding:0.75em;">
//! <b>Warning:</b> Although the name
//! <code style="background:rgba(41, 24, 0, 0.1);">_EMBED_INFO_PLIST</code>
//! can be seen here, you <strong>should not</strong> reference this symbol with
//! e.g. an
//! <code style="background:rgba(41, 24, 0, 0.1);">extern "C"</code>
//! block. I reserve the right to change this name in a SemVer-compatible
//! update.
//! </p>
//!
//! # Implementation
//!
//! Files are read using [`include_bytes!`]. This normally places data in
//! `__TEXT,__const`, where immutable data is kept. However, property list data
//! is expected to be in `__TEXT,__info_plist` or `__TEXT,__launchd_plist`. This
//! section will explain how I accomplish that.
//!
//! We begin by reading the file from disk:
//!
//! ```rust
//! const BYTES: &[u8] = include_bytes!("Info.plist");
//! ```
//!
//! A naïve approach is to do the following:
//!
//! ```rust
//! # const BYTES: &[u8] = &[];
//! #[used] // Prevent optimizing out
//! #[link_section = "__TEXT,__info_plist"]
//! static PLIST: &[u8] = BYTES;
//! ```
//!
//! This doesn't work because only the slice's pointer and length that are
//! placed in `__TEXT,__info_plist`. The referenced bytes are still placed in
//! `__TEXT,__const`.
//!
//! Instead, we need to arrive at the following:
//!
//! ```rust
//! # const N: usize = 0;
//! # const BYTES: &[u8; N] = &[];
//! #[used]
//! #[link_section = "__TEXT,__info_plist"]
//! static PLIST: [u8; N] = *BYTES;
//! ```
//!
//! We can get `N` by using [`len`]. As of Rust 1.39, it is possible to get the
//! length of a slice within a `const`.
//!
//! ```rust
//! # const BYTES: &[u8] = &[];
//! const N: usize = BYTES.len();
//! ```
//!
//! The next step is to dereference the bytes into a `[u8; N]`.
//!
//! There are two approaches:
//!
//! 1. <span id="approach-1"></span>Call [`include_bytes!`] again.
//!
//!    This is not the approach used by this library because of concerns about
//!    compile performance. See the [second approach](#approach-2) for what this
//!    library does.
//!
//!    The following is all we need:
//!
//!    ```rust
//!    # const BYTES: &[u8] = include_bytes!("Info.plist");
//!    # const N: usize = BYTES.len();
//!    #[used]
//!    #[link_section = "__TEXT,__info_plist"]
//!    static PLIST: [u8; N] = *include_bytes!("Info.plist");
//!    ```
//!
//!    This works because `include_bytes!` actually returns a `&[u8; N]`. It's
//!    often used as a `&[u8]` because we don't know the size when calling it.
//!
//! 2. <span id="approach-2"></span>Dereference our current bytes through
//!    pointer casting.
//!
//!    This is tricker than the [first approach](#approach-1) (and somewhat
//!    cursed). If you know me, then it's predictable I'd go this route.
//!
//!    We can get a pointer to the bytes via [`as_ptr`], which is usable in
//!    `const`:
//!
//!    ```rust
//!    # const BYTES: &[u8] = &[];
//!    # const N: usize = 0;
//!    const PTR: *const [u8; N] = BYTES.as_ptr() as *const [u8; N];
//!    ```
//!
//!    Unfortunately, this pointer can't be directly dereferenced in Rust 1.39
//!    (minimum supported version).
//!
//!    ```compile_fail
//!    # // This may work in future versions, so we intentionally make this
//!    # // always fail to compile so long as our MSRV is 1.39.
//!    #[used]
//!    #[link_section = "__TEXT,__info_plist"]
//!    static PLIST: [u8; N] = unsafe { *PTR };
//!    ```
//!
//!    Instead, we must cast the pointer to a reference.
//!
//!    You may want to reach for [`transmute`], which was stabilized for use in
//!    `const` in Rust 1.46. However, earlier versions need to be supported, so
//!    that is not an option for this library.
//!
//!    This bitwise cast can be accomplished with a `union`:
//!
//!    ```rust
//!    # const BYTES: &[u8] = &[];
//!    # const N: usize = 0;
//!    # const PTR: *const [u8; N] = BYTES.as_ptr() as *const [u8; N];
//!    union Transmute {
//!        from: *const [u8; N],
//!        into: &'static [u8; N],
//!    }
//!
//!    const REF: &[u8; N] = unsafe { Transmute { from: PTR }.into };
//!    ```
//!
//!    Finally, we can dereference our bytes:
//!
//!    ```rust
//!    # const N: usize = 0;
//!    # const REF: &[u8; N] = &[];
//!    #[used]
//!    #[link_section = "__TEXT,__info_plist"]
//!    static PLIST: [u8; N] = *REF;
//!    ```
//!
//! # License
//!
//! This project is released under either:
//!
//! - [MIT License](https://github.com/nvzqz/embed-plist-rs/blob/master/LICENSE-MIT)
//! - [Apache License (Version 2.0)](https://github.com/nvzqz/embed-plist-rs/blob/master/LICENSE-APACHE)
//!
//! at your choosing.
//!
//! [@NikolaiVazquez]: https://twitter.com/NikolaiVazquez
//!
//! [`Cargo.toml`]: https://doc.rust-lang.org/cargo/reference/manifest.html
//! [crate]:        https://crates.io/crates/embed_plist
//!
//! [`Info.plist`]:    https://developer.apple.com/library/archive/documentation/General/Reference/InfoPlistKeyReference/Introduction/Introduction.html
//! [`launchd.plist`]: https://developer.apple.com/library/archive/documentation/MacOSX/Conceptual/BPSystemStartup/Chapters/CreatingLaunchdJobs.html#//apple_ref/doc/uid/TP40001762-104142
//!
//! [`include_bytes!`]: https://doc.rust-lang.org/std/macro.include_bytes.html
//! [`as_ptr`]:         https://doc.rust-lang.org/std/primitive.slice.html#method.as_ptr
//! [`len`]:            https://doc.rust-lang.org/std/primitive.slice.html#method.len
//! [`transmute`]:      https://doc.rust-lang.org/std/mem/fn.transmute.html

#![no_std]

// This exists to ensure there are no conflicts when calling `include_bytes!`.
// It is not part of this crate's public API, so I reserve the right to change
// or remove this in a SemVer-compatible update.
#[doc(hidden)]
pub use core as _core;

/// Embeds the [`Info.plist`] file at `$path` directly in the current binary.
///
/// # Accidental Reuse Protection
///
/// Only one copy of `Info.plist` should exist in a binary. Accidentally embedding
/// it multiple times would break tools that read this section.
///
/// Fortunately, this library makes reuse a compile-time error! This protection
/// works even if this macro is reused in different modules.
///
/// ```compile_fail
/// # #[cfg(pass_reuse_doctest)]
/// # compile_error!("hack to force a compile error pre 1.43");
/// embed_plist::embed_info_plist!("Info.plist");
/// embed_plist::embed_info_plist!("Info.plist");
/// ```
///
/// <p style="background:rgba(34, 133, 255, 0.1);padding:0.75em;">
/// <b>Note:</b> This check only works on Rust 1.43 and later.
/// </p>
///
/// This example produces the following error:
///
/// ```text
/// error: symbol `_EMBED_INFO_PLIST` is already defined
///  --> src/main.rs:4:1
///   |
/// 4 | embed_plist::embed_info_plist!("Info.plist");
///   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
///   |
///   = note: this error originates in a macro (in Nightly builds, run with -Z macro-backtrace for more info)
///
/// error: aborting due to previous error
/// ```
///
/// <p style="background:rgba(255, 181, 77, 0.16);padding:0.75em;">
/// <b>Warning:</b> Although the name
/// <code style="background:rgba(41, 24, 0, 0.1);">_EMBED_INFO_PLIST</code>
/// can be seen here, you <strong>should not</strong> reference this symbol with
/// e.g. an
/// <code style="background:rgba(41, 24, 0, 0.1);">extern "C"</code>
/// block. I reserve the right to change this name in a SemVer-compatible
/// update.
/// </p>
///
/// [`Info.plist`]: https://developer.apple.com/library/archive/documentation/General/Reference/InfoPlistKeyReference/Introduction/Introduction.html
#[macro_export]
macro_rules! embed_info_plist {
    ($path:expr) => {
        // The wildcard `_` prevents polluting the call site with identifiers.
        const _: () = {
            // Because `len` is a `const fn`, we can use it to turn `SLICE` into
            // an array that gets directly embedded. This is necessary because
            // the `__info_plist` section must contain the direct data, not a
            // reference to it.
            const SLICE: &[u8] = $crate::_core::include_bytes!($path);
            const LEN: usize = SLICE.len();

            union Transmute {
                from: *const [u8; LEN],
                into: &'static [u8; LEN],
            }

            const PTR: *const [u8; LEN] = SLICE.as_ptr() as *const _;
            const REF: &[u8; LEN] = unsafe { Transmute { from: PTR }.into };

            // Prevents this from being optimized out of the binary.
            #[used]
            // Places this data in the correct location.
            #[link_section = "__TEXT,__info_plist"]
            // Prevents repeated use by creating a linker error.
            #[no_mangle]
            static _EMBED_INFO_PLIST: [u8; LEN] = *REF;
        };
    };
}

/// Embeds the [`launchd.plist`] file at `$path` directly in the current binary.
///
/// # Accidental Reuse Protection
///
/// Only one copy of `launchd.plist` should exist in a binary. Accidentally
/// embedding it multiple times would break tools that read this section.
///
/// Fortunately, this library makes reuse a compile-time error! This protection
/// works even if this macro is reused in different modules.
///
/// ```compile_fail
/// # #[cfg(pass_reuse_doctest)]
/// # compile_error!("hack to force a compile error pre 1.43");
/// embed_plist::embed_launchd_plist!("launchd.plist");
/// embed_plist::embed_launchd_plist!("launchd.plist");
/// ```
///
/// <p style="background:rgba(34, 133, 255, 0.1);padding:0.75em;">
/// <b>Note:</b> This check only works on Rust 1.43 and later.
/// </p>
///
/// This example produces the following error:
///
/// ```txt
/// error: symbol `_EMBED_LAUNCHD_PLIST` is already defined
///  --> src/main.rs:4:1
///   |
/// 4 | embed_plist::embed_launchd_plist!("launchd.plist");
///   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
///   |
///   = note: this error originates in a macro (in Nightly builds, run with -Z macro-backtrace for more info)
///
/// error: aborting due to previous error
/// ```
///
/// <p style="background:rgba(255, 181, 77, 0.16);padding:0.75em;">
/// <b>Warning:</b> Although the name
/// <code style="background:rgba(41, 24, 0, 0.1);">_EMBED_LAUNCHD_PLIST</code>
/// can be seen here, you <strong>should not</strong> reference this symbol with
/// e.g. an
/// <code style="background:rgba(41, 24, 0, 0.1);">extern "C"</code>
/// block. I reserve the right to change this name in a SemVer-compatible
/// update.
/// </p>
///
/// [`launchd.plist`]: https://developer.apple.com/library/archive/documentation/MacOSX/Conceptual/BPSystemStartup/Chapters/CreatingLaunchdJobs.html#//apple_ref/doc/uid/TP40001762-104142
#[macro_export]
macro_rules! embed_launchd_plist {
    ($path:expr) => {
        // The wildcard `_` prevents polluting the call site with identifiers.
        const _: () = {
            // Because `len` is a `const fn`, we can use it to turn `SLICE` into
            // an array that gets directly embedded. This is necessary because
            // the `__launchd_plist` section must contain the direct data, not a
            // reference to it.
            const SLICE: &[u8] = $crate::_core::include_bytes!($path);
            const LEN: usize = SLICE.len();

            union Transmute {
                from: *const [u8; LEN],
                into: &'static [u8; LEN],
            }

            const PTR: *const [u8; LEN] = SLICE.as_ptr() as *const _;
            const REF: &[u8; LEN] = unsafe { Transmute { from: PTR }.into };

            // Prevents this from being optimized out of the binary.
            #[used]
            // Places this data in the correct location.
            #[link_section = "__TEXT,__launchd_plist"]
            // Prevents repeated use by creating a linker error.
            #[no_mangle]
            static _EMBED_LAUNCHD_PLIST: [u8; LEN] = *REF;
        };
    };
}
