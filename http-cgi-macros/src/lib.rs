use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::parse_macro_input;

#[proc_macro_attribute]
pub fn main(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let func = parse_macro_input!(item as syn::ItemFn);

    let original_name = &func.sig.ident;
    let internal_name = format_ident!("__{}", original_name);
    let block = &func.block;
    let inputs = &func.sig.inputs;
    let output = &func.sig.output;

    let params: Vec<_> = inputs
        .iter()
        .map(|arg| match arg {
            syn::FnArg::Typed(pat_type) => {
                let name = &pat_type.pat;
                let ty = &pat_type.ty;
                (name, ty)
            }
            syn::FnArg::Receiver(_) => panic!("self not supported"),
        })
        .collect();

    let mut extractors: Vec<_> = params[..params.len().saturating_sub(1)]
        .iter()
        .map(|(name, ty)| {
            quote! {
                let #name = match <#ty as http_cgi::request::FromRequestParts>::from_request_parts(&__parts) {
                    Ok(v) => v,
                    Err(_) => todo!()
                };
            }
        })
        .collect();
    if params.len() > 0 {
        let (name, ty) = params[params.len() - 1];
        extractors.push(quote! {
            let #name = match <#ty as http_cgi::request::FromRequest>::from_request(http_cgi::request::Request::from_parts(__parts, __body)) {
                Ok(v) => v,
                Err(_) => todo!()
            };
        });
    }

    let param_names = params.iter().map(|(name, _)| name);

    quote! {
        fn #internal_name(#inputs) #output #block

        fn main() {
            let __env = std::env::vars_os();
            let __reader = std::io::stdin();
            let __req = http_cgi::request::read_request(__env, __reader.lock()).unwrap();
            let (__parts, __body) = __req.into_parts();

            #(#extractors)*

            let __res = <_ as http_cgi::response::IntoResponse>::into_response(#internal_name(#(#param_names),*));
            let __writer = std::io::stdout();
            http_cgi::response::write_response(__res, __writer.lock()).unwrap();
        }
    }
    .into()
}
