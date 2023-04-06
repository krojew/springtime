use itertools::Itertools;
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{Attribute, Error, Expr, ExprArray, ExprLit, ExprPath, Lit, LitInt, LitStr, Token};

pub enum DefaultDefinition {
    Default,
    Expr(ExprPath),
}

pub struct FieldAttributes {
    pub default: Option<DefaultDefinition>,
    pub name: Option<LitStr>,
    pub ignore: bool,
}

impl TryFrom<&Attribute> for FieldAttributes {
    type Error = Error;

    fn try_from(value: &Attribute) -> Result<Self, Self::Error> {
        let mut default = None;
        let mut name = None;
        let mut ignore = false;

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
            } else if meta.path.is_ident("ignore") {
                ignore = true;
            }

            Ok(())
        })?;

        Ok(Self {
            default,
            name,
            ignore,
        })
    }
}

pub struct ConstructorParameter {
    pub component_type: String,
    pub name: Option<String>,
}

impl From<&str> for ConstructorParameter {
    fn from(value: &str) -> Self {
        if let Some((component_type, name)) = value.splitn(2, '/').collect_tuple() {
            ConstructorParameter {
                component_type: component_type.to_string(),
                name: Some(name.to_string()),
            }
        } else {
            ConstructorParameter {
                component_type: value.to_string(),
                name: None,
            }
        }
    }
}

#[derive(Default)]
pub struct ComponentAttributes {
    pub names: Option<ExprArray>,
    pub condition: Option<ExprPath>,
    pub priority: i8,
    pub scope: Option<LitStr>,
    pub constructor: Option<ExprPath>,
    pub constructor_parameters: Vec<ConstructorParameter>,
}

impl ComponentAttributes {
    fn parse_constructor_parameters(value: &LitStr) -> Vec<ConstructorParameter> {
        let value = value.value();
        value.split(',').map_into().collect()
    }
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
            } else if meta.path.is_ident("priority") {
                if let Expr::Lit(ExprLit {
                    lit: Lit::Int(priority),
                    ..
                }) = meta.value()?.parse::<Expr>()?
                {
                    result.priority = priority.base10_parse()?;
                }
            } else if meta.path.is_ident("constructor") {
                if result.constructor.is_some() {
                    return Err(Error::new(value.span(), "Constructor is already defined!"));
                }

                if let Expr::Lit(ExprLit {
                    lit: Lit::Str(string),
                    ..
                }) = meta.value()?.parse::<Expr>()?
                {
                    result.constructor = Some(string.parse()?);
                }
            } else if meta.path.is_ident("scope") {
                if result.scope.is_some() {
                    return Err(Error::new(value.span(), "Scope type is already defined!"));
                }

                if let Expr::Lit(ExprLit {
                    lit: Lit::Str(string),
                    ..
                }) = meta.value()?.parse::<Expr>()?
                {
                    result.scope = Some(string);
                }
            } else if meta.path.is_ident("constructor_parameters") {
                if !result.constructor_parameters.is_empty() {
                    return Err(Error::new(
                        value.span(),
                        "Constructor parameters are already defined!",
                    ));
                }

                if let Expr::Lit(ExprLit {
                    lit: Lit::Str(string),
                    ..
                }) = meta.value()?.parse::<Expr>()?
                {
                    result.constructor_parameters = Self::parse_constructor_parameters(&string);
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
    pub priority: i8,
    pub scope: Option<LitStr>,
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

                result.condition = Some(
                    input
                        .parse::<LitArg<kw::condition, LitStr>>()?
                        .value
                        .parse()?,
                );
            } else if lookahead.peek(kw::priority) {
                result.priority = input
                    .parse::<LitArg<kw::priority, LitInt>>()?
                    .value
                    .base10_parse()?;
            } else if lookahead.peek(kw::scope) {
                if result.scope.is_some() {
                    return Err(Error::new(input.span(), "Scope type is already defined!"));
                }

                result.scope = Some(input.parse::<LitArg<kw::scope, LitStr>>()?.value);
            } else if lookahead.peek(Token![,]) {
                let _ = input.parse::<Token![,]>()?;
            } else {
                return Err(lookahead.error());
            }
        }

        Ok(result)
    }
}

struct LitArg<T, A> {
    value: A,
    _p: std::marker::PhantomData<T>,
}

impl<T: Parse, A: Parse> Parse for LitArg<T, A> {
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
    custom_keyword!(priority);
    custom_keyword!(scope);
}
