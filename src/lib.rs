#![deny(missing_docs)]
#![warn(clippy::pedantic)]
#![doc = include_str!("../README.md")]

// Re-export macros
pub use dynamic_plugin_macros::*;
pub use const_format::concatcp as const_concat;

// Re-export libloading library
pub use libloading::Library as PluginDynamicLibrary;
pub use libloading::Symbol as PluginLibrarySymbol;

/// Re-exported libc types for convenience.
pub use libc;

/// The result type returned by dynamic plugin functions.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors returned from dynamic plugin functions.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An error returned from libloading when trying to communicate with the dynamic library.
    #[error("An error while calling the plugin library: {0}")]
    DynamicLibrary(#[from] libloading::Error),

    /// The discovered library is not a plugin, as in it does not expose the `_dynamic_plugin_signature` function.
    #[error("The discovered library is not a plugin.")]
    NotAPlugin,

    /// The plugin's signature (i.e. name, function names, function arguments and function return types) does not match the expected value.
    #[error("The plugin's signature does not match.")]
    InvalidPluginSignature,
}

/// Common trait to all plugin interfaces
///
/// This can be usefull if you want to have your own custom loading function to use for multiple
/// plugins types
/// ```
/// fn custom_load<T, P>(path: P) -> Result<T>
/// where
///     T: PluginInterface,
///     P: ::std::convert::AsRef<::std::ffi::OsStr>
/// {
///     // do stuff
///     let plugin = T::load_plugin_and_check(path)?;
///     // do stuff
///     Ok(plugin)
/// }
/// ```
#[cfg(feature = "host")]
pub trait PluginInterface {
    /// Search `path` to find compatible plugins.
    fn find_plugins<P>(path: P) -> ::std::vec::Vec<Self>
    where
        P: ::std::convert::AsRef<::std::path::Path>,
        Self: Sized;

    /// Load the plugin at `path`
    ///
    /// # Errors
    ///
    /// - [`Error::NotAPlugin`] if the file provided is determined not to be a compatible (dynamic_plugin style) plugin.
    /// - [`Error::InvalidPluginSignature`] if the signature does not match this loader.
    fn load_plugin_and_check<P>(path: P) -> Result<Self>
    where
        P: ::std::convert::AsRef<::std::ffi::OsStr>,
        Self: Sized;

    /// Load the plugin at `path`
    ///
    /// # Errors
    ///
    /// - [`Error::NotAPlugin`] if the file provided is determined not to be a compatible (dynamic_plugin style) plugin.
    /// - [`Error::InvalidPluginSignature`] if `check_signature` is true and the signature does not match this loader.
    fn load_plugin<P>(path: P, check_signature: bool) -> Result<Self>
    where
        P: ::std::convert::AsRef<::std::ffi::OsStr>,
        Self: Sized;

    /// Load the plugin at `path`, checking if it is valid
    /// using a more compatible method, checking for the
    /// presence of each function rather than just the
    /// signature function.
    ///
    /// This makes it slightly easier to implement plugins
    /// in languages other than Rust, however slightly
    /// increases the chances of errors being returned
    /// later, for example if function parameters do not
    /// match, as in compatability mode this is not checked
    /// when the plugin is loaded.
    ///
    /// # Errors
    ///
    /// - [`Error::NotAPlugin`] if the file provided is determined not to be a compatible plugin, i.e. not having the required functions present and exposed.
    fn load_plugin_and_check_compat<P>(path: P) -> Result<Self>
    where
        P: ::std::convert::AsRef<::std::ffi::OsStr>,
        Self: Sized;
}

/// Statically assert an expression with an error message.
///
/// This is used internally by the dynamic-plugin macros.
#[macro_export]
macro_rules! static_assert {
    ($exp:expr, $msg:expr) => {
        #[deny(const_err)]
        #[allow(unused_must_use)]
        const _: () = {
            if !($exp) {
                core::panic!("{}", $msg);
            }

            ()
        };
    };
}
