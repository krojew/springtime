use syn::{Attribute, Error, ExprPath, LitStr, Token};

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
    pub name: Option<LitStr>,
    pub is_primary: bool,
}

impl TryFrom<&Attribute> for ComponentAttributes {
    type Error = Error;

    fn try_from(value: &Attribute) -> Result<Self, Self::Error> {
        let mut name = None;
        let mut is_primary = false;
        value.parse_nested_meta(|meta| {
            if meta.path.is_ident("name") {
                name = Some(meta.value().and_then(|value| value.parse())?);
            } else if meta.path.is_ident("primary") {
                is_primary = true;
            }

            Ok(())
        })?;

        Ok(Self { name, is_primary })
    }
}
