use syn::parse::{Parse, ParseStream};
use syn::{Attribute, Error, Expr, ExprArray, ExprPath, LitStr, Token};

pub enum DefaultDefinition {
    Default,
    Expr(ExprPath),
}

pub struct FieldAttributes {
    pub default: Option<DefaultDefinition>,
}

impl TryFrom<&Attribute> for FieldAttributes {
    type Error = Error;

    fn try_from(value: &Attribute) -> Result<Self, Self::Error> {
        let mut default = None;
        value.parse_nested_meta(|meta| {
            if meta.path.is_ident("default") {
                if meta.input.peek(Token![=]) {
                    let value = meta.value()?;
                    let expr: LitStr = value.parse()?;
                    default = Some(DefaultDefinition::Expr(expr.parse()?));
                } else {
                    default = Some(DefaultDefinition::Default);
                }
            }

            Ok(())
        })?;

        Ok(Self { default })
    }
}

pub struct ComponentAttributes {
    pub names: Option<ExprArray>,
}

impl TryFrom<&Attribute> for ComponentAttributes {
    type Error = Error;

    fn try_from(value: &Attribute) -> Result<Self, Self::Error> {
        let mut names = None;
        value.parse_nested_meta(|meta| {
            if meta.path.is_ident("names") {
                if let Expr::Array(array) = meta.value()?.parse::<Expr>()? {
                    names = Some(array);
                }
            }

            Ok(())
        })?;

        Ok(Self { names })
    }
}

#[derive(Default)]
pub struct ComponentAliasAttributes {
    pub is_primary: bool,
}

impl Parse for ComponentAliasAttributes {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut result = Self::default();
        while !input.is_empty() {
            let lookahead = input.lookahead1();
            if lookahead.peek(kw::primary) {
                result.is_primary = true;
                let _ = input.parse::<proc_macro2::TokenTree>();
            } else if lookahead.peek(Token![,]) {
                let _ = input.parse::<Token![,]>()?;
            } else {
                return Err(lookahead.error());
            }
        }

        Ok(result)
    }
}

mod kw {
    use syn::custom_keyword;

    custom_keyword!(primary);
}
