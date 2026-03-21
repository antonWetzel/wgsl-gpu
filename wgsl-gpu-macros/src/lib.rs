extern crate proc_macro;

use std::ops::{Deref, Not};

use proc_macro::TokenStream;
use quote::{ToTokens, quote, quote_spanned};
use syn::{DeriveInput, parenthesized, spanned::Spanned};

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

    let generate_attributes = input
        .attrs
        .iter()
        .filter(|attribute| attribute.path().is_ident("wgsl_gpu"))
        .filter_map(|attribute| match &attribute.meta {
            syn::Meta::List(list) => Some(list.tokens.clone()),
            _ => None,
        })
        .filter_map(|tokens| syn::parse2::<syn::Meta>(tokens).ok())
        .any(|meta| meta.path().is_ident("attributes"));

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

    let locations = generate_attributes.then(|| {
        let attrs = data.fields.iter().map(|field| {
            let location = field
                .attrs
                .iter()
                .filter(|attribute| attribute.path().is_ident("wgsl_gpu"))
                .filter_map(|attribute| match &attribute.meta {
                    syn::Meta::List(list) => Some(list.tokens.clone()),
                    _ => None,
                })
                .filter_map(|tokens| syn::parse2::<syn::Meta>(tokens).ok())
                .filter(|meta| meta.path().is_ident("location"))
                .find_map(|meta| match meta {
                    syn::Meta::NameValue(named) => Some(named.value),
                    _ => None,
                })
                .expect("attributes require locations");
            let ty = match &field.ty {
                syn::Type::Path(path) => {
                    let name = path.path.segments.last().unwrap().ident.to_string();
                    match name.as_str() {
                        "f32" => quote::format_ident!("Float32"),
                        "Vec2" => quote::format_ident!("Float32x2"),
                        "Vec3" => quote::format_ident!("Float32x3"),
                        "Vec4" => quote::format_ident!("Float32x4"),
                        "u32" => quote::format_ident!("Uint32"),
                        "UVec2" => quote::format_ident!("Uint32x2"),
                        "UVec3" => quote::format_ident!("Uint32x3"),
                        "UVec4" => quote::format_ident!("Uint32x4"),
                        _ => panic!("invalid type for attributes"),
                    }
                }
                _ => panic!("invalid type for attributes"),
            };
            quote! { #location => #ty }
        });

        quote! {
            #[cfg(feature = "native")]
            impl #name {
                pub const ATTRIBUTES: &[wgpu::VertexAttribute] = wgpu::vertex_attr_array![
                    #( #attrs, )*
                ]
                    .as_slice();
            }
        }
    });

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

        #locations

        #[spirv_std::macros::spirv_recursive_for_testing]
        #[macro_export]
        #[doc(hidden)]
        macro_rules! #macro_name {
            // get arumnents and parameters
            (__arg, $target:path, $context:tt) => {
                $target!($context, (#( #[spirv(#attributes)] #tuple_arg: #tuple_types),*), (wgsl_gpu::Arguments::from_arguments((#(#tuple_fields,)*))),);
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
    match wgsl_gpu_entry_inner(item) {
        Ok(tokens) => tokens.into(),
        Err(tokens) => tokens.into_compile_error().into(),
    }
}

fn wgsl_gpu_entry_inner(item: syn::Item) -> Result<proc_macro2::TokenStream, syn::Error> {
    let mut item = match item {
        syn::Item::Fn(item) => item,
        _ => return Err(syn::Error::new(item.span(), "expected function")),
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

    let mut step_modes = Vec::new();
    let mut argument_types = Vec::new();

    for input in item.sig.inputs.iter_mut() {
        let syn::FnArg::Typed(arg) = input else {
            return Err(syn::Error::new(input.span(), "self not supported"));
        };

        let att = arg
            .attrs
            .iter()
            .find(|att| att.path().is_ident("wgsl_gpu"))
            .map(|att| att.clone());

        arg.attrs
            .retain(|att| att.path().is_ident("wgsl_gpu").not());

        let prefix = if let Some(att) = att {
            let mut arguments = false;
            att.parse_nested_meta(|meta| {
                if meta.path.is_ident("arguments") {
                    arguments = true;
                } else if meta.path.is_ident("step_mode") {
                    let value = meta.value()?;
                    let ident = value.parse::<syn::Ident>()?;
                    step_modes.push(ident);
                }
                Ok(())
            })?;
            if arguments.not() {
                return Err(syn::Error::new(att.span(), "missing 'arguments' argument"));
            }

            argument_types.push(arg.ty.deref().clone());
            let macro_name = transform_macro_name(&arg.ty);
            quote! { __expand #macro_name => }
        } else {
            quote! { __keep => }
        };

        inputs.extend(quote! { #prefix (#arg), });

        arg.attrs.retain(|att| att.path().is_ident("spirv").not());
    }

    let ident = &item.sig.ident;
    let ident_upper = ident.to_string().to_uppercase();
    let ident_gpu = quote::format_ident!("{}_gpu", ident);
    let ident_gpu_value = ident_gpu.to_string();
    let const_name = quote::format_ident!("{}_NAME", ident_upper);

    let vertex_buffer_layout = if step_modes.is_empty() {
        quote! {}
    } else {
        if step_modes.len() != argument_types.len() {
            return Err(syn::Error::new(
                item.sig.ident.span(),
                "step mode must be provided for all or none arguments",
            ));
        }
        let const_name = quote::format_ident!("{}_VERTEX_BUFFER_LAYOUTS", ident_upper);
        let types = argument_types.iter();
        let step_modes = step_modes.iter();

        quote! {
            #[cfg(feature = "native")]
            pub const #const_name: &[wgpu::VertexBufferLayout] = &[#(
                wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<#types>() as u64,
                    step_mode: wgpu::VertexStepMode::#step_modes,
                    attributes: #types::ATTRIBUTES,
                },
            )*];
        }
    };

    // let arg_macros = signature.inputs.iter().map(|ty| transform_macro_name(ty));
    let ret_macro = match &item.sig.output {
        syn::ReturnType::Default => panic!("return type required"),
        syn::ReturnType::Type(_, ty) => transform_macro_name(ty),
    };

    let tokens = quote! {
        #item

        wgsl_gpu::create_wrapper_function!(
            (#attributes pub fn #ident_gpu), #ident,
            #ret_macro,
            (
                #inputs
            ),
        );

        pub const #const_name: &str = #ident_gpu_value;

        #vertex_buffer_layout
    };

    Ok(tokens)
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
