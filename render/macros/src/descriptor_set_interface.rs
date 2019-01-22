use super::autograph_name;
use darling::{util::Flag, FromDeriveInput, FromField};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::Ident;

#[derive(FromDeriveInput, Debug)]
#[darling(forward_attrs(allow, doc, cfg, repr))]
struct DescriptorSetInterfaceStruct {
    ident: syn::Ident,
    generics: syn::Generics,
    vis: syn::Visibility,
    attrs: Vec<syn::Attribute>,
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

pub fn generate(ast: &syn::DeriveInput, fields: &syn::Fields) -> TokenStream {
    let s = <DescriptorSetInterfaceStruct as FromDeriveInput>::from_derive_input(ast).unwrap();

    let gfx = autograph_name();

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
    // see pipeline_interface::generate for an explanation of how item lists are built.
    let mut desc_iter = Vec::new();

    for f in fields.iter() {
        let name = f.ident.as_ref().unwrap();
        let field_ty = &f.ty;
        //let field_ty_without_lifetimes = field_ty.uses_type_params()
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

        desc_iter.push(quote! {
            // Into<Descriptor>
            std::iter::once(self.#name.into())
        });

        bindings.push(quote! {
            #gfx::descriptor::DescriptorSetLayoutBinding {
                binding: #index,
                descriptor_type: #gfx::descriptor::DescriptorType::#descriptor_type,
                stage_flags: #gfx::pipeline::ShaderStageFlags::ALL_GRAPHICS,
                count: 1,
                tydesc: <#field_ty as #gfx::descriptor::DescriptorInterface>::TYPE,
            }
        });

        binding_indices.push(index);
        index += 1;
    }

    let privmod = syn::Ident::new(
        &format!("__DescriptorSetInterface_UniqueTypeFor_{}", struct_name),
        Span::call_site(),
    );

    //----------------------------------------------------------------------------------------------
    let q = quote! {
        #[doc(hidden)]
        mod #privmod {
            pub struct Dummy;
        }
        impl #impl_generics #gfx::descriptor::DescriptorSetInterface<'a>
            for #struct_name #ty_generics #where_clause {
            const LAYOUT: #gfx::descriptor::DescriptorSetLayout<'static> =
                #gfx::descriptor::DescriptorSetLayout {
                    bindings: &[#(#bindings,)*],
                    typeid: Some(std::any::TypeId::of::<#privmod::Dummy>())
                };
            type UniqueType = #privmod::Dummy;
            type IntoInterface = Self;
            fn into_descriptor_set(self, arena: &'a #gfx::Arena) -> #gfx::descriptor::DescriptorSet<'a, Self> {
                arena.create_descriptor_set(std::iter::empty()#(.chain(#desc_iter))*)
            }
        }
    };

    q
}
