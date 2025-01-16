use std::hash::{Hash, Hasher};

use syn::{
    braced, parenthesized,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Attribute, FnArg, Ident, Result, Token, Type,
};

pub struct PluginDefinition {
    pub name: Ident,
    pub functions: Vec<PluginFunction>,
}

impl Hash for PluginDefinition {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash name
        self.name.hash(state);

        // Sort functions
        let mut functions = self.functions.clone();
        functions.sort_by(|a, b| a.name.cmp(&b.name));
        for function in functions {
            // Hash function ident
            function.name.hash(state);

            for inp in function.arguments {
                // Hash argument types only
                if let FnArg::Typed(typed) = inp {
                    let ty = typed.ty;
                    ty.hash(state);
                }
            }

            // Hash return type
            if let Some(ty) = function.return_type {
                ty.hash(state);
            }
        }
    }
}

impl Parse for PluginDefinition {
    fn parse(input: ParseStream) -> Result<Self> {
        let _: Token![extern] = input.parse()?;
        let _: Token![trait] = input.parse()?;
        let name = input.parse()?;
        let plugin_content;
        braced!(plugin_content in input);

        let mut functions = vec![];

        while !plugin_content.is_empty() {
            let lookahead = plugin_content.lookahead1();
            let mut attrs = vec![];
            if lookahead.peek(Token![#]) {
                // Parse attributes
                attrs = Attribute::parse_outer(&plugin_content)?;
            }
            // Parse as function
            let _: Token![fn] = plugin_content.parse()?;
            let fn_name = plugin_content.parse()?;
            let args_content;
            parenthesized!(args_content in plugin_content);
            let vars: Punctuated<FnArg, Token![,]> =
                args_content.parse_terminated(FnArg::parse, Token![,])?;

            let mut return_type = None;
            let lookahead = plugin_content.lookahead1();
            if lookahead.peek(Token![->]) {
                let _: Token![->] = plugin_content.parse()?;
                return_type = Some(plugin_content.parse()?);
                let _: Token![;] = plugin_content.parse()?;
            } else if lookahead.peek(Token![;]) {
                let _: Token![;] = plugin_content.parse()?;
            } else {
                return Err(lookahead.error());
            }

            functions.push(PluginFunction {
                attributes: attrs,
                name: fn_name,
                arguments: vars.into_iter().collect(),
                return_type,
            })
        }

        Ok(Self { name, functions })
    }
}

#[derive(Clone)]
pub struct PluginFunction {
    pub attributes: Vec<Attribute>,
    pub name: Ident,
    pub arguments: Vec<FnArg>,
    pub return_type: Option<Type>,
}
