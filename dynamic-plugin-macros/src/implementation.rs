use std::hash::Hash;

use syn::{
    parse::{Parse, ParseStream},
    FnArg, ItemFn, Result, ReturnType, Token, TypePath,
};

pub struct PluginImplementation {
    pub target_plugin: TypePath,
    pub functions: Vec<ItemFn>,
}

impl Hash for PluginImplementation {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
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
        functions.sort_by(|a, b| a.sig.ident.cmp(&b.sig.ident));
        for function in functions {
            // Hash function ident
            function.sig.ident.hash(state);

            for inp in function.sig.inputs {
                // Hash argument types only
                if let FnArg::Typed(typed) = inp {
                    let ty = typed.ty;
                    ty.hash(state);
                }
            }

            // Hash return type
            if let ReturnType::Type(_, ty) = function.sig.output {
                ty.hash(state);
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
