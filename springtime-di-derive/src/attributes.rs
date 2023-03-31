use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{Attribute, Error, Expr, ExprArray, ExprLit, ExprPath, Lit, LitStr, Token};

pub enum DefaultDefinition {
    Default,
    Expr(ExprPath),
}

pub struct FieldAttributes {
    pub default: Option<DefaultDefinition>,
    pub name: Option<LitStr>,
}

impl TryFrom<&Attribute> for FieldAttributes {
    type Error = Error;

    fn try_from(value: &Attribute) -> Result<Self, Self::Error> {
        let mut default = None;
        let mut name = None;

        value.parse_nested_meta(|meta| {
            if meta.path.is_ident("default") {
                if name.is_some() {
                    return Err(Error::new(
                        value.span(),
                        "Cannot use default value when injecting a named instance!",
                    ));
                }

                if meta.input.peek(Token![=]) {
                    let value = meta.value()?;
                    let expr: LitStr = value.parse()?;
                    default = Some(DefaultDefinition::Expr(expr.parse()?));
                } else {
                    default = Some(DefaultDefinition::Default);
                }
            } else if meta.path.is_ident("name") {
                if default.is_some() {
                    return Err(Error::new(
                        value.span(),
                        "Cannot inject a named instance if using the default value!",
                    ));
                }

                let value = meta.value()?;
                name = Some(value.parse()?);
            }

            Ok(())
        })?;

        Ok(Self { default, name })
    }
}

#[derive(Default)]
pub struct ComponentAttributes {
    pub names: Option<ExprArray>,
    pub condition: Option<ExprPath>,
}

impl TryFrom<&Attribute> for ComponentAttributes {
    type Error = Error;

    fn try_from(value: &Attribute) -> Result<Self, Self::Error> {
        let mut result = Self::default();
        value.parse_nested_meta(|meta| {
            if meta.path.is_ident("names") {
                if result.names.is_some() {
                    return Err(Error::new(value.span(), "Names are already defined!"));
                }

                if let Expr::Array(array) = meta.value()?.parse::<Expr>()? {
                    result.names = Some(array);
                }
            } else if meta.path.is_ident("condition") {
                if result.condition.is_some() {
                    return Err(Error::new(value.span(), "Condition is already defined!"));
                }

                if let Expr::Lit(ExprLit {
                    lit: Lit::Str(path),
                    ..
                }) = meta.value()?.parse::<Expr>()?
                {
                    result.condition = Some(path.parse()?);
                }
            }

            Ok(())
        })?;

        Ok(result)
    }
}

#[derive(Default)]
pub struct ComponentAliasAttributes {
    pub is_primary: bool,
    pub condition: Option<ExprPath>,
}

impl Parse for ComponentAliasAttributes {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut result = Self::default();
        while !input.is_empty() {
            let lookahead = input.lookahead1();
            if lookahead.peek(kw::primary) {
                result.is_primary = true;
                input.parse::<kw::primary>()?;
            } else if lookahead.peek(kw::condition) {
                if result.condition.is_some() {
                    return Err(Error::new(input.span(), "Condition is already defined!"));
                }

                result.condition = Some(input.parse::<StrArg<kw::condition>>()?.value.parse()?);
            } else if lookahead.peek(Token![,]) {
                let _ = input.parse::<Token![,]>()?;
            } else {
                return Err(lookahead.error());
            }
        }

        Ok(result)
    }
}

struct StrArg<T> {
    value: LitStr,
    _p: std::marker::PhantomData<T>,
}

impl<T: Parse> Parse for StrArg<T> {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let _ = input.parse::<T>()?;
        let _ = input.parse::<Token![=]>()?;
        let value = input.parse()?;
        Ok(Self {
            value,
            _p: std::marker::PhantomData,
        })
    }
}

mod kw {
    use syn::custom_keyword;

    custom_keyword!(primary);
    custom_keyword!(condition);
}
