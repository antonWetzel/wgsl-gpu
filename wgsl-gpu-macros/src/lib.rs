extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{quote, quote_spanned};
use syn::DeriveInput;

#[proc_macro]
pub fn make_answer(_item: TokenStream) -> TokenStream {
    "fn answer() -> u32 { 42 }".parse().unwrap()
}

#[proc_macro_derive(WgslGpuArguments, attributes(wgsl_gpu))]
pub fn wgsl_gpu_arguments(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);
    wgsl_gpu_arguments_inner(input).into()
}

fn wgsl_gpu_arguments_inner(input: DeriveInput) -> proc_macro2::TokenStream {
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
        impl wgsl_gpu::WglsGpuArguments for #name {
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
        macro_rules! #macro_name {
            // arg matches, transform and return to main macro
            ($target:path, ($($context:tt)*), $macros:tt, ($name:ident: #name)) => {
                $target!($($context)*, (#( #[spirv(#attributes)] #tuple_arg: #tuple_types),*), (wgsl_gpu::WglsGpuArguments::from_arguments((#(#tuple_fields),*))),);
            };
            // no match, contiue with other tranform macros
            ($target:path, $context:tt, ($macro:path, $($macro_tail:tt)*), $arg:tt) => {
                $macro!($target, $context, ($($macro_tail)*), $arg);
            };
            // get arguments for return values
            ($target:path, ($($context:tt)*)) => {
                $target!($($context)*, (#( #[spirv(#ret_attributes)] #ret_fields: &mut #ret_types),*), output, (#(*#ret_fields_asign = output.#ret_field_read;)*));
            };
        }
    }
}
