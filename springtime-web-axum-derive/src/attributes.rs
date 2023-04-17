use syn::parse::{Parse, ParseStream};
use syn::{Error, LitStr, Token};

#[derive(Default)]
pub struct ControllerAttributes {
    pub path: Option<LitStr>,
}

impl Parse for ControllerAttributes {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut result = Self::default();
        while !input.is_empty() {
            let lookahead = input.lookahead1();
            if lookahead.peek(kw::path) {
                if result.path.is_some() {
                    return Err(Error::new(input.span(), "Path is already defined!"));
                }

                result.path = Some(input.parse::<LitArg<kw::path, LitStr>>()?.value);
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
}
