#![deny(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(clippy::missing_panics_doc)]

//! # Macros for the [`dynamic-plugin`](https://docs.rs/dynamic-plugin/latest/dynamic_plugin/) crate.

use std::hash::{Hash, Hasher};

use def::PluginDefinition;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, FnArg, Type};

use crate::hasher::PluginSignatureHasher;

mod def;
mod hasher;
mod implementation;

/// Define an interface for a plugin. See the `dynamic_plugin` crate documentation for more.
///
/// ## Example
/// ```ignore
/// plugin_interface! {
///     extern trait ExamplePlugin {
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
    
    let hash_debug: Option<TokenStream2> = {
        #[cfg(feature = "debug-hashes")]
        {
            let hash_debug = format!("{hasher:?}");
            Some(quote! {
                #[no_mangle]
                pub fn _dynamic_plugin_signature_unhashed() -> &'static str {
                    #hash_debug
                }
            })
        }
        #[cfg(not(feature = "debug-hashes"))]
        {
            None
        }
    };

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
                #hash_debug

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
                                if let Ok(plugin) = Self::load_plugin_and_check(path.path()) {
                                    plugins.push(plugin);
                                }
                            }
                        }
                    }

                    plugins
                }

                /// Load the plugin at `path`
                /// 
                /// # Errors
                /// 
                /// - [`::dynamic_plugin::Error::NotAPlugin`] if the file provided is determined not to be a compatible (dynamic_plugin style) plugin.
                /// - [`::dynamic_plugin::Error::InvalidPluginSignature`] if the signature does not match this loader.
                pub fn load_plugin_and_check<P>(path: P) -> ::dynamic_plugin::Result<Self>
                where
                    P: ::std::convert::AsRef<::std::ffi::OsStr>,
                {
                    Self::load_plugin(path, true)
                }

                /// Load the plugin at `path`
                /// 
                /// # Errors
                /// 
                /// - [`::dynamic_plugin::Error::NotAPlugin`] if the file provided is determined not to be a compatible (dynamic_plugin style) plugin.
                /// - [`::dynamic_plugin::Error::InvalidPluginSignature`] if `check_signature` is true and the signature does not match this loader.
                pub fn load_plugin<P>(path: P, check_signature: bool) -> ::dynamic_plugin::Result<Self>
                where
                    P: ::std::convert::AsRef<::std::ffi::OsStr>,
                {
                    unsafe {
                        // Attempt to load library
                        let library = ::dynamic_plugin::PluginDynamicLibrary::new(path)?;

                        // Check that signature function exists
                        let func: ::dynamic_plugin::PluginLibrarySymbol<unsafe extern fn() -> u64> =
                            library.get(b"_dynamic_plugin_signature").map_err(|_| ::dynamic_plugin::Error::NotAPlugin)?;
                        if check_signature {
                            // Check plugin library signature
                            let hash = func();
                            
                            if hash != #hash {
                                return ::dynamic_plugin::Result::Err(::dynamic_plugin::Error::InvalidPluginSignature);
                            }
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
    let functions = plugin.functions.iter().map(|maybe_unsafe_func| {
        let unsafe_ = maybe_unsafe_func.unsafe_;
        let func = &maybe_unsafe_func.func;
        quote! {
            #[no_mangle]
            pub #unsafe_ extern "C" #func
        }
    });
    let mut hasher = PluginSignatureHasher::default();
    plugin.hash(&mut hasher);
    let hash = hasher.finish();

    let hash_debug: Option<TokenStream2> = {
        #[cfg(feature = "debug-hashes")]
        {
            let hash_debug = format!("{hasher:?}");
            Some(quote! {
                #[no_mangle]
                pub fn _dynamic_plugin_signature_unhashed() -> &'static str {
                    #hash_debug
                }
            })
        }
        #[cfg(not(feature = "debug-hashes"))]
        {
            None
        }
    };

    quote! {
        ::dynamic_plugin::static_assert!(#target_plugin::PLUGIN_SIGNATURE == #hash, "The implementation signature does not match the definition. Check that all functions are implemented with the correct types.");

        #[no_mangle]
        pub extern "C" fn _dynamic_plugin_signature() -> u64 {
            #hash
        }

        #hash_debug

        #(#functions)*
    }
    .into()
}

fn hash_type<H: Hasher>(hasher: &mut H, ty: Type) {
    match ty {
        Type::Array(inner) => {
            "arr".hash(hasher);
            hash_type(hasher, *inner.elem);
        },
        Type::BareFn(_) => panic!("Bare functions are not supported in plugin interfaces"),
        Type::Group(inner) => hash_type(hasher, *inner.elem),
        Type::ImplTrait(_) => panic!("Traits are supported in plugin interfaces"),
        Type::Infer(_) => panic!("Compiler inference is supported in plugin interfaces"),
        Type::Macro(_) => panic!("Macros are not supported in plugin interfaces"),
        Type::Never(_) => "never".hash(hasher),
        Type::Paren(inner) => hash_type(hasher, *inner.elem),
        Type::Path(inner) => {
            if inner.qself.is_some() {
                panic!("Qualified types are not supported in plugin interfaces");
            }
            // Hash only last segment
            let last_segment = inner.path.segments.last().unwrap();
            if !last_segment.arguments.is_none() {
                panic!("Types cannot be generic or require lifetimes in plugin interfaces");
            }
            last_segment.ident.hash(hasher);
        },
        Type::Ptr(inner) => hash_type(hasher, *inner.elem),
        Type::Reference(_) => panic!("References are not supported in plugin interfaces (use raw pointers instead)"),
        Type::Slice(_) => panic!("Slices are not supported in plugin interfaces (use raw pointers instead)"),
        Type::TraitObject(_) => panic!("Trait objects not supported in plugin interfaces"),
        Type::Tuple(_) => panic!("Tuples not supported in plugin interfaces"),
        Type::Verbatim(_) => panic!("This type is not supported in plugin interfaces"),
        _ => todo!("This type is not yet supported by dynamic-plugin"),
    }
}
