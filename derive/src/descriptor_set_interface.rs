use super::gfx2_name;
use darling::{util::Flag, FromDeriveInput, FromField};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{parse_str, AngleBracketedGenericArguments, Ident};

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
                tydesc: <#field_ty as #gfx::interface::DescriptorInterface<_>>::TYPE,
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
                #gfx::interface::DescriptorInterface::do_visit(&self.#field_name, #binding_index, visitor);
            }
        })
        .collect::<Vec<_>>();

    //----------------------------------------------------------------------------------------------
    let q = if let Some(ref args) = s.arguments {
        let args: AngleBracketedGenericArguments =
            parse_str(args).expect("failed to parse angle bracketed generic arguments");
        quote! {
            impl #impl_generics #gfx::interface::DescriptorSetInterface #args for #struct_name #ty_generics #where_clause {
                const INTERFACE: &'static [#gfx::DescriptorSetLayoutBinding<'static>] = &[#(#bindings,)*];
                fn do_visit(&self, visitor: &mut impl #gfx::interface::DescriptorSetInterfaceVisitor#args) {
                    #(#do_visit_calls)*
                }
            }
        }
    } else {
        quote! {
            impl #impl_generics #gfx::interface::DescriptorSetInterface #ty_generics for #struct_name #ty_generics #where_clause {
                const INTERFACE: &'static [#gfx::DescriptorSetLayoutBinding<'static>] = &[#(#bindings,)*];
                fn do_visit(&self, visitor: &mut impl #gfx::interface::DescriptorSetInterfaceVisitor#ty_generics) {
                    #(#do_visit_calls)*
                }
            }
        }
    };

    q
}
