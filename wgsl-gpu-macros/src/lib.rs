mod arguments;
mod entry;

extern crate proc_macro;

use crate::arguments::{ArgumentsAttributes, ArgumentsFieldAttributes};

#[zyn::attribute]
fn entry(#[zyn(input)] item: syn::ItemFn, _args: zyn::Args) -> zyn::TokenStream {
	zyn::zyn! {
		@entry::entry_generation(item = item)
	}
}

#[zyn::derive("Arguments", attributes(wgsl_gpu))]
pub fn arguments(
	#[zyn(input)] attributes: ArgumentsAttributes,
	#[zyn(input)] ident: zyn::Extract<syn::Ident>,
	#[zyn(input)] fields: zyn::Fields<syn::FieldsNamed>,
) -> zyn::Output {
	let name = ident.inner();
	let fields = fields.inner();

	let fields_attributes = match ArgumentsFieldAttributes::parse(&fields) {
		Ok(value) => value,
		Err(diag) => return diag.into(),
	};

	zyn::zyn! {
		@arguments::arguments_locations(name = &name, attributes = &attributes, fields = &fields, fields_attributes = &fields_attributes)
		@arguments::arguments_trait_impl(name = &name, fields = &fields)
		@arguments::arguments_data_macro(name = &name, fields = &fields, fields_attributes = &fields_attributes)
	}
}
