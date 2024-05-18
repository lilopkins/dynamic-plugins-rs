use std::hash::{Hash, Hasher};

use def::PluginDefinition;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, FnArg};

use crate::hasher::PluginSignatureHasher;

mod def;
mod implementation;
mod hasher;

/// Define an interface for a plugin. See the `dynamic_plugin` crate documentation for more.
/// 
/// ## Example
/// ```ignore
/// plugin_interface! {
///     extern struct ExamplePlugin {
///         /// Ask the plugin to do a thing
///         fn do_a_thing();
///         /// Say hello to a person
///         fn say_hello(to: *const c_char) -> bool;
///     }
/// }
/// ```
#[proc_macro]
pub fn plugin_interface(tokens: TokenStream) -> TokenStream {
    let plugin_def = parse_macro_input!(tokens as PluginDefinition);
    let plugin_ident = &plugin_def.name;

    let mut hasher = PluginSignatureHasher::default();
    plugin_def.hash(&mut hasher);
    let hash = hasher.finish();

    let host_impl = if cfg!(feature = "host") {
        let funcs = plugin_def.functions.iter().map(|pf| {
            let attributes = &pf.attributes;
            let name = &pf.name;
            let name_as_str = format!(r#"b"{name}""#).parse::<TokenStream2>().unwrap();
            let args = &pf.arguments;
            let mut arg_types = vec![];
            let mut arg_names = vec![];
            for arg in args {
                if let FnArg::Typed(typed) = arg {
                    arg_types.push(typed.ty.clone());
                    arg_names.push(typed.pat.clone());
                }
            }
            let ret = if let Some(typ) = &pf.return_type { quote! { #typ } } else { quote! { () } };
            let sig = quote! { unsafe extern fn(#(#arg_types),*) -> #ret };
            quote! {
                #(#attributes)*
                pub extern "C" fn #name(&self, #(#args),*) -> ::dynamic_plugin::Result<#ret> {
                    unsafe {
                        let func: ::dynamic_plugin::PluginLibrarySymbol<#sig> = self.library.get(#name_as_str)?;
                        Ok(func(#(#arg_names),*))
                    }
                }
            }
        });

        quote! {
            impl #plugin_ident {
                /// Search `path` to find compatible plugins.
                pub fn find_plugins<P>(path: P) -> ::std::vec::Vec<Self>
                where
                    P: ::std::convert::AsRef<::std::path::Path>,
                {
                    let mut plugins = vec![];

                    // Iterate through directory entries
                    if let Ok(paths) = ::std::fs::read_dir(path) {
                        for path in paths {
                            if let Ok(path) = path {
                                // Try to load each potential plugin. Catch errors and ignore.
                                if let Ok(plugin) = Self::load_plugin(path.path()) {
                                    plugins.push(plugin);
                                }
                            }
                        }
                    }

                    plugins
                }

                /// Load the plugin at `path`
                pub fn load_plugin<P>(path: P) -> ::dynamic_plugin::Result<Self>
                where
                    P: ::std::convert::AsRef<::std::ffi::OsStr>,
                {
                    unsafe {
                        // Attempt to load library
                        let library = ::dynamic_plugin::PluginDynamicLibrary::new(path)?;

                        // Check plugin library signature
                        let func: ::dynamic_plugin::PluginLibrarySymbol<unsafe extern fn() -> u64> =
                            library.get(b"_dynamic_plugin_signature").map_err(|_| ::dynamic_plugin::Error::NotAPlugin)?;
                        let hash = func();

                        if hash != #hash {
                            return ::dynamic_plugin::Result::Err(::dynamic_plugin::Error::InvalidPluginSignature);
                        }

                        Ok(Self {
                            library,
                        })
                    }
                }

                #(#funcs)*
            }
        }
    } else {
        TokenStream2::new()
    };

    quote! {
        pub struct #plugin_ident {
            library: ::dynamic_plugin::PluginDynamicLibrary,
        }

        impl #plugin_ident {
            pub const PLUGIN_SIGNATURE: u64 = #hash;
        }

        #host_impl
    }
    .into()
}

/// Write an implementation for a plugin. See the `dynamic_plugin` crate documentation for more.
/// 
/// ## `attempt to compute '0_usize - 1_usize', which would overflow`
/// 
/// If you come across this compile-time error, this indicates that the implementation you are writing does not match the expected implementation for the plugin definition. Please check that you:
/// 
/// - Are using the correct definition.
/// - Have all the functions you need to meet the definition.
/// - That all the functions are named correctly (identically to the definition).
/// - That all the function arguments are the same order and types as the definition.
/// - That all the function return types are the same as the definition.
/// 
/// ## Example
/// 
/// ```ignore
/// plugin_impl! {
///     ExamplePlugin,
/// 
///     fn do_a_thing() {
///         println!("A thing has been done!");
///     }
/// 
///     fn say_hello(name: *const c_char) -> bool {
///         unsafe {
///             let name = CStr::from_ptr(name);
///             println!("Hello, {}!", name.to_string_lossy());
///         }
///         true
///     }
/// }
/// ```
#[proc_macro]
#[cfg(feature = "client")]
pub fn plugin_impl(tokens: TokenStream) -> TokenStream {
    use implementation::PluginImplementation;

    let plugin = parse_macro_input!(tokens as PluginImplementation);
    let target_plugin = &plugin.target_plugin;
    let functions = plugin.functions.iter().map(|fun| {
        quote! {
            #[no_mangle]
            pub extern "C" #fun
        }
    });
    let mut hasher = PluginSignatureHasher::default();
    plugin.hash(&mut hasher);
    let hash = hasher.finish();

    quote! {
        ::dynamic_plugin::const_assert_eq!(#target_plugin::PLUGIN_SIGNATURE, #hash);

        #[no_mangle]
        pub extern "C" fn _dynamic_plugin_signature() -> u64 {
            #hash
        }

        #(#functions)*
    }
    .into()
}
