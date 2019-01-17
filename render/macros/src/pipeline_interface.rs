use super::autograph_name;
use darling::{FromDeriveInput, FromField};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{parse_str, AngleBracketedGenericArguments, Ident};
use syn::spanned::Spanned;

// Q: which attributes to expose?
// arrays should rarely be used => prefer assigning a meaningful name to each binding
// vertex_buffer, vertex_buffer_array

#[derive(FromDeriveInput, Debug)]
#[darling(attributes(interface), forward_attrs(allow, doc, cfg, repr))]
struct PipelineInterfaceStruct {
    ident: syn::Ident,
    generics: syn::Generics,
    vis: syn::Visibility,
    attrs: Vec<syn::Attribute>,
    #[darling(default)]
    arguments: Option<String>,
}


#[derive(FromField)]
#[darling(attributes(framebuffer))]
struct Framebuffer {}

#[derive(FromField)]
#[darling(attributes(descriptor_set))]
struct DescriptorSet {
    // TODO: honor index attribute
    index: u32,
}

#[derive(FromField)]
#[darling(attributes(descriptor_set_array))]
struct DescriptorSetArray {
    base_index: u32,
}

#[derive(FromField)]
#[darling(attributes(viewport))]
struct Viewport {}

#[derive(FromField)]
#[darling(attributes(viewport_array))]
struct ViewportArray {}

#[derive(FromField)]
#[darling(attributes(scissor))]
struct Scissor {}

#[derive(FromField)]
#[darling(attributes(scissor_array))]
struct ScissorArray {}

#[derive(FromField)]
#[darling(attributes(vertex_buffer))]
struct VertexBuffer {
    // TODO: honor index attribute
    index: u32
}

#[derive(FromField)]
#[darling(attributes(vertex_buffer_array))]
struct VertexBufferArray {
    // TODO: honor base index attribute
    base_index: u32
}

#[derive(FromField)]
#[darling(attributes(index_buffer))]
struct IndexBuffer {}

pub fn generate(ast: &syn::DeriveInput, fields: &syn::Fields) -> TokenStream {
    let s = <PipelineInterfaceStruct as FromDeriveInput>::from_derive_input(ast).unwrap();

    let gfx = autograph_name();
    let struct_name = &s.ident;
    let (impl_generics, ty_generics, where_clause) = s.generics.split_for_impl();

    //----------------------------------------------------------------------------------------------
    let fields = match fields {
        syn::Fields::Named(ref fields_named) => &fields_named.named,
        syn::Fields::Unnamed(ref fields_unnamed) => &fields_unnamed.unnamed,
        syn::Fields::Unit => {
            panic!("PipelineInterfaceStruct trait cannot be derived on unit structs")
        }
    };


    let mut stmts = Vec::new();
    // for vertex buffers, descriptor sets, viewport, and scissors, build an iterator by chaining
    // individual items in the struct.
    // Another option would be to copy all items into a temporary array and pass a slice
    // to this array, but it needs dynamic allocation with structs that contain item arrays
    // where the number of items is not known in advance.
    //
    // However, when it's possible, the preferred way to create pipeline interfaces is to pass
    // each pipeline item into its own struct field, with a meaningful name,
    // and do not use item arrays.
    //
    // Chaining individual items is not a very efficient approach: we rely on the optimizer
    // to clean this for us. This *will* generate abysmal code in debug mode, though.
    let mut vbuf_iter = Vec::new();
    let mut desc_set_iter = Vec::new();
    let mut viewport_iter = Vec::new();
    let mut scissor_iter = Vec::new();
    let mut seen_ibuf = false;

    for f in fields.iter() {
        let ty = &f.ty;
        let name = &f.ident.as_ref().unwrap();

        if let Ok(_) = <Framebuffer as FromField>::from_field(f) {
            stmts.push(quote!{
                visitor.visit_framebuffer(#name.into());
            });
        }
        else if let Ok(_) = <DescriptorSet as FromField>::from_field(f) {
            desc_set_iter.push(quote!{
                std::iter::once(#name)
            });
        }
        else if let Ok(_) = <VertexBuffer as FromField>::from_field(f) {
            vbuf_iter.push(quote!{
                std::iter::once(#name)
            });
        }
        else if let Ok(_) = <IndexBuffer as FromField>::from_field(f) {
            if !seen_ibuf {
                // need indextype + offset
                stmts.push(quote! {
                    visitor.visit_index_buffer(#name.into());
                });
                seen_ibuf = true;
            }
            else {
                return syn::Error::new(f.span(), "duplicate 'index_buffer' attribute")
                    .to_compile_error();
            }
        }
        else if let Ok(_) = <Viewport as FromField>::from_field(f) {
            viewport_iter.push(quote!{
                std::iter::once(#name)
            });
        }
        else if let Ok(_) = <Scissor as FromField>::from_field(f) {
            scissor_iter.push(quote! {
                std::iter::once(#name)
            });
        }
        // pipeline item arrays --------------------------------------------------------------------
        else if let Ok(_) = <DescriptorSetArray as FromField>::from_field(f) {
            desc_set_iter.push(quote!{
                #name.into_iter()
            });
        }
        else if let Ok(_) = <VertexBufferArray as FromField>::from_field(f) {
            vbuf_iter.push(quote!{
                #name.into_iter()
            });
        }
        else if let Ok(_) = <ViewportArray as FromField>::from_field(f) {
            viewport_iter.push(quote!{
                #name.into_iter()
            });
        }
        else if let Ok(_) = <ScissorArray as FromField>::from_field(f) {
            scissor_iter.push(quote!{
                #name.into_iter()
            });
        }
        else {
            // emit warning
            //f.span().unstable().
            //quote!()
        }
    }

    let q = quote!{
        impl<'a, R: RendererBackend> #gfx::interface::PipelineInterface<'a, R> for #struct_name {
            const VERTEX_INPUT_INTERFACE: &'static [VertexInputBufferDescription<'static>] = &[];
            const FRAGMENT_OUTPUT_INTERFACE: &'static [FragmentOutputDescription] = &[];
            const DESCRIPTOR_SET_INTERFACE: &'static [&'static [DescriptorSetLayoutBinding<'static>]] = &[];

            fn do_visit<V: PipelineInterfaceVisitor<'a,R>>(&self, visitor: &mut V) {
                #(#stmts)*

                visitor.visit_descriptor_sets(
                    std::iter::empty()#(.chain(#desc_set_iter))*
                );
                visitor.visit_vertex_buffers(
                    std::iter::empty()#(.chain(#vbuf_iter))*
                );
                visitor.visit_viewport(
                    std::iter::empty()#(.chain(#viewport_iter))*
                );
                visitor.visit_scissor(
                    std::iter::empty()#(.chain(#scissor_iter))*
                );
            }
        }
    };

    unimplemented!()
}