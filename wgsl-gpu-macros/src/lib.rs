extern crate proc_macro;

use std::ops::Not;

use proc_macro::TokenStream;
use quote::{ToTokens, quote, quote_spanned};
use syn::{DeriveInput, spanned::Spanned};

#[proc_macro_derive(Arguments, attributes(wgsl_gpu))]
pub fn arguments(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);
    arguments_inner(input).into()
}

fn arguments_inner(input: DeriveInput) -> proc_macro2::TokenStream {
    let data = match &input.data {
        syn::Data::Struct(data) => data,
        syn::Data::Enum(data) => {
            return quote_spanned!(data.enum_token.span => compile_eror!("Expected struct, not enum"));
        }
        syn::Data::Union(data) => {
            return quote_spanned!(data.union_token.span => compile_eror!("Expected struct, not union"));
        }
    };

    let name = &input.ident;

    let arg_types = data.fields.iter().map(|field| &field.ty);
    let fields = data
        .fields
        .iter()
        .map(|field| field.ident.as_ref().unwrap());
    let indices = (0..data.fields.len()).map(syn::Index::from);

    let attributes = data.fields.iter().map(|field| {
        field
            .attrs
            .iter()
            .rev()
            .filter(|attribute| attribute.path().is_ident("wgsl_gpu"))
            .find_map(|attribute| match &attribute.meta {
                syn::Meta::List(list) => Some(&list.tokens),
                _ => None,
            })
            .expect("every member requires a wgsl_gpu attribute")
    });

    let tuple_types = arg_types.clone();
    let tuple_fields = fields.clone();
    let tuple_arg = fields.clone();

    let ret_attributes = data.fields.iter().map(|field| {
        field
            .attrs
            .iter()
            .filter(|attribute| attribute.path().is_ident("wgsl_gpu"))
            .find_map(|attribute| match &attribute.meta {
                syn::Meta::List(list) => Some(&list.tokens),
                _ => None,
            })
            .expect("every member requires a wgsl_gpu attribute")
    });
    let ret_types = arg_types.clone();
    let ret_fields = fields.clone();

    let ret_fields_asign = fields.clone();
    let ret_field_read = fields.clone();

    let macro_name = quote::format_ident!("wgsl_gpu_{}_transform", name);

    quote! {
        impl wgsl_gpu::Arguments for #name {
            type Arguments = (#(#arg_types,)*);

            fn from_arguments(arguments: Self::Arguments) -> Self {
                Self {
                    #(
                        #fields: arguments.#indices,
                    )*
                }
            }
        }

        #[spirv_std::macros::spirv_recursive_for_testing]
        #[macro_export]
        #[doc(hidden)]
        macro_rules! #macro_name {
            // get arumnents and parameters
            (__arg, $target:path, $context:tt) => {
                $target!($context, (#( #[spirv(#attributes)] #tuple_arg: #tuple_types),*), (wgsl_gpu::Arguments::from_arguments((#(#tuple_fields),*))),);
            };
            // get arguments for return values
            (__ret, $target:path, $context:tt) => {
                $target!($context, (#( #[spirv(#ret_attributes)] #ret_fields: &mut #ret_types),*), output, (#(*#ret_fields_asign = output.#ret_field_read;)*));
            };
        }
    }
}

#[proc_macro_attribute]
pub fn entry(_arguments: TokenStream, input: TokenStream) -> TokenStream {
    let item = syn::parse_macro_input!(input as syn::Item);
    wgsl_gpu_entry_inner(item).into()
}

fn wgsl_gpu_entry_inner(item: syn::Item) -> proc_macro2::TokenStream {
    let mut item = match item {
        syn::Item::Fn(item) => item,
        _ => return quote_spanned! {item.span() => compile_error!("expected function") },
    };

    let mut attributes = proc_macro2::TokenStream::new();
    item.attrs.retain(|att| {
        if att.path().is_ident("spirv") {
            att.to_tokens(&mut attributes);
            false
        } else {
            true
        }
    });

    let mut inputs = proc_macro2::TokenStream::new();
    for input in item.sig.inputs.iter_mut() {
        let syn::FnArg::Typed(arg) = input else {
            return quote_spanned! {input.span() => compile_error!("self not supported") };
        };
        let arguments = arg
            .attrs
            .iter()
            .filter(|att| att.path().is_ident("wgsl_gpu"))
            .filter_map(|att| match &att.meta {
                syn::Meta::List(list) => Some(&list.tokens),
                _ => None,
            })
            .any(|tokens| {
                tokens.clone().into_iter().all(|token| match token {
                    proc_macro2::TokenTree::Ident(ident) => ident == "arguments",
                    _ => false,
                })
            });
        arg.attrs
            .retain(|att| att.path().is_ident("wgsl_gpu").not());

        let prefix = match arguments {
            false => quote! { __keep => },
            true => {
                let macro_name = transform_macro_name(&arg.ty);
                quote! { __expand #macro_name => }
            }
        };
        inputs.extend(quote! { #prefix (#arg), });

        arg.attrs.retain(|att| att.path().is_ident("spirv").not());
    }

    let ident = &item.sig.ident;
    let ident_gpu = quote::format_ident!("{}_gpu", ident);

    // let arg_macros = signature.inputs.iter().map(|ty| transform_macro_name(ty));
    let ret_macro = match &item.sig.output {
        syn::ReturnType::Default => panic!("return type required"),
        syn::ReturnType::Type(_, ty) => transform_macro_name(ty),
    };

    quote! {
        #item

        wgsl_gpu::create_wrapper_function!(
            (#attributes pub fn #ident_gpu), #ident,
            #ret_macro,
            (
                #inputs
            ),
        );
    }
}

fn transform_macro_name(ty: &syn::Type) -> syn::Type {
    let mut ty = ty.clone();
    let syn::Type::Path(path) = &mut ty else {
        panic!("type must be a path")
    };
    if let Some(last) = path.path.segments.last_mut() {
        last.ident = quote::format_ident!("wgsl_gpu_{}_transform", &last.ident);
    }
    ty
}
