use std::hash::{Hash, Hasher};

use syn::{
    parse::{Parse, ParseStream},
    FnArg, ItemFn, Result, ReturnType, Token, TypePath,
};

pub struct PluginImplementation {
    pub target_plugin: TypePath,
    pub functions: Vec<MaybeUnsafeFn>,
}

impl Hash for PluginImplementation {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash name
        let type_ident = self
            .target_plugin
            .path
            .segments
            .last()
            .unwrap()
            .ident
            .clone();
        type_ident.hash(state);

        // Sort functions
        let mut functions = self.functions.clone();
        functions.sort_by(|a, b| a.func.sig.ident.cmp(&b.func.sig.ident));
        for maybe_unsafe_func in functions {
            let function = maybe_unsafe_func.func;
            "fn".hash(state);
            // Hash function ident
            function.sig.ident.hash(state);

            for inp in function.sig.inputs {
                // Hash argument types only
                if let FnArg::Typed(typed) = inp {
                    let ty = typed.ty;
                    "arg".hash(state);
                    crate::hash_type(state, *ty);
                }
            }

            // Hash return type
            if let ReturnType::Type(_, ty) = function.sig.output {
                "ret".hash(state);
                crate::hash_type(state, *ty);
            }
        }
    }
}

impl Parse for PluginImplementation {
    fn parse(input: ParseStream) -> Result<Self> {
        let target_plugin = input.parse()?;
        let _: Token![,] = input.parse()?;
        let mut functions = vec![];
        while !input.is_empty() {
            functions.push(input.parse()?);
        }

        Ok(Self {
            target_plugin,
            functions,
        })
    }
}

#[derive(Clone)]
pub struct MaybeUnsafeFn {
    pub _unsafe: Option<Token![unsafe]>,
    pub func: ItemFn,
}

impl Parse for MaybeUnsafeFn {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            _unsafe: input.parse()?,
            func: input.parse()?,
        })
    }
}
