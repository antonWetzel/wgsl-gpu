use std::ops::{Deref, Not};

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::spanned::Spanned;

pub fn entry(item: syn::Item) -> Result<TokenStream, syn::Error> {
    let mut item = match item {
        syn::Item::Fn(item) => item,
        _ => return Err(syn::Error::new(item.span(), "expected function")),
    };

    let mut attributes = proc_macro2::TokenStream::new();
    item.attrs.retain(|att| {
        if att.path().is_ident("spirv").not() {
            return true;
        }
        att.to_tokens(&mut attributes);
        false
    });

    let mut inputs = TokenStream::new();

    let mut step_modes = Vec::new();
    let mut argument_types = Vec::new();

    for input in item.sig.inputs.iter_mut() {
        let syn::FnArg::Typed(arg) = input else {
            return Err(syn::Error::new(input.span(), "self not supported"));
        };

        let att = arg.attrs.iter().find(|att| att.path().is_ident("wgsl_gpu"));

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

        arg.attrs
            .retain(|att| att.path().is_ident("wgsl_gpu").not());
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
                "step mode must be provided for every or no arguments",
            ));
        }
        let const_name = quote::format_ident!("{}_VERTEX_BUFFER_LAYOUTS", ident_upper);
        let types = argument_types.iter();
        let step_modes = step_modes.iter();

        quote! {
            #[cfg(not(target_arch = "spirv"))]
            pub const #const_name: &[wgpu::VertexBufferLayout] = &[#(
                wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<#types>() as u64,
                    step_mode: wgpu::VertexStepMode::#step_modes,
                    attributes: #types::ATTRIBUTES,
                },
            )*];
        }
    };

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

/*
binding types
pub enum BindingType {
    Texture {
        sample_type: TextureSampleType,
        view_dimension: TextureViewDimension,
        multisampled: bool,
    },
    StorageTexture {
        access: StorageTextureAccess,
        format: TextureFormat,
        view_dimension: TextureViewDimension,
    },
    AccelerationStructure {
        vertex_return: bool,
    },
    ExternalTexture,
}
*/
