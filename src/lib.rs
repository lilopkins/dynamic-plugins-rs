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
