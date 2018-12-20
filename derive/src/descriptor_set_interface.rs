use super::gfx2_name;
use darling::{util::Flag, FromDeriveInput, FromField};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{parse_str, AngleBracketedGenericArguments, Ident};

/*#[derive(Default, FromMeta)]
#[darling(default)]
pub struct BackendType {

}*/

#[derive(FromDeriveInput, Debug)]
#[darling(attributes(interface), forward_attrs(allow, doc, cfg, repr))]
struct DescriptorSetInterfaceStruct {
    ident: syn::Ident,
    generics: syn::Generics,
    vis: syn::Visibility,
    attrs: Vec<syn::Attribute>,
    #[darling(default)]
    arguments: Option<String>,
}

#[derive(FromField)]
#[darling(attributes(descriptor))]
struct Descriptor {
    //ident: Option<syn::Ident>,
    //ty: syn::Type,
    //vis: syn::Visibility,
    #[darling(default)]
    index: Option<u32>,
    #[darling(default)]
    uniform_buffer: Flag,
    #[darling(default)]
    storage_buffer: Flag,
    #[darling(default)]
    sampled_image: Flag,
    #[darling(default)]
    storage_image: Flag,
}

/*
fn make_option_tokens<T: quote::ToTokens>(v: &Option<T>) -> TokenStream {
    if let Some(v) = v.as_ref() {
        quote!(Some(#v))
    } else {
        quote!(None)
    }
}*/

pub fn generate(ast: &syn::DeriveInput, fields: &syn::Fields) -> TokenStream {
    let s = <DescriptorSetInterfaceStruct as FromDeriveInput>::from_derive_input(ast).unwrap();

    let gfx = gfx2_name();

    let struct_name = &s.ident;
    let (impl_generics, ty_generics, where_clause) = s.generics.split_for_impl();

    //----------------------------------------------------------------------------------------------
    let fields = match fields {
        syn::Fields::Named(ref fields_named) => &fields_named.named,
        syn::Fields::Unnamed(ref fields_unnamed) => &fields_unnamed.unnamed,
        syn::Fields::Unit => {
            panic!("DescriptorSetInterface trait cannot be derived on unit structs")
        }
    };

    let mut bindings = Vec::new();
    let mut binding_indices = Vec::new();
    let mut index = 0;

    for f in fields.iter() {
        let field_ty = &f.ty;
        //let field_ty_without_lifetimes = field_ty.uses_type_params()
        //let field_name = f.ident.clone().unwrap();
        let descriptor = <Descriptor as FromField>::from_field(f).unwrap();

        //if let Ok(descriptor) = descriptor {
        if let Some(i) = descriptor.index {
            index = i;
        }

        let mut descriptor_type = None;

        // UNIFORM BUFFER ----------------------------
        if descriptor.uniform_buffer.is_some() {
            descriptor_type = Some(Ident::new("UniformBuffer", Span::call_site()));
        }
        // STORAGE BUFFER ----------------------------
        if descriptor.storage_buffer.is_some() {
            if descriptor_type.is_some() {
                panic!("expected only one of `storage_image`, `sampled_image`, `uniform_buffer`, `storage_buffer`");
            }
            descriptor_type = Some(Ident::new("StorageBuffer", Span::call_site()));
        }
        // SAMPLED IMAGE ----------------------------
        if descriptor.sampled_image.is_some() {
            if descriptor_type.is_some() {
                panic!("expected only one of `storage_image`, `sampled_image`, `uniform_buffer`, `storage_buffer`");
            }
            descriptor_type = Some(Ident::new("SampledImage", Span::call_site()));
        }
        // STORAGE IMAGE ----------------------------
        if descriptor.storage_image.is_some() {
            if descriptor_type.is_some() {
                panic!("expected only one of `storage_image`, `sampled_image`, `uniform_buffer`, `storage_buffer`");
            }
            descriptor_type = Some(Ident::new("StorageImage", Span::call_site()));
        }

        let descriptor_type = descriptor_type.expect(
            "expected one of `storage_image`, `sampled_image`, `uniform_buffer`, `storage_buffer`",
        );

        bindings.push(quote! {
            #gfx::DescriptorSetLayoutBinding {
                binding: #index,
                descriptor_type: #gfx::DescriptorType::#descriptor_type,
                stage_flags: #gfx::ShaderStageFlags::ALL_GRAPHICS,
                count: 1,
                tydesc: <#field_ty as #gfx::DescriptorInterface<_>>::TYPE,
            }
        });

        binding_indices.push(index);
        index += 1;
        /*} else {
            // TODO more info
            panic!("invalid descriptor set entry");
        }*/
    }

    let field_names = fields.iter().map(|f| f.ident.as_ref().unwrap());

    let do_visit_calls = field_names
        .zip(binding_indices.iter())
        .map(|(field_name, binding_index)| {
            quote! {
                #gfx::DescriptorInterface::do_visit(&self.#field_name, #binding_index, visitor);
            }
        })
        .collect::<Vec<_>>();

    //----------------------------------------------------------------------------------------------
    let q = if let Some(ref args) = s.arguments {
        let args: AngleBracketedGenericArguments =
            parse_str(args).expect("failed to parse angle bracketed generic arguments");
        quote! {
            impl #impl_generics #gfx::DescriptorSetInterface #args for #struct_name #ty_generics #where_clause {
                const INTERFACE: &'static [#gfx::DescriptorSetLayoutBinding<'static>] = &[#(#bindings,)*];
                fn do_visit(&self, visitor: &mut impl #gfx::DescriptorSetInterfaceVisitor#args) {
                    #(#do_visit_calls)*
                }
            }
        }
    } else {
        quote! {
            impl #impl_generics #gfx::DescriptorSetInterface #ty_generics for #struct_name #ty_generics #where_clause {
                const INTERFACE: &'static [#gfx::DescriptorSetLayoutBinding<'static>] = &[#(#bindings,)*];
                fn do_visit(&self, visitor: &mut impl #gfx::DescriptorSetInterfaceVisitor#ty_generics) {
                    #(#do_visit_calls)*
                }
            }
        }
    };

    //println!("{:?}", q.to_string());

    q

    /*let mut uniform_constants = Vec::new();
    let mut texture_bindings = Vec::new();
    let mut vertex_buffers = Vec::new();
    let mut render_targets = Vec::new();
    let mut uniform_buffers = Vec::new();
    let mut index_buffer = None;

    let fields =
        match *fields {
            syn::Fields::Named(ref fields) => { fields },
            _ => { panic!("PipelineInterface trait cannot be derived on unit structs or tuple structs.") }
        };

    for f in fields.named.iter() {
        let field_name = f.ident.clone().unwrap();
        let mut seen_interface_attr = false;
        for a in f.attrs.iter() {
            let meta = a.interpret_meta();
            let meta = if let Some(meta) = meta {
                meta
            } else {
                continue;
            };

            match meta.name().to_string().as_ref() {
                "uniform_constant" => {
                    if seen_interface_attr {
                        error_multiple_interface_attrs();
                    }
                    let uniform_constant =
                        <UniformConstant as FromField>::from_field(f).unwrap();
                    uniform_constants.push(uniform_constant);
                    seen_interface_attr = true;
                }
                "texture_binding" => {
                    if seen_interface_attr {
                        error_multiple_interface_attrs();
                    }
                    let texture_binding =
                        <TextureBinding as FromField>::from_field(f).unwrap();
                    texture_bindings.push(texture_binding);
                    seen_interface_attr = true;
                }
                "vertex_buffer" => {
                    if seen_interface_attr {
                        error_multiple_interface_attrs();
                    }
                    let vb = <VertexBuffer as FromField>::from_field(f).unwrap();
                    vertex_buffers.push(vb);
                    seen_interface_attr = true;
                }
                "uniform_buffer" => {
                    if seen_interface_attr {
                        error_multiple_interface_attrs();
                    }
                    let ub = <UniformBuffer as FromField>::from_field(f).unwrap();
                    uniform_buffers.push(ub);
                    seen_interface_attr = true;
                }
                "index_buffer" => {
                    if seen_interface_attr {
                        error_multiple_interface_attrs();
                    }
                    if index_buffer.is_some() {
                        panic!("Only one index buffer can be specified.")
                    }
                    let ib = <IndexBuffer as FromField>::from_field(f).unwrap();
                    index_buffer = Some(ib);
                    seen_interface_attr = true;
                }
                "render_target" => {
                    if seen_interface_attr {
                        error_multiple_interface_attrs();
                    }
                    let rt = <RenderTarget as FromField>::from_field(f).unwrap();
                    render_targets.push(rt);
                    seen_interface_attr = true;
                }
                _ => {}
            }
        }
    }

    unimplemented!()*/
}

/*//
    // named uniforms
    //
    let uniform_constant_items = uniform_constants
        .iter()
        .map(|u| {
            let name = u
                .rename
                .as_ref()
                .map_or(u.ident.clone().unwrap(), |s| syn::Ident::new(s.as_str(), Span::call_site()));
            let index_tokens = make_option_tokens(&u.index);
            let ty = &u.ty;

            //let index_tokens = make_option_tokens(texbind.index);
            quote! {
                UniformConstantDesc {
                    name: Some(stringify!(#name).into()),
                    index: #index_tokens,
                    ty: <#ty as UniformInterface>::get_description()
                }
            }
        })
        .collect::<Vec<_>>();
    let num_uniform_constant_items = uniform_constant_items.len();

    //
    // texture+sampler bindings
    //
    let mut texture_binding_items = Vec::new();
    let mut texture_bind_statements = Vec::new();
    for texbind in texture_bindings.iter() {
        let orig_name = texbind.ident.clone().unwrap();
        let name = texbind
            .rename
            .as_ref()
            .map_or(texbind.ident.clone().unwrap(), |s| syn::Ident::new(s.as_str(), Span::call_site()));
        let index_tokens = make_option_tokens(&texbind.index);
        let ty = &texbind.ty;

        texture_binding_items.push(quote! {
            ::autograph::gfx::shader_interface::TextureBindingDesc {
                name: Some(stringify!(#name).into()),
                index: #index_tokens,
                data_type: <<#ty as SampledTextureInterface>::TextureType as TextureInterface>::get_data_type(),
                dimensions: <<#ty as SampledTextureInterface>::TextureType as TextureInterface>::get_dimensions()
            }
        });

        texture_bind_statements.push(quote! {
             {
                let tex = interface.#orig_name.get_texture().into_texture_any();
                let sampler = interface.#orig_name.get_sampler();
                let sampler_obj = frame.queue().context().get_sampler(sampler);
                state_cache.set_texture((#index_tokens).unwrap(), &tex, &sampler_obj);
                frame.ref_texture(tex);
            }
        });
    }
    let num_texture_binding_items = texture_binding_items.len();

    //
    // vertex buffers
    //
    let vertex_buffer_items = vertex_buffers
        .iter()
        .map(|vb| {
            let name = vb
                .rename
                .as_ref()
                .map_or(vb.ident.clone().unwrap(), |s| syn::Ident::new(s.as_str(), Span::call_site()));
            let index_tokens = make_option_tokens(&vb.index);
            let ty = &vb.ty;

            quote! {
                ::autograph::gfx::shader_interface::VertexBufferDesc {
                    name: Some(stringify!(#name).into()),
                    index: #index_tokens,
                    layout: <<#ty as ::autograph::gfx::VertexDataSource>::ElementType as ::autograph::gfx::VertexType>::get_layout()
                }
            }
        })
        .collect::<Vec<_>>();
    let num_vertex_buffer_items = vertex_buffer_items.len();

    //
    // uniform buffers
    //
    let mut uniform_buffer_items = Vec::new();
    let mut uniform_buffer_bind_statements = Vec::new();
    for ub in uniform_buffers.iter() {
        let orig_name = ub.ident.clone().unwrap();
        let name = ub
            .rename
            .as_ref()
            .map_or(ub.ident.clone().unwrap(), |s| syn::Ident::new(s.as_str(), Span::call_site()));
        let index_tokens = make_option_tokens(&ub.index);
        let ty = &ub.ty;

        uniform_buffer_items.push(quote! {
            ::autograph::gfx::shader_interface::UniformBufferDesc {
                name: Some(stringify!(#name).into()),
                index: #index_tokens,
                tydesc: <#ty as ::autograph::gfx::BufferInterface>::get_layout()
            }
        });
        uniform_buffer_bind_statements.push(quote! {
            {
                let slice_any = interface.#orig_name.to_slice_any();
                state_cache.set_uniform_buffer((#index_tokens).unwrap(), &slice_any);
                frame.ref_buffer(slice_any.owner);
            }
        });
    }

    let num_uniform_buffer_items = uniform_buffer_items.len();

    //
    // render targets
    //
    let render_target_items = render_targets
        .iter()
        .map(|rt| {
            let name = rt
                .rename
                .as_ref()
                .map_or(rt.ident.clone().unwrap(), |s| syn::Ident::new(s.as_str(), Span::call_site()));
            let index_tokens = make_option_tokens(&rt.index);
            let ty = &rt.ty;
            quote! {
                ::autograph::gfx::shader_interface::RenderTargetDesc {
                    name: Some(stringify!(#name).into()),
                    index: #index_tokens,
                    format: None
                }
            }
        })
        .collect::<Vec<_>>();
    let num_render_target_items = render_target_items.len();

    let index_buffer_item = if let Some(ib) = index_buffer {
        let ty = &ib.ty;
        quote! {
            Some(IndexBufferDesc {
                format: <<#ty as ::autograph::gfx::IndexDataSource>::ElementType as ::autograph::gfx::IndexElementType>::FORMAT
            })
        }
    } else {
        quote!(None)
    };

    let private_module_name = syn::Ident::new(
        &format!("__shader_interface_{}", struct_name),
        proc_macro2::Span::call_site(),
    );

    // generate impls
    quote!{
        #[allow(non_snake_case)]
        mod #private_module_name {
            use super::#struct_name;
            use ::autograph::gfx::shader_interface::*;

            pub(super) struct Desc;
            pub(super) struct Binder;

            lazy_static!{
                static ref UNIFORM_CONSTANTS: [UniformConstantDesc;#num_uniform_constant_items] = [#(#uniform_constant_items),*];
                static ref TEXTURE_BINDINGS: [TextureBindingDesc;#num_texture_binding_items] = [#(#texture_binding_items),*];
                static ref VERTEX_BUFFERS: [VertexBufferDesc;#num_vertex_buffer_items] = [#(#vertex_buffer_items),*];
                static ref UNIFORM_BUFFERS: [UniformBufferDesc;#num_uniform_buffer_items] = [#(#uniform_buffer_items),*];
                static ref RENDER_TARGETS: [RenderTargetDesc;#num_render_target_items] = [#(#render_target_items),*];
                static ref INDEX_BUFFER: Option<IndexBufferDesc> = #index_buffer_item;
            }

            impl ShaderInterfaceDesc for Desc {
                fn get_uniform_constants(&self) -> &'static [UniformConstantDesc] {
                    &*UNIFORM_CONSTANTS
                }
                fn get_render_targets(&self) -> &'static [RenderTargetDesc] {
                     &*RENDER_TARGETS
                }
                fn get_vertex_buffers(&self) -> &'static [VertexBufferDesc] {
                    &*VERTEX_BUFFERS
                }
                fn get_index_buffer(&self) -> Option<&'static IndexBufferDesc> {
                    INDEX_BUFFER.as_ref()
                }
                fn get_texture_bindings(&self) -> &'static [TextureBindingDesc] {
                    &*TEXTURE_BINDINGS
                }
                fn get_uniform_buffers(&self) -> &'static [UniformBufferDesc] {
                    &*UNIFORM_BUFFERS
                }
                //fn get_framebuffer(&self) ->
            }

            impl InterfaceBinder<#struct_name> for Binder {
                unsafe fn bind_unchecked(&self, interface: &#struct_name, frame: &::autograph::gfx::Frame, state_cache: &mut ::autograph::gfx::StateCache) {
                    use ::autograph::gfx::ToBufferSliceAny;
                    use ::autograph::gfx::SampledTextureInterface;
                    unsafe {
                        #(#uniform_buffer_bind_statements)*
                    }
                }
            }

        }

        impl ::autograph::gfx::ShaderInterface for #struct_name {
            fn get_description() -> &'static ::autograph::gfx::ShaderInterfaceDesc {
                static INSTANCE: &'static ::autograph::gfx::ShaderInterfaceDesc = &#private_module_name::Desc;
                INSTANCE
            }

            fn create_interface_binder(pipeline: &::autograph::gfx::GraphicsPipeline) -> Result<Box<::autograph::gfx::InterfaceBinder<Self>>, ::failure::Error> where Self: Sized {
                // TODO: verify interface
                Ok(Box::new(#private_module_name::Binder))
            }
        }
    }
}*/
