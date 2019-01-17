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

mod layout;
mod descriptor_set_interface;
mod pipeline_interface;

fn autograph_name() -> syn::Path {
    syn::parse_str("autograph_render").unwrap()
}

#[proc_macro_derive(BufferLayout)]
pub fn buffer_layout_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).expect("Couldn't parse item");

    let result = match ast.data {
        syn::Data::Struct(ref s) => layout::generate_buffer_layout(&ast, &s.fields),
        _ => panic!("BufferLayout trait can only be automatically derived on structs."),
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

#[proc_macro_derive(DescriptorSetInterface, attributes(interface, descriptor))]
pub fn descriptor_set_interface_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).expect("Couldn't parse item");

    let result = match ast.data {
        syn::Data::Struct(ref s) => descriptor_set_interface::generate(&ast, &s.fields),
        _ => panic!("DescriptorSetInterface trait can only be derived on structs"),
    };

    result.into()
}

/// Derives an implementation of [PipelineInterface] for a given struct.
///
/// By default, the implementation is generic over the backend type, and expects the struct to have
/// one lifetime parameter (`'a`) and a single type parameter for the renderer backend
/// (`T: RendererBackend`).
///
/// Attributes define the interpretation of struct fields in terms of pipeline bindings.
/// The field must be convertible via [Into] to the target binding type:
/// * `descriptor_set(index=n)` specifies a descriptor set.
/// The field must implement `Into<DescriptorSet>`.
/// * `vertex_buffer` specifies a vertex buffer. The field must implement `Into<BufferTypeless>`.
/// * `index_buffer` specifies an index buffer. The field must implement `Into<BufferTypeless>`.
///   Annotating more than one field with this attribute is an error.
/// * `viewport` specifies a viewport. The field must implement `Into<Viewport>`.
/// * `scissor` specifies a scissor rectangle. The field must implement `Into<ScissorRect>`.
///
/// It is also possible to specify arrays of pipeline bindings:
/// * `descriptor_set_array(base_index=n)` specifies an array of descriptor sets.
///   The field must implement `IntoIterator<DescriptorSet>`.
/// * `vertex_buffer_array(base_index=n)` specifies an array of vertex buffers.
///   The field must implement `IntoIterator<BufferTypeless>`.
/// * `viewport_array(base_index=n)` specifies an array of viewports.
///   The field must implement `IntoIterator<Viewport>`.
/// * `scissor_array(base_index=n)` specifies an array of scissor rectangles.
///   The field must implement `IntoIterator<ScissorRect>`.
///
/// When a binding index is required, it is derived from the order of appearance of the binding in
/// the struct. For instance:
///
/// ```
/// #[derive(PipelineInterface)]
/// pub struct ExampleInterface<'a, R: RendererBackend> {
///     #[descriptor_set] ds_a: DescriptorSet<'a, R>,     // index 0
///     ...
///     #[descriptor_set] ds_b: DescriptorSet<'a, R>,     // index 1
///     ...
///     #[descriptor_set_array] dss: DescriptorSet<'a, R>,   // indices 2..2+dss.len
///     ...
///     #[descriptor_set] ds_c: DescriptorSet<'a, R>,   // index dss.len
/// }
/// ```
///
/// #### Example
///
///```
/// #[derive(PipelineInterface)]
/// pub struct ExampleInterface<'a, R: Render> {
///    #[framebuffer]
///    pub framebuffer: Framebuffer<'a>,
///    #[descriptor_set]
///    pub per_object: DescriptorSet<'a>,   // DS index 0
///    #[viewport]
///    pub viewport: Viewport,
///    // Buffer<T> implements Into<BufferTypeless>
///    #[vertex_buffer]
///    pub vertex_buffer: Buffer<'a, [Vertex]>,  // VB index 0
///    #[descriptor_set]
///    pub other: DescriptorSet<'a>,  // DS index 1
/// }
/// ```
///
#[proc_macro_derive(PipelineInterface, attributes(
    interface,
    framebuffer,
    descriptor_set,
    descriptor_set_array,
    viewport,
    viewport_array,
    scissor,
    scissor_array,
    vertex_buffer,
    vertex_buffer_array,
    index_buffer))]
pub fn pipeline_interface_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).expect("Couldn't parse item");

    let result = match ast.data {
        syn::Data::Struct(ref s) => pipeline_interface::generate(&ast, &s.fields),
        _ => panic!("PipelineInterface trait can only be derived on structs"),
    };

    result.into()
}
