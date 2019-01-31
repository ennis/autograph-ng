use super::autograph_name;
use darling::{util::Flag, FromDeriveInput, FromField};
use proc_macro2::TokenStream;
use quote::quote;
use syn::spanned::Spanned;
use proc_macro2::Span;

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
    render_target: Flag,
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
    // Shader interfaces -----------------------
    #[darling(default)]
    uniform_buffer: Flag,
    #[darling(default)]
    storage_buffer: Flag,
    #[darling(default)]
    sampled_image: Flag,
    #[darling(default)]
    storage_image: Flag,
}

fn quote_descriptor(gfx: &syn::Path, index: usize, ty: &syn::Type, descty: &str) -> TokenStream {
    let descty = Some(syn::Ident::new(descty, Span::call_site()));
    quote! {
        #gfx::descriptor::DescriptorBinding {
            binding: #index,
            descriptor_type: #gfx::descriptor::DescriptorType::#descty,
            stage_flags: #gfx::pipeline::ShaderStageFlags::ALL_GRAPHICS,
            count: 1,
            tydesc: <#ty as #gfx::descriptor::DescriptorInterface>::TYPE,
        }
    }
}

pub fn generate(ast: &syn::DeriveInput, fields: &syn::Fields) -> TokenStream {
    let s : PipelineInterfaceStruct = <PipelineInterfaceStruct as FromDeriveInput>::from_derive_input(ast).unwrap();

    let gfx = autograph_name();
    let struct_name = &s.ident;


    //----------------------------------------------------------------------------------------------
    // adjust generics:
    // if ty_generics has only one lifetime => no host data
    // otherwise => first lifetime is
    let (impl_generics, ty_generics, where_clause) = s.generics.split_for_impl();
    let first_lt = s.generics.lifetimes().next();

    let lt_arena = if let Some(lt) = first_lt { lt } else {
        return
            syn::Error::new(
                s.generics.span(),
                "expected exactly one lifetime on target of `#[derive(PipelineInterface)]`",
            )
                .to_compile_error();
    };

    //----------------------------------------------------------------------------------------------
    let fields = match fields {
        syn::Fields::Named(ref fields_named) => &fields_named.named,
        syn::Fields::Unnamed(ref fields_unnamed) => &fields_unnamed.unnamed,
        syn::Fields::Unit => {
            panic!("PipelineInterface trait cannot be derived on unit structs")
        }
    };


    let mut stmts = Vec::new();
    let mut i_subsig = Vec::new();
    let mut i_fragout = Vec::new();
    let mut i_vtxin = Vec::new();
    let mut i_desc = Vec::new();
    let mut seen_ibuf = false;
    let mut n_viewports = 0usize;
    let mut n_scissors = 0usize;

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
                if pitem.render_target.is_some() {
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
                if pitem.uniform_buffer.is_some() {
                    num_attrs += 1;
                }
                if pitem.storage_buffer.is_some() {
                    num_attrs += 1;
                }
                if pitem.sampled_image.is_some() {
                    num_attrs += 1;
                }
                if pitem.storage_image.is_some() {
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
                    // arguments --------------------------------------------
                    let index = i_subsig.len();
                    stmts.push(quote! { a.set_arguments(#index, self.#name.clone().into()); });
                    i_subsig.push(quote! { <#ty as #gfx::pipeline::PipelineInterface>::SIGNATURE });
                } else if pitem.render_target.is_some() {
                    // render target --------------------------------------------
                    let index = i_fragout.len();
                    stmts.push(quote! { a.set_render_target(#index, self.#name.clone().into()); });
                    i_fragout.push(quote! { #gfx::framebuffer::FragmentOutputDescription{} });
                } else if pitem.uniform_buffer.is_some() {
                    // descriptor: ubo --------------------------------------------
                    let index = i_desc.len();
                    stmts.push(quote! { a.set_descriptor(#index, self.#name.clone().into()); });
                    i_desc.push(quote_descriptor(&gfx, index, ty, "UniformBuffer"));
                } else if pitem.storage_buffer.is_some() {
                    // descriptor: ssbo --------------------------------------------
                    let index = i_desc.len();
                    stmts.push(quote! { a.set_descriptor(#index, self.#name.clone().into()); });
                    i_desc.push(quote_descriptor(&gfx, index, ty, "StorageBuffer"));
                } else if pitem.sampled_image.is_some() {
                    // descriptor: tex --------------------------------------------
                    let index = i_desc.len();
                    stmts.push(quote! { a.set_descriptor(#index, self.#name.clone().into()); });
                    i_desc.push(quote_descriptor(&gfx, index, ty, "SampledImage"));
                } else if pitem.storage_image.is_some() {
                    // descriptor: img --------------------------------------------
                    let index = i_desc.len();
                    stmts.push(quote! { a.set_descriptor(#index, self.#name.clone().into()); });
                    i_desc.push(quote_descriptor(&gfx, index, ty, "StorageImage"));
                }
                else if pitem.vertex_buffer.is_some() {
                    // vertex buffer --------------------------------------------
                    let index = i_vtxin.len();
                    stmts.push(quote! { a.set_vertex_buffer(#index, self.#name.clone().into()) });
                    i_vtxin.push(quote! {
                        <<#ty as #gfx::vertex::VertexBufferInterface>::Vertex as #gfx::vertex::VertexData>::LAYOUT
                    });
                }
                else if pitem.index_buffer.is_some() {
                    // index buffer --------------------------------------------
                    if !seen_ibuf {
                        // need indextype + offset
                        stmts.push(quote! {
                            a.set_index_buffer(Some(self.#name.clone().into()));
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
                    // viewport --------------------------------------------
                    stmts.push(quote! { a.set_viewport(#n_viewports, self.#name.clone().into()) });
                    n_viewports += 1;
                } else if pitem.scissor.is_some() {
                    // scissor --------------------------------------------
                    stmts.push(quote! { a.set_scissor(#n_scissors, self.#name.clone().into()) });
                    n_scissors += 1;
                } else if pitem.viewport_array.is_some() {
                    unimplemented!()
                } else if pitem.scissor_array.is_some() {
                    unimplemented!()
                } else if pitem.vertex_buffer_array.is_some() {
                    unimplemented!()
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

    let privmod = syn::Ident::new(
        &format!("__PipelineInterface_UniqueType_{}", struct_name),
        Span::call_site(),
    );

    let ty_params = s.generics.type_params();
    let ty_params2 = s.generics.type_params();

    let q = quote! {

        #[doc(hidden)]
        mod #privmod {
            // IMPORTANT must be generic if interface struct is generic
            // but the generic parameters must also be 'static...
            pub struct Dummy<#(#ty_params,)*>;
        }

        impl #impl_generics #gfx::pipeline::PipelineInterface<#lt_arena> for #struct_name #ty_generics #where_clause {

            type UniqueType = #privmod::Dummy<#(#ty_params2,)*>;
            type IntoInterface = Self;

            const SIGNATURE: &'static #gfx::pipeline::Signature<'static> = &#gfx::pipeline::Signature {
                sub_signatures:    &[#(#i_subsig,)*],
                descriptors:       &[#(#i_desc,)*],
                vertex_layouts:    &[#(#i_vtxin,)*],
                fragment_outputs:  &[#(#i_fragout,)*],
                typeid:            Some(std::any::TypeId::of::<Self::UniqueType>()),
            };

            fn update_pipeline_arguments(
                    &self,
                    arena: &'a #gfx::Arena,
                    a: &dyn #gfx::pipeline::PipelineArguments<#lt_arena>)
            {
                use #gfx::pipeline::PipelineInterface;
                #(#stmts)*
            }
        }
    };

    q
}
