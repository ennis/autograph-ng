//! Proc-macro for auto-deriving pipeline interfaces:
//! - `PipelineInterface`
//! - `BufferLayout` for verifying the layout of uniform buffer data with SPIR-V
//! - `AttachmentGroup` for groups of attachments
//! - `VertexLayout` for verifying the layout of vertex buffers
//!
#![recursion_limit = "128"]

extern crate darling; // this is a _good crate_
extern crate proc_macro;
extern crate proc_macro2;
extern crate quote;
extern crate syn;

mod buffer_layout;
mod descriptor_set_interface;

fn gfx2_name() -> syn::Path {
    syn::parse_str("gfx2").unwrap()
}

#[proc_macro_derive(BufferLayout)]
pub fn buffer_layout_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).expect("Couldn't parse item");

    let result = match ast.data {
        syn::Data::Struct(ref s) => buffer_layout::generate(&ast, &s.fields),
        _ => panic!("BufferLayout trait can only be automatically derived on structs."),
    };

    result.into()
}

#[proc_macro_derive(DescriptorSetInterface, attributes(interface, descriptor))]
pub fn descriptor_set_interface_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).expect("Couldn't parse item");

    let result = match ast.data {
        syn::Data::Struct(ref s) => descriptor_set_interface::generate(&ast, &s.fields),
        _ => panic!("DescriptorSetInterface trait can only be derived on structs"),
    };

    result.into()
}

#[cfg(test)]
mod test {
    #[test]
    fn compiles() {}
}
