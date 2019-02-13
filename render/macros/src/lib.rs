//! Proc-macro for auto-deriving pipeline interfaces:
//! - `PipelineInterface`
//! - `BufferLayout` for verifying the layout of uniform buffer data with SPIR-V
//! - `AttachmentGroup` for groups of attachments
//! - `VertexLayout` for verifying the layout of vertex buffers
//!
#![recursion_limit = "256"]

extern crate darling; // this is a _good crate_
extern crate proc_macro;
extern crate proc_macro2;
extern crate quote;
extern crate syn;

//mod descriptor_set_interface;
mod layout;
mod arguments;

fn autograph_name() -> syn::Path {
    syn::parse_str("autograph_render").unwrap()
}

#[proc_macro_derive(StructuredBufferData)]
pub fn structured_buffer_data_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).expect("Couldn't parse item");

    let result = match ast.data {
        syn::Data::Struct(ref s) => layout::generate_structured_buffer_data(&ast, &s.fields),
        _ => panic!("StructuredBufferData trait can only be automatically derived on structs."),
    };

    result.into()
}

#[proc_macro_derive(VertexData)]
pub fn vertex_data_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).expect("Couldn't parse item");

    let result = match ast.data {
        syn::Data::Struct(ref s) => layout::generate_vertex_data(&ast, &s.fields),
        _ => panic!("BufferLayout trait can only be automatically derived on structs."),
    };

    result.into()
}

#[proc_macro_derive(Arguments, attributes(argument))]
pub fn arguments_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).expect("Couldn't parse item");

    let result = match ast.data {
        syn::Data::Struct(ref s) => arguments::generate(&ast, &s.fields),
        _ => panic!("PipelineInterface trait can only be derived on structs"),
    };

    result.into()
}
