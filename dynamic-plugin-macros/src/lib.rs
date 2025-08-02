#![deny(missing_docs)]
#![warn(clippy::pedantic)]

//! # Macros for the [`dynamic-plugin`](https://docs.rs/dynamic-plugin/latest/dynamic_plugin/) crate.

use std::hash::{Hash, Hasher};

use def::PluginDefinition;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use proc_macro_error2::{abort, proc_macro_error};
use quote::quote;
use syn::{parse_macro_input, FnArg, Lit, ReturnType, Type};

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
#[proc_macro_error]
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

        let fn_checks = plugin_def.functions.iter().map(|f| {
            let name_bytes = f.name.to_string();
            quote! {
                let _: ::dynamic_plugin::PluginLibrarySymbol<unsafe extern fn()> =
                    library.get(#name_bytes.as_bytes()).map_err(|_| ::dynamic_plugin::Error::NotAPlugin)?;
            }
        });

        Some(quote! {
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
                /// - [`::dynamic_plugin::Error::NotAPlugin`] if the file provided is determined not to be a compatible plugin, i.e. not having the required functions present and exposed.
                pub fn load_plugin_and_check_compat<P>(path: P) -> ::dynamic_plugin::Result<Self>
                where
                    P: ::std::convert::AsRef<::std::ffi::OsStr>,
                {
                    unsafe {
                        // Attempt to load library
                        let library = ::dynamic_plugin::PluginDynamicLibrary::new(path)?;

                        // Check that each function exists
                        #(#fn_checks)*

                        Ok(Self {
                            library,
                        })
                    }
                }

                #(#funcs)*
            }
        })
    } else {
        None
    };

    let definition =
        {
            let mut s = String::new();
            for def::PluginFunction {
                attributes,
                name,
                arguments,
                return_type,
                ..
            } in &plugin_def.functions
            {
                for attr in attributes {
                    if attr.path().is_ident("doc") {
                        match &attr.meta {
                            syn::Meta::NameValue(inner) => {
                                if inner.path.is_ident("doc") {
                                    if let syn::Expr::Lit(expr) = &inner.value {
                                        if let Lit::Str(doc) = &expr.lit {
                                            s.push_str(&format!("/// {}\n", doc.value().trim()));
                                        }
                                    }
                                }
                            }
                            _ => (),
                        }
                    }
                }
                s.push_str("fn ");
                s.push_str(&name.to_string());
                s.push('(');
                for (idx, arg) in arguments.iter().enumerate() {
                    match arg {
                        FnArg::Receiver(..) => s.push_str("self"),
                        FnArg::Typed(ty) => {
                            s.push_str("_: ");
                            s.push_str(&crate::type_to_string(*ty.ty.clone()).expect(
                                "this should have failed earlier! please open a bug report!",
                            ));
                        }
                    };
                    if idx < arguments.len() - 1 {
                        s.push_str(", ");
                    }
                }
                s.push(')');
                if let ::std::option::Option::Some(ret) = return_type {
                    s.push_str(" -> ");
                    s.push_str(
                        &crate::type_to_string(ret.clone())
                            .expect("this should have failed earlier! please open a bug report!"),
                    );
                }
                s.push_str(r#" { todo!("not yet implemented") }"#);
                s.push('\n');
            }
            s
        };
    let func_sigs = plugin_def.functions.iter().map(|f| {
        let func_name = f.name.to_string();
        let args = f.arguments.iter().map(|a| match a {
            FnArg::Receiver(..) => "self".to_string(),
            FnArg::Typed(ty) => crate::type_to_string(*ty.ty.clone())
                .expect("this should have failed earlier! please open a bug report!"),
        });
        let return_typ = if let Some(ty) = f
            .return_type
            .as_ref()
            .map(|ty| crate::type_to_string(ty.clone()))
        {
            quote!(::std::option::Option::Some(#ty))
        } else {
            quote!(::std::option::Option::None)
        };
        quote! {
            (#func_name, &[#(#args),*], #return_typ)
        }
    });

    quote! {
        pub struct #plugin_ident {
            library: ::dynamic_plugin::PluginDynamicLibrary,
        }

        impl #plugin_ident {
            /// The signature of this plugin. This number is dependent
            /// on the functions, their arguments and their return
            /// types. Two plugins with the same signature are *likely*
            /// to be compatible.
            pub const PLUGIN_SIGNATURE: u64 = #hash;
            /// The plugin definition is a string which defines an empty
            /// Rust definition of the plugin. It is used to generate
            /// useful error messages.
            pub const PLUGIN_DEFINITION: &str = #definition;
            /// The functions and their signatures. Each tuple holds
            /// (function name, [arguments], maybe return type)
            pub const PLUGIN_FUNCTIONS: &[(&'static str, &[&'static str], ::std::option::Option<&'static str>)] = &[
                #(#func_sigs),*
            ];
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
        let unsafe_ = maybe_unsafe_func._unsafe;
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
        ::dynamic_plugin::static_assert!(
            #target_plugin::PLUGIN_SIGNATURE == #hash,
            ::dynamic_plugin::const_concat!(
                "\nThe implementation does not match the definition:\n\n",
                #target_plugin::PLUGIN_DEFINITION
            )
        );

        #[no_mangle]
        pub extern "C" fn _dynamic_plugin_signature() -> u64 {
            #hash
        }

        #hash_debug

        #(#functions)*
    }
    .into()
}

/// Convert a type to string, returning None if the macro would be
/// failing elsewhere
fn type_to_string(ty: Type) -> Option<String> {
    match ty {
        Type::Array(inner) => Some(format!("[{}]", type_to_string(*inner.elem)?)),
        Type::BareFn(inner) => {
            let mut s = String::new();
            s.push_str(r#"unsafe extern "C" fn("#);
            let has_inputs = !inner.inputs.is_empty();
            for inp in inner.inputs {
                s.push_str(&type_to_string(inp.ty)?);
                s.push_str(", ");
            }
            if has_inputs {
                // remove last ", "
                s.pop();
                s.pop();
            }
            s.push(')');
            if inner.variadic.is_some() {
                return None;
            }
            match inner.output {
                ReturnType::Default => (),
                ReturnType::Type(_, ty) => s.push_str(&format!("-> {}", type_to_string(*ty)?)),
            }
            Some(s)
        }
        Type::Group(inner) => type_to_string(*inner.elem),
        Type::Paren(inner) => type_to_string(*inner.elem),
        Type::Ptr(inner) => type_to_string(*inner.elem),
        Type::Never(_) => Some("!".to_string()),
        Type::Path(inner) => {
            if inner.qself.is_some() {
                return None;
            }
            // Hash only last segment
            let last_segment = inner.path.segments.last().unwrap();
            if !last_segment.arguments.is_none() {
                return None;
            }
            Some(last_segment.ident.to_string())
        }
        Type::ImplTrait(_)
        | Type::Infer(_)
        | Type::Macro(_)
        | Type::Reference(_)
        | Type::Slice(_)
        | Type::TraitObject(_)
        | Type::Tuple(_)
        | Type::Verbatim(_) => None,
        _ => todo!("This type is not yet supported by dynamic-plugin"),
    }
}

fn hash_type<H: Hasher>(hasher: &mut H, ty: Type) {
    match ty {
        Type::Array(inner) => {
            "arr".hash(hasher);
            hash_type(hasher, *inner.elem);
        }
        Type::BareFn(inner) => {
            "fn".hash(hasher);
            for inp in inner.inputs {
                hash_type(hasher, inp.ty);
            }
            if inner.variadic.is_some() {
                abort!(
                    inner.variadic,
                    "Bare functions with variadics are not supported in plugin interfaces"
                );
            }
            "->".hash(hasher);
            match inner.output {
                ReturnType::Default => "()".hash(hasher),
                ReturnType::Type(_, ty) => hash_type(hasher, *ty),
            }
            ";".hash(hasher);
        }
        Type::Group(inner) => hash_type(hasher, *inner.elem),
        Type::ImplTrait(inner) => abort!(inner, "Traits are supported in plugin interfaces"),
        Type::Infer(inner) => abort!(
            inner,
            "Compiler inference is supported in plugin interfaces"
        ),
        Type::Macro(inner) => abort!(inner, "Macros are not supported in plugin interfaces"),
        Type::Never(_) => "never".hash(hasher),
        Type::Paren(inner) => hash_type(hasher, *inner.elem),
        Type::Path(inner) => {
            if inner.qself.is_some() {
                abort!(
                    inner,
                    "Qualified types are not supported in plugin interfaces"
                );
            }
            // Hash only last segment
            let last_segment = inner.path.segments.last().unwrap();
            if !last_segment.arguments.is_none() {
                abort!(
                    last_segment.arguments,
                    "Types cannot be generic or require lifetimes in plugin interfaces"
                );
            }
            last_segment.ident.hash(hasher);
        }
        Type::Ptr(inner) => hash_type(hasher, *inner.elem),
        Type::Reference(inner) => abort!(
            inner,
            "References are not supported in plugin interfaces (use raw pointers instead)"
        ),
        Type::Slice(inner) => abort!(
            inner,
            "Slices are not supported in plugin interfaces (use raw pointers instead)"
        ),
        Type::TraitObject(inner) => {
            abort!(inner, "Trait objects not supported in plugin interfaces")
        }
        Type::Tuple(inner) => abort!(inner, "Tuples not supported in plugin interfaces"),
        Type::Verbatim(inner) => abort!(inner, "This type is not supported in plugin interfaces"),
        _ => todo!("This type is not yet supported by dynamic-plugin"),
    }
}
