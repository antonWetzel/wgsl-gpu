use std::ops::Not;

use syn::{Ident, PatType, Type, spanned::Spanned};
use zyn::{Args, Diagnostic, Span, ext::AttrExt, format_ident};

#[derive(zyn::Attribute)]
pub struct EntryAttributes {
	#[zyn(default)]
	vertex: bool,

	#[zyn(default)]
	fragment: bool,
}

#[zyn::element]
pub fn entry_generation(item: syn::ItemFn) -> zyn::Output {
	let mut args = Args::new();
	for attr in item.attrs.iter().filter(|item| item.is("spirv")) {
		args = match attr.args() {
			Ok(value) => args.merge(&value),
			Err(err) => return Diagnostic::from(err).into(),
		}
	}
	let attr = match EntryAttributes::from_args(&args) {
		Ok(value) => value,
		Err(err) => return Diagnostic::from(err).into(),
	};

	let visibility = if attr.vertex {
		Ident::new("VERTEX", item.span())
	} else if attr.fragment {
		Ident::new("FRAGMENT", item.span())
	} else {
		return Diagnostic::from(syn::Error::new(
			item.span(),
			"Vertex or fragment attribute required",
		))
		.into();
	};

	let attributes = item.attrs.iter().filter(|item| item.is("spirv"));

	let args = item
		.sig
		.inputs
		.iter()
		.map(|arg| match arg {
			syn::FnArg::Receiver(arg) => Err(syn::Error::new(arg.span(), "self not supported")),
			syn::FnArg::Typed(arg) => Ok(arg.clone()),
		})
		.collect::<Result<Vec<_>, _>>();
	let args = match args {
		Ok(value) => value,
		Err(err) => return Diagnostic::from(err).into(),
	};

	let fields_attributes = match FieldAttributes::parse(&args) {
		Ok(value) => value,
		Err(diag) => return diag.into(),
	};

	let macro_args = fields_attributes
		.iter()
		.zip(args.iter())
		.map(|(attr, arg)| {
			let prefix = if attr.arguments {
				let macro_name = transform_macro_name(&arg.ty);
				zyn::zyn! { __expand #macro_name => }
			} else {
				zyn::zyn! { __keep => }
			};
			let attributes = arg.attrs.iter().filter(|attr| attr.is("wgsl_gpu").not());
			zyn::zyn! { {{ prefix }} ( @for (att in attributes) { {{ att }} } {{ arg.pat }}: {{ arg.ty }}), }
		});

	let ident = &item.sig.ident;
	let ident_upper = ident.to_string().to_uppercase();
	let ident_gpu = zyn::format_ident!("{}_gpu", ident);
	let ident_gpu_value = ident_gpu.to_string();
	let const_name = zyn::format_ident!("{}_NAME", ident_upper);

	let ret_macro = match &item.sig.output {
		syn::ReturnType::Default => panic!("return type required"),
		syn::ReturnType::Type(_, ty) => transform_macro_name(ty),
	};

	zyn::zyn! {
		@filtered_input(input = item)

		wgsl_gpu::create_wrapper_function!(
			(@for (att in attributes) { {{ att }} } pub fn #ident_gpu), #ident,
			#ret_macro,
			(@for (arg in macro_args ) { {{ arg }} }),
		);

		pub const #const_name: &str = #ident_gpu_value;

		@vertex_buffer_layout(name = &ident, name_upper = &ident_upper, fields_attributes = &fields_attributes, args = &args)

		@bind_groups(name = &ident, args = &args, visibility = &visibility)
	}
}

#[zyn::element]
fn filtered_input<'a>(input: &'a syn::ItemFn) -> zyn::Output {
	fn filter(att: &syn::Attribute) -> bool {
		att.is("spirv").not() && att.is("wgsl_gpu").not()
	}

	let mut input = (*input).clone();
	input.attrs.retain(filter);

	for input in input.sig.inputs.iter_mut() {
		let syn::FnArg::Typed(arg) = input else {
			continue;
		};
		arg.attrs.retain(filter);
	}

	zyn::zyn! { {{ input }} }
}

#[zyn::element]
fn vertex_buffer_layout<'a>(
	name: &'a syn::Ident,
	name_upper: &'a str,
	fields_attributes: &'a [FieldAttributes],
	args: &'a [syn::PatType],
) -> zyn::Output {
	let mut entries = Vec::new();
	for (attr, arg) in fields_attributes.iter().zip(args.iter()) {
		if attr.arguments.not() {
			continue;
		}
		let step_mode = match attr.step_mode.as_ref().map(|v| v.as_str()) {
			None if entries.is_empty().not() => {
				return Diagnostic::from(syn::Error::new(
					name.span(),
					"Step mode must be provided for every or no arguments",
				))
				.into();
			}
			None => continue,
			Some("vertex") => Ident::new("Vertex", Span::call_site()),
			Some("instance") => Ident::new("Instance", Span::call_site()),
			Some(_) => {
				return Diagnostic::from(syn::Error::new(name.span(), "Invalid step mode")).into();
			}
		};

		entries.push(zyn::zyn! {
			wgpu::VertexBufferLayout {
				array_stride: std::mem::size_of::<{{ arg.ty }}>() as u64,
				step_mode: wgpu::VertexStepMode::{{ step_mode }},
				attributes: {{ arg.ty }}::ATTRIBUTES,
			}
		});
	}

	let const_name = zyn::format_ident!("{}_VERTEX_BUFFER_LAYOUTS", name_upper);

	zyn::zyn! {
		#[cfg(not(target_arch = "spirv"))]
		pub const {{ const_name }}: &[wgpu::VertexBufferLayout] = &[@for (entry in entries) {
			{{ entry }},
		}];
	}
}

fn transform_macro_name(ty: &syn::Type) -> zyn::Output {
	let mut ty = ty.clone();
	let syn::Type::Path(path) = &mut ty else {
		return Diagnostic::from(syn::Error::new(ty.span(), "Return type must be a path")).into();
	};
	if let Some(last) = path.path.segments.last_mut() {
		last.ident = zyn::format_ident!("wgsl_gpu_{}_transform", &last.ident);
	}
	zyn::zyn! { {{ ty }} }
}

#[derive(zyn::Attribute)]
pub struct FieldAttributes {
	#[zyn(default)]
	arguments: bool,

	#[zyn(default)]
	step_mode: Option<String>,
}

impl FieldAttributes {
	pub fn parse(args: &[syn::PatType]) -> Result<Vec<Self>, Diagnostic> {
		args.iter()
			.map(|arg| {
				arg.attrs
					.iter()
					.find(|attr| attr.is("wgsl_gpu"))
					.map(|attr| {
						let arg = attr.parse_args::<zyn::Args>()?;
						Self::from_args(&arg)
					})
					.unwrap_or(Ok(FieldAttributes {
						arguments: false,
						step_mode: None,
					}))
			})
			.collect()
	}
}

#[derive(zyn::Attribute)]
pub struct BindGroupAttributes {
	#[zyn(default)]
	descriptor_set: u32,

	#[zyn(default)]
	binding: u32,

	#[zyn(default)]
	uniform: bool,
}

#[zyn::element]
fn bind_groups<'a>(
	name: &'a syn::Ident,
	args: &'a [PatType],
	visibility: &'a syn::Ident,
) -> zyn::Output {
	let mut sets = <[Vec<(usize, zyn::Output)>; 8]>::default();

	args.iter()
		.filter_map(|arg| {
			arg.attrs
				.iter()
				.find(|attr| attr.is("spirv"))
				.map(|attr| {
					let arg = attr.parse_args::<zyn::Args>().ok()?;
					BindGroupAttributes::from_args(&arg).ok()
				})
				.flatten()
				.map(|attr| (attr, arg))
		})
		.for_each(|(attr, ty)| {
			let binding_type = bind_group_wgpu_type(&attr, &ty.ty);
			sets[attr.descriptor_set as usize].push((attr.binding as usize, binding_type));
		});

	let total = sets.iter().map(|value| value.len()).max().unwrap_or(0);

	let args = zyn::zyn! {
		(
			{{ total }},
			[@for (set in sets.iter()) { {{ set.len() }}, }],
			[@for (set in sets.iter()) {
				{
					@for (edit in set.iter()) {
						$entry[{{ edit.0 }}].binding = {{ edit.0 as u32 }};
						$entry[{{ edit.0 }}].visibility =
							$entry[{{ edit.0 }}].visibility.union(wgpu::ShaderStages::{{ visibility }});
						$entry[{{ edit.0 }}].ty = {{ edit.1 }};
					}
				},
			}]
		)
	};

	zyn::zyn! {
		#[macro_export]
		macro_rules! {{ format_ident!("wgsl_gpu_{}_bind_groups_macro", name) }} {
			($target:path, $context:tt, $entry:ident) => {
				$target!(
					$context,
					{{ args }}
				);
			};
		}
	}
}

fn bind_group_wgpu_type(attr: &BindGroupAttributes, ty: &Type) -> zyn::Output {
	if attr.uniform {
		return zyn::zyn! {
			wgpu::BindingType::Buffer {
				ty: wgpu::BufferBindingType::Uniform,
				has_dynamic_offset: false,
				min_binding_size: None,
			}
		};
	}

	// todo: storage texture
	match ty {
		Type::Reference(ty) => bind_group_wgpu_type(attr, &ty.elem),
		Type::Path(ty) => match ty.path.segments.last() {
			// todo: check spirv or wgsl_gpu attributes for sampler type
			Some(last) if last.ident == "Sampler" => zyn::zyn! {
				wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering)
			},
			_ => Diagnostic::from(syn::Error::new(
				ty.span(),
				"Could not identify binding type",
			))
			.into(),
		},
		Type::Macro(_ty) => {
			// todo: check if image macro
			// todo: parse the macro body for parameters
			zyn::zyn! {
				wgpu::BindingType::Texture {
					sample_type: wgpu::TextureSampleType::Float { filterable: true },
					view_dimension: wgpu::TextureViewDimension::D2,
					multisampled: false,
				}
			}
		}
		_ => Diagnostic::from(syn::Error::new(
			ty.span(),
			"Could not identify binding type",
		))
		.into(),
	}
}
