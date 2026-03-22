use std::ops::Not;

use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::DeriveInput;

pub fn arguments(input: DeriveInput) -> TokenStream {
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

    let attributes = data
        .fields
        .iter()
        .map(|field| find_attribute(field.attrs.iter().rev()));

    let tuple_types = arg_types.clone();
    let tuple_fields = fields.clone();
    let tuple_arg = fields.clone();

    let locations = generate_attributes.then(|| {
        let attrs = data.fields.iter().map(|field| {
            let location = field_location(field);
            let att = rust_type_to_wgpu_attribute(&field.ty);
            quote! { #location => #att }
        });

        quote! {
            #[cfg(not(target_arch = "spirv"))]
            impl #name {
                pub const ATTRIBUTES: &[wgpu::VertexAttribute] = wgpu::vertex_attr_array![
                    #( #attrs, )*
                ]
                    .as_slice();
            }
        }
    });

    let ret_attributes = data
        .fields
        .iter()
        .map(|field| find_attribute(field.attrs.iter()));
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

fn find_attribute<'a>(attrs: impl Iterator<Item = &'a syn::Attribute>) -> &'a TokenStream {
    attrs
        .filter(|attribute| attribute.path().is_ident("wgsl_gpu"))
        .find_map(|attribute| match &attribute.meta {
            syn::Meta::List(list) => Some(&list.tokens),
            _ => None,
        })
        .expect("every member requires a wgsl_gpu attribute")
}

fn field_location(field: &syn::Field) -> syn::Expr {
    let tokens = find_attribute(field.attrs.iter());

    let meta = syn::parse2::<syn::Meta>(tokens.clone()).unwrap();

    if meta.path().is_ident("location").not() {
        panic!("attributes require locations");
    }

    match meta {
        syn::Meta::NameValue(named) => named.value,
        _ => panic!("location must be \"location = value\""),
    }
}

fn rust_type_to_wgpu_attribute(ty: &syn::Type) -> syn::Ident {
    match ty {
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
                "i32" => quote::format_ident!("Sint32"),
                "IVec2" => quote::format_ident!("Sint32x2"),
                "IVec3" => quote::format_ident!("Sint32x3"),
                "IVec4" => quote::format_ident!("Sint32x4"),
                _ => panic!("unsupported type for vertex attribute: {}", name),
            }
        }

        syn::Type::Array(array) => {
            // Extract the element type as a string (assuming it's a simple path)
            let elem_ty = match &*array.elem {
                syn::Type::Path(elem_path) => {
                    elem_path.path.segments.last().unwrap().ident.to_string()
                }
                _ => panic!("unsupported element type in array (must be a simple path)"),
            };

            // Get the dimension as a literal (e.g., 3 from [f32; 3])
            let dim = match &array.len {
                syn::Expr::Lit(lit) => {
                    if let syn::Lit::Int(int_lit) = &lit.lit {
                        int_lit.base10_parse::<usize>().unwrap()
                    } else {
                        panic!("array length must be an integer literal");
                    }
                }
                _ => panic!("array length must be a literal"),
            };

            // Map (element type, dimension) to the wgpu format
            match (elem_ty.as_str(), dim) {
                ("f32", 2) => quote::format_ident!("Float32x2"),
                ("f32", 3) => quote::format_ident!("Float32x3"),
                ("f32", 4) => quote::format_ident!("Float32x4"),
                ("u32", 2) => quote::format_ident!("Uint32x2"),
                ("u32", 3) => quote::format_ident!("Uint32x3"),
                ("u32", 4) => quote::format_ident!("Uint32x4"),
                ("i32", 2) => quote::format_ident!("Sint32x2"),
                ("i32", 3) => quote::format_ident!("Sint32x3"),
                ("i32", 4) => quote::format_ident!("Sint32x4"),
                _ => panic!("unsupported array type for vertex attribute: [{elem_ty}; {dim}]"),
            }
        }

        _ => panic!("invalid type for attributes (expected path or array)"),
    }
}
