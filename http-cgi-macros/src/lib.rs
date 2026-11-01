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

    let mut params = Vec::new();
    for arg in inputs {
        match arg {
            syn::FnArg::Typed(pat_type) => params.push((&pat_type.pat, &pat_type.ty)),
            syn::FnArg::Receiver(receiver) => {
                return syn::Error::new_spanned(
                    receiver,
                    "`#[http_cgi::main]` does not support methods with `self`",
                )
                .to_compile_error()
                .into();
            }
        }
    }

    let mut extractors: Vec<_> = params[..params.len().saturating_sub(1)]
        .iter()
        .map(|(name, ty)| {
            quote! {
                let #name = match <#ty as http_cgi::request::FromRequestParts>::from_request_parts(&__parts) {
                    Ok(value) => value,
                    // Infallible extractors have an uninhabited error type, so
                    // rustc flags this call as unreachable. It is the only
                    // thing the arm can do; the allow is scoped to the call.
                    Err(error) => {
                        #[allow(unreachable_code)]
                        return http_cgi::response::IntoResponse::into_response(error);
                    }
                };
            }
        })
        .collect();

    if !params.is_empty() {
        let (name, ty) = params[params.len() - 1];
        extractors.push(quote! {
            let #name = match <#ty as http_cgi::request::FromRequest>::from_request(http_cgi::request::Request::from_parts(__parts, __body)) {
                Ok(value) => value,
                // Same `Infallible` caveat as the parts extractors.
                Err(error) => {
                    #[allow(unreachable_code)]
                    return http_cgi::response::IntoResponse::into_response(error);
                }
            };
        });
    }

    let param_names = params.iter().map(|(name, _)| name);

    quote! {
        fn #internal_name(#inputs) #output #block

        fn main() {
            let __response = http_cgi::run::catch(__http_cgi_run);
            if let Err(__error) =
                http_cgi::response::write_response(__response, std::io::stdout().lock())
            {
                eprintln!("http-cgi: failed to write response: {__error:?}");
                std::process::exit(1);
            }
        }

        fn __http_cgi_run() -> http_cgi::response::Response {
            let __env = std::env::vars_os();
            let __reader = std::io::stdin();
            let __request = match http_cgi::request::read_request(__env, __reader.lock()) {
                Ok(request) => request,
                Err(error) => {
                    return http_cgi::response::IntoResponse::into_response(error);
                }
            };
            let (__parts, __body) = __request.into_parts();

            #(#extractors)*

            <_ as http_cgi::response::IntoResponse>::into_response(
                #internal_name(#(#param_names),*),
            )
        }
    }
    .into()
}
