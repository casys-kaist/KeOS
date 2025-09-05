use proc_macro::TokenStream;
use quote::quote_spanned;
use syn::{ItemFn, LitByteStr, LitInt, parse_macro_input, spanned::Spanned};

#[proc_macro_attribute]
pub fn stdin(attr: TokenStream, item: TokenStream) -> TokenStream {
    let ident = parse_macro_input!(attr as LitByteStr);
    let mut input_fn = parse_macro_input!(item as ItemFn);
    let block = *input_fn.block;
    *input_fn.block = syn::parse_quote! {
        {
            keos::thread::with_current(|th| th.hook_stdin(#ident));
            {
                #block
            }
        }
    };
    TokenStream::from(quote_spanned! { input_fn.span() =>
        #input_fn
    })
}

#[proc_macro_attribute]
pub fn assert_exit_code(attr: TokenStream, item: TokenStream) -> TokenStream {
    let code = parse_macro_input!(attr as LitInt);
    let mut input_fn = parse_macro_input!(item as ItemFn);
    let block = *input_fn.block;
    *input_fn.block = syn::parse_quote! {
        {
            fn _f() {}
            fn _get_name<T>(_: T) -> &'static str {
                let n = core::any::type_name::<T>();
                &n[..n.len() - 4]
            }
            let mut builder = keos::thread::ThreadBuilder::new(_get_name(_f));
            if let Some(task) = keos::thread::with_current(|th| th.task.as_ref().map(|t| t.new_boxed())) {
                builder = builder.attach_task(task);
            };
            assert_eq!(
                builder
                    .spawn(move || { #block })
                    .join(),
            #code);
        }
    };
    TokenStream::from(quote_spanned! { input_fn.span() =>
        #input_fn
    })
}

#[proc_macro_attribute]
pub fn assert_output(attr: TokenStream, item: TokenStream) -> TokenStream {
    let output = parse_macro_input!(attr as LitByteStr);
    let mut input_fn = parse_macro_input!(item as ItemFn);
    let block = *input_fn.block;
    *input_fn.block = syn::parse_quote! {
        {
            let _return_val = (move || { #block })();
            if let Some(output) = keos::thread::with_current(|th| th.finish_hook()) {
                assert_eq!(Ok(output.as_str()), core::str::from_utf8(#output));
            } else {
                panic!("Output is not hooked.\nTo hook the output, stdin must be hooked.");
            }
            _return_val
        }
    };
    TokenStream::from(quote_spanned! { input_fn.span() =>
        #input_fn
    })
}

#[proc_macro_attribute]
pub fn validate_alloc(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut input_fn = parse_macro_input!(item as ItemFn);
    let block = *input_fn.block;
    *input_fn.block = syn::parse_quote! {
        {
            keos::thread::with_current(|th| th.track_alloc());
            let _return_val = (move || { #block })();
            keos::thread::with_current(|th| th.validate_alloc());
            _return_val
        }
    };
    TokenStream::from(quote_spanned! { input_fn.span() =>
        #input_fn
    })
}
