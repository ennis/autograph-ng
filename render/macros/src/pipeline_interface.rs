use super::autograph_name;
use darling::{util::Flag, FromDeriveInput, FromField};
use proc_macro2::TokenStream;
use quote::quote;
use syn::spanned::Spanned;

// Q: which attributes to expose?
// arrays should rarely be used => prefer assigning a meaningful name to each binding
// vertex_buffer, vertex_buffer_array

#[derive(FromDeriveInput, Debug)]
#[darling(attributes(pipeline), forward_attrs(allow, doc, cfg, repr))]
struct PipelineInterfaceStruct {
    ident: syn::Ident,
    generics: syn::Generics,
    vis: syn::Visibility,
    attrs: Vec<syn::Attribute>,
    //vertex_shader: String,
    //fragment_shader: String,
    //topology: String,
}

#[derive(FromField)]
#[darling(attributes(pipeline))]
struct PipelineInterfaceItem {
    #[darling(default)]
    inherit: Flag,
    #[darling(default)]
    framebuffer: Flag,
    #[darling(default)]
    descriptor_set: Flag,
    #[darling(default)]
    descriptor_set_array: Flag,
    #[darling(default)]
    viewport: Flag,
    #[darling(default)]
    viewport_array: Flag,
    #[darling(default)]
    scissor: Flag,
    #[darling(default)]
    scissor_array: Flag,
    #[darling(default)]
    vertex_buffer: Flag,
    #[darling(default)]
    vertex_buffer_array: Flag,
    #[darling(default)]
    index_buffer: Flag,
}

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
    let mut vtx_iface = Vec::new();
    let mut desc_set_iter = Vec::new();
    let mut desc_set_iface = Vec::new();
    let mut viewport_iter = Vec::new();
    let mut scissor_iter = Vec::new();
    let mut seen_ibuf = false;

    for f in fields.iter() {
        let ty = &f.ty;
        let name = &f.ident.as_ref().unwrap();

        match <PipelineInterfaceItem as FromField>::from_field(f) {
            Ok(pitem) => {
                // check for duplicates
                let mut num_attrs = 0;
                if pitem.inherit.is_some() {
                    num_attrs += 1;
                }
                if pitem.framebuffer.is_some() {
                    num_attrs += 1;
                }
                if pitem.descriptor_set.is_some() {
                    num_attrs += 1;
                }
                if pitem.descriptor_set_array.is_some() {
                    num_attrs += 1;
                }
                if pitem.viewport.is_some() {
                    num_attrs += 1;
                }
                if pitem.viewport_array.is_some() {
                    num_attrs += 1;
                }
                if pitem.scissor.is_some() {
                    num_attrs += 1;
                }
                if pitem.scissor_array.is_some() {
                    num_attrs += 1;
                }
                if pitem.vertex_buffer.is_some() {
                    num_attrs += 1;
                }
                if pitem.vertex_buffer_array.is_some() {
                    num_attrs += 1;
                }
                if pitem.index_buffer.is_some() {
                    num_attrs += 1;
                }

                if num_attrs == 0 {
                    stmts.push(syn::Error::new(name.span(), "missing or incomplete `pipeline(...)` attribute. See the documentation of `PipelineInterface` for more information.")
                        .to_compile_error());
                    continue;
                } else if num_attrs > 1 {
                    stmts.push(
                        syn::Error::new(
                            name.span(),
                            "field has more than one `pipeline(...)` attribute.",
                        )
                        .to_compile_error(),
                    );
                    continue;
                }

                if pitem.inherit.is_some() {
                    stmts.push(quote! { self.#name.do_visit(arena, visitor); });
                } else if pitem.framebuffer.is_some() {
                    stmts.push(quote! { visitor.visit_framebuffer(self.#name.clone()); });
                } else if pitem.descriptor_set.is_some() {
                    desc_set_iter.push(quote! { std::iter::once(self.#name.clone().into_descriptor_set(arena).into()) });
                    desc_set_iface
                        .push(quote! { <#ty as #gfx::descriptor::DescriptorSetInterface>::LAYOUT });
                } else if pitem.vertex_buffer.is_some() {
                    vbuf_iter.push(quote! { std::iter::once(self.#name.clone().into()) });
                    vtx_iface.push(quote! {
                        <<#ty as #gfx::vertex::VertexBufferInterface>::Vertex as #gfx::vertex::VertexData>::LAYOUT
                    });
                }
                if pitem.index_buffer.is_some() {
                    if !seen_ibuf {
                        // need indextype + offset
                        stmts.push(quote! {
                            visitor.visit_index_buffer(self.#name.clone().into());
                        });
                        seen_ibuf = true;
                    } else {
                        stmts.push(
                            syn::Error::new(
                                name.span(),
                                "duplicate `pipeline(index_buffer)` attribute",
                            )
                            .to_compile_error(),
                        );
                    }
                } else if pitem.viewport.is_some() {
                    viewport_iter.push(quote! {
                        std::iter::once(self.#name)
                    });
                } else if pitem.scissor.is_some() {
                    scissor_iter.push(quote! {
                        std::iter::once(self.#name)
                    });
                }
                // pipeline item arrays --------------------------------------------------------------------
                else if pitem.descriptor_set_array.is_some() {
                    desc_set_iter.push(
                        quote! {self.#name.into_iter().map(|a| a.into_descriptor_set().into())},
                    );
                } else if pitem.vertex_buffer_array.is_some() {
                    vbuf_iter.push(quote! {self.#name.into_iter().map(|a| a.into())});
                } else if pitem.viewport_array.is_some() {
                    viewport_iter.push(quote! {self.#name.into_iter()});
                } else if pitem.scissor_array.is_some() {
                    scissor_iter.push(quote! {self.#name.into_iter()});
                }
            }
            Err(e) => {
                stmts.push(
                    syn::Error::new(
                        name.span(),
                        format!("failed to parse `pipeline(...)` attribute: {}", e),
                    )
                    .to_compile_error(),
                );
            }
        }
    }

    let q = quote! {
        impl #impl_generics #gfx::pipeline::PipelineInterface<'a> for #struct_name #ty_generics #where_clause {
            const LAYOUT: &'static #gfx::pipeline::PipelineLayout<'static> = &#gfx::pipeline::PipelineLayout {
                descriptor_set_layouts: &[#(#desc_set_iface,)*],
                vertex_layouts: &[#(#vtx_iface,)*],
                fragment_outputs: &[]
            };

            fn do_visit<V: #gfx::pipeline::PipelineInterfaceVisitor<'a>>(&self, arena: &'a #gfx::Arena, visitor: &mut V) {
                use #gfx::descriptor::DescriptorSetInterface;
                #(#stmts)*

                visitor.visit_descriptor_sets(
                    std::iter::empty()#(.chain(#desc_set_iter))*
                );
                visitor.visit_vertex_buffers(
                    std::iter::empty()#(.chain(#vbuf_iter))*
                );
                visitor.visit_viewports(
                    std::iter::empty()#(.chain(#viewport_iter))*
                );
                visitor.visit_scissors(
                    std::iter::empty()#(.chain(#scissor_iter))*
                );
            }
        }
    };

    q
}
