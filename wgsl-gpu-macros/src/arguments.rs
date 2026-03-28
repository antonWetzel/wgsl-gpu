use std::ops::Not;

use zyn::{
    Diagnostic, Span,
    ext::{AttrExt, TypeExt},
    format_ident,
};

#[derive(zyn::Attribute)]
#[zyn("wgsl_gpu")]
pub struct ArgumentsAttributes {
    #[zyn(default)]
    attributes: bool,
}

#[derive(zyn::Attribute)]
pub struct ArgumentsFieldAttributes {
    #[zyn(default)]
    location: Option<u32>,

    #[zyn(default)]
    output: Option<String>,

    #[zyn(default)]
    input: Option<String>,
}

impl ArgumentsFieldAttributes {
    pub fn parse(fields: &syn::FieldsNamed) -> Result<Vec<Self>, Diagnostic> {
        fields
            .named
            .iter()
            .map(|field| {
                field
                    .attrs
                    .iter()
                    .find(|attr| attr.is("wgsl_gpu"))
                    .map(|attr| {
                        let arg = attr.parse_args::<zyn::Args>()?;
                        return ArgumentsFieldAttributes::from_args(&arg);
                    })
                    .unwrap_or_else(|| {
                        Err(syn::Error::new(field.span(), "missing wgsl_gpu attribute").into())
                    })
            })
            .collect()
    }

    fn output_attribute(&self) -> zyn::Output {
        if let Some(location) = self.location {
            zyn::zyn! { location = {{ location }} }
        } else if let Some(output) = &self.output {
            zyn::zyn! { {{ zyn::format_ident!("{}", output) }} }
        } else {
            Diagnostic::from(syn::Error::new(
                Span::call_site(),
                "Expected location or output attribute",
            ))
            .into()
        }
    }
    fn input_attribute(&self) -> zyn::Output {
        if let Some(location) = self.location {
            zyn::zyn! { location = {{ location }} }
        } else if let Some(input) = &self.input {
            zyn::zyn! { {{ zyn::format_ident!("{}", input) }} }
        } else {
            Diagnostic::from(syn::Error::new(
                Span::call_site(),
                "Expected location or input attribute",
            ))
            .into()
        }
    }
}

#[zyn::element]
pub fn arguments_locations<'a>(
    name: &'a zyn::syn::Ident,
    attributes: &'a ArgumentsAttributes,
    fields: &'a syn::FieldsNamed,
    fields_attributes: &'a [ArgumentsFieldAttributes],
) -> zyn::TokenStream {
    if attributes.attributes.not() {
        return zyn::Output::default();
    }

    zyn::zyn! {
        #[cfg(not(target_arch = "spirv"))]
        impl {{ name }} {
            pub const ATTRIBUTES: &[wgpu::VertexAttribute] = &[
                @for (arg in fields.named.iter().zip(fields_attributes.iter())) {
                    @arguments_locations_field(name = name, field = arg.0, attributes = arg.1)
                }
            ];
        }
    }
}

#[zyn::element]
fn arguments_locations_field<'a>(
    name: &'a syn::Ident,
    field: &'a syn::Field,
    attributes: &'a ArgumentsFieldAttributes,
) -> zyn::Output {
    let Some(location) = attributes.location else {
        return Diagnostic::from(syn::Error::new_spanned(
            field,
            "location attribute required",
        ))
        .into();
    };

    let wgpu_attribute = rust_type_to_wgpu_attribute(&field.ty);
    zyn::zyn! {
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::{{ wgpu_attribute }},
            offset: std::mem::offset_of!({{ name }}, {{ field.ident }}) as u64,
            shader_location: {{ location }},
        },
    }
}

#[zyn::element]
pub fn arguments_trait_impl<'a>(
    name: &'a zyn::syn::Ident,
    fields: &'a syn::FieldsNamed,
) -> zyn::TokenStream {
    zyn::zyn! {
        impl wgsl_gpu::Arguments for {{ name }} {
            type Arguments = (@for (field in fields.named.iter()) {
                {{ field.ty }},
            });

            fn from_arguments(arguments: Self::Arguments) -> Self {
                Self {
                    @for (arg in fields.named.iter().enumerate()) {
                        {{ arg.1.ident }}: arguments.{{ zyn::syn::Index::from(arg.0) }},
                    }
                }
            }
        }
    }
}

#[zyn::element]
pub fn arguments_data_macro<'a>(
    name: &'a syn::Ident,
    fields: &'a syn::FieldsNamed,
    fields_attributes: &'a [ArgumentsFieldAttributes],
) -> zyn::Output {
    zyn::zyn! {
        #[spirv_std::macros::spirv_recursive_for_testing]
        #[macro_export]
        #[doc(hidden)]
        macro_rules! {{ format_ident!("wgsl_gpu_{}_transform", name) }} {
            // get arguments and parameters
            (__arg, $target:path, $context:tt) => {
                $target!(
                    $context,
                    (@for (arg in fields.named.iter().zip(fields_attributes.iter())) {
                        #[spirv({{arg.1.input_attribute()}})] {{ arg.0.ident }}: {{ arg.0.ty }},
                    }),
                    (wgsl_gpu::Arguments::from_arguments((@for (field in fields.named.iter()) {
                        {{ field.ident }},
                    })),),
                );
            };
            // get arguments for return values
            (__ret, $target:path, $context:tt) => {
                $target!(
                    $context,
                    (@for (arg in fields.named.iter().zip(fields_attributes.iter())) {
                        #[spirv({{arg.1.output_attribute()}})] {{ arg.0.ident }}: &mut {{ arg.0.ty }},
                    }),
                    output,
                    (@for (field in fields.named.iter()) {
                        *{{ field.ident }} = output.{{ field.ident }};
                    }),
                );
            };
        }
    }
}

// todo: replace panic with compile errors
fn rust_type_to_wgpu_attribute(ty: &syn::Type) -> syn::Ident {
    match ty {
        syn::Type::Path(path) => {
            let name = path.path.segments.last().unwrap().ident.to_string();
            match name.as_str() {
                "f32" => zyn::format_ident!("Float32"),
                "Vec2" => zyn::format_ident!("Float32x2"),
                "Vec3" => zyn::format_ident!("Float32x3"),
                "Vec4" => zyn::format_ident!("Float32x4"),
                "u32" => zyn::format_ident!("Uint32"),
                "UVec2" => zyn::format_ident!("Uint32x2"),
                "UVec3" => zyn::format_ident!("Uint32x3"),
                "UVec4" => zyn::format_ident!("Uint32x4"),
                "i32" => zyn::format_ident!("Sint32"),
                "IVec2" => zyn::format_ident!("Sint32x2"),
                "IVec3" => zyn::format_ident!("Sint32x3"),
                "IVec4" => zyn::format_ident!("Sint32x4"),
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
                ("f32", 2) => zyn::format_ident!("Float32x2"),
                ("f32", 3) => zyn::format_ident!("Float32x3"),
                ("f32", 4) => zyn::format_ident!("Float32x4"),
                ("u32", 2) => zyn::format_ident!("Uint32x2"),
                ("u32", 3) => zyn::format_ident!("Uint32x3"),
                ("u32", 4) => zyn::format_ident!("Uint32x4"),
                ("i32", 2) => zyn::format_ident!("Sint32x2"),
                ("i32", 3) => zyn::format_ident!("Sint32x3"),
                ("i32", 4) => zyn::format_ident!("Sint32x4"),
                _ => panic!("unsupported array type for vertex attribute: [{elem_ty}; {dim}]"),
            }
        }

        _ => panic!("invalid type for attributes (expected path or array)"),
    }
}
