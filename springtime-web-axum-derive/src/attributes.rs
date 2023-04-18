use syn::parse::{Parse, ParseStream};
use syn::{Error, ExprArray, LitStr, Token};

#[derive(Default)]
pub struct ControllerAttributes {
    pub path: Option<LitStr>,
    pub server_names: Option<ExprArray>,
}

impl Parse for ControllerAttributes {
    //noinspection DuplicatedCode
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut result = Self::default();
        while !input.is_empty() {
            let lookahead = input.lookahead1();
            if lookahead.peek(kw::path) {
                if result.path.is_some() {
                    return Err(Error::new(input.span(), "Path is already defined!"));
                }

                result.path = Some(input.parse::<LitArg<kw::path, LitStr>>()?.value);
            } else if lookahead.peek(kw::server_names) {
                if result.server_names.is_some() {
                    return Err(Error::new(
                        input.span(),
                        "Server names are already defined!",
                    ));
                }

                result.server_names =
                    Some(input.parse::<LitArg<kw::server_names, ExprArray>>()?.value);
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

//noinspection DuplicatedCode
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

    custom_keyword!(path);
    custom_keyword!(server_names);
}
