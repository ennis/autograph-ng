use crate::G;
use autograph_spirv as spirv;
use autograph_spirv::{
    ast::Variable,
    layout::{Layout, LayoutDetails},
    ArrayLayout, FieldsLayout, ImageType, TypeDesc,
};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use shaderc::ShaderKind;

/*
fn gen_spirv_option_image_format(_fmt: Option<spirv::headers::ImageFormat>) -> TokenStream {
    quote!(None)
}*/

fn gen_format_from_spirv(fmt: spirv::headers::ImageFormat) -> syn::Ident {
    let s = match fmt {
        spirv::headers::ImageFormat::Unknown => "UNDEFINED",
        spirv::headers::ImageFormat::Rgba32f => "R32G32B32A32_SFLOAT",
        spirv::headers::ImageFormat::Rgba16f => "R16G16B16A16_SFLOAT",
        spirv::headers::ImageFormat::R32f => "R32_SFLOAT",
        spirv::headers::ImageFormat::Rgba8 => "R8G8B8A8_UNORM",
        spirv::headers::ImageFormat::Rgba8Snorm => "RGBA8_SNORM",
        spirv::headers::ImageFormat::Rg32f => "R32G32_SFLOAT",
        spirv::headers::ImageFormat::Rg16f => "R16G16_SFLOAT",
        spirv::headers::ImageFormat::R11fG11fB10f => {
            unimplemented!("{:?}", fmt) /*"R11F_G11F_B10F"*/
        }
        spirv::headers::ImageFormat::R16f => "R16_SFLOAT",
        spirv::headers::ImageFormat::Rgba16 => "R16G16B16A16_UNORM",
        spirv::headers::ImageFormat::Rgb10A2 => {
            unimplemented!("{:?}", fmt) /*"RGB10_A2"*/
        }
        spirv::headers::ImageFormat::Rg16 => "R16G16_UNORM",
        spirv::headers::ImageFormat::Rg8 => "R8G8_UNORM",
        spirv::headers::ImageFormat::R16 => "R16_UNORM",
        spirv::headers::ImageFormat::R8 => "R8_UNORM",
        spirv::headers::ImageFormat::Rgba16Snorm => "R16G16B16A16_SNORM",
        spirv::headers::ImageFormat::Rg16Snorm => "R16G16_SNORM",
        spirv::headers::ImageFormat::Rg8Snorm => "R8G8_SNORM",
        spirv::headers::ImageFormat::R16Snorm => "R16_SNORM",
        spirv::headers::ImageFormat::R8Snorm => "R8_SNORM",
        spirv::headers::ImageFormat::Rgba32i => "R32G32B32A32_SINT",
        spirv::headers::ImageFormat::Rgba16i => "R16G16B16A16_SINT",
        spirv::headers::ImageFormat::Rgba8i => "R8G8B8A8_SINT",
        spirv::headers::ImageFormat::R32i => "R32_SINT",
        spirv::headers::ImageFormat::Rg32i => "R32G32_SINT",
        spirv::headers::ImageFormat::Rg16i => "R16G16_SINT",
        spirv::headers::ImageFormat::Rg8i => "R8G8_SINT",
        spirv::headers::ImageFormat::R16i => "R16_SINT",
        spirv::headers::ImageFormat::R8i => "R8_SINT",
        spirv::headers::ImageFormat::Rgba32ui => "R32G32B32A32_UINT",
        spirv::headers::ImageFormat::Rgba16ui => "R16G16B16A16_UINT",
        spirv::headers::ImageFormat::Rgba8ui => "R8G8B8A8_UINT",
        spirv::headers::ImageFormat::R32ui => "R32_UINT",
        spirv::headers::ImageFormat::Rgb10a2ui => {
            unimplemented!("{:?}", fmt) /*"rgb10a2ui"*/
        }
        spirv::headers::ImageFormat::Rg32ui => "R32G32_UINT",
        spirv::headers::ImageFormat::Rg16ui => "R16G16_UINT",
        spirv::headers::ImageFormat::Rg8ui => "R8G8_UINT",
        spirv::headers::ImageFormat::R16ui => "R16G16_UINT",
        spirv::headers::ImageFormat::R8ui => "R8_UINT",
    };
    syn::Ident::new(s, Span::call_site())
}

fn gen_primitive_type(ty: spirv::PrimitiveType) -> TokenStream {
    match ty {
        spirv::PrimitiveType::Int => quote!(#G::typedesc::PrimitiveType::Int),
        spirv::PrimitiveType::UnsignedInt => quote!(#G::typedesc::PrimitiveType::UnsignedInt),
        spirv::PrimitiveType::Half => quote!(#G::typedesc::PrimitiveType::Half),
        spirv::PrimitiveType::Float => quote!(#G::typedesc::PrimitiveType::Float),
        spirv::PrimitiveType::Double => quote!(#G::typedesc::PrimitiveType::Double),
        spirv::PrimitiveType::Bool => quote!(#G::typedesc::PrimitiveType::Bool),
    }
}

fn gen_image_type_info(img: &spirv::ImageType) -> TokenStream {
    let sampled_ty = gen_type_info(img.sampled_ty);
    let format = syn::Ident::new(&format!("{:?}", img.format), Span::call_site());
    let dimensions = syn::Ident::new(&format!("{:?}", img.dimensions), Span::call_site());
    quote! {
        #G::typedesc::ImageType {
            sampled_ty: #sampled_ty,
            format: #G::typedesc::ImageFormat::#format,
            dimensions: #G::typedesc::Dim::#dimensions,
        }
    }
}

fn gen_type_info(ty: &spirv::TypeDesc) -> TokenStream {
    match ty {
        TypeDesc::Primitive(ty) => {
            let prim = gen_primitive_type(*ty);
            quote!(#G::typedesc::TypeDesc::Primitive(#prim))
        }
        TypeDesc::Array { elem_ty, len } => {
            let elem_ty = gen_type_info(elem_ty);
            quote!(#G::typedesc::TypeDesc::Array { elem_ty: &#elem_ty, len: #len })
        }
        TypeDesc::Vector { elem_ty, len } => {
            let elem_ty = gen_primitive_type(*elem_ty);
            quote!(#G::typedesc::TypeDesc::Vector { elem_ty: #elem_ty, len: #len })
        }
        TypeDesc::Matrix {
            elem_ty,
            rows,
            columns,
        } => {
            let elem_ty = gen_primitive_type(*elem_ty);
            quote!(#G::typedesc::TypeDesc::Matrix { elem_ty: #elem_ty, rows: #rows, columns: #columns })
        }
        TypeDesc::Struct { fields } => {
            let fields = fields.iter().map(|&f| gen_type_info(f));
            quote!(
                #G::typedesc::TypeDesc::Struct { fields: &[#(&#fields,)*] }
            )
        }
        TypeDesc::Image(image_ty) => {
            let image_ty = gen_image_type_info(image_ty);
            quote! {
                #G::typedesc::TypeDesc::Image(#image_ty)
            }
        }
        TypeDesc::SampledImage(image_ty) => {
            let image_ty = gen_image_type_info(*image_ty);
            quote! {
                #G::typedesc::TypeDesc::SampledImage(&#image_ty)
            }
        }
        TypeDesc::Void => unimplemented!("{:?}", ty),
        TypeDesc::Pointer(_ty) => unimplemented!("{:?}", ty),
        TypeDesc::Unknown => unimplemented!("{:?}", ty),
    }
}

fn gen_layout_info(layout: &Layout) -> TokenStream {
    let align = layout.align;
    let size = layout.size;

    let details = match layout.details {
        LayoutDetails::None => quote!(#G::typedesc::LayoutDetails::None),
        LayoutDetails::Array(ArrayLayout {
            elem_layout,
            stride,
        }) => {
            let elem_layout = gen_layout_info(elem_layout);
            quote!(#G::typedesc::LayoutDetails::Array(#G::typedesc::ArrayLayout {
                elem_layout: &#elem_layout,
                stride: #stride
            }))
        }
        LayoutDetails::Struct(FieldsLayout { offsets, layouts }) => {
            let field_layouts = layouts.iter().map(|l| gen_layout_info(l));
            quote!(#G::typedesc::LayoutDetails::Struct(#G::typedesc::FieldsLayout {
                offsets: &[#(#offsets,)*],
                layouts: &[#(&#field_layouts,)*],
            }))
        }
    };

    quote!(#G::typedesc::Layout {
        align: #align,
        size: #size,
        details: #details
    })
}

fn gen_resource_shape(dim: spirv::headers::Dim) -> TokenStream {
    match dim {
        spirv::headers::Dim::Dim1D => quote!(#G::descriptor::ResourceShape::R1d),
        spirv::headers::Dim::Dim2D => quote!(#G::descriptor::ResourceShape::R2d),
        spirv::headers::Dim::Dim3D => quote!(#G::descriptor::ResourceShape::R3d),
        spirv::headers::Dim::DimCube => quote!(#G::descriptor::ResourceShape::RCube),
        spirv::headers::Dim::DimRect => unimplemented!("rectangle textures are unsupported"),
        spirv::headers::Dim::DimBuffer => unimplemented!("buffer textures are unsupported"),
        spirv::headers::Dim::DimSubpassData => unimplemented!("subpass inputs are unsupported"),
    }
}

fn gen_stage_flags(sk: ShaderKind) -> TokenStream {
    match sk {
        ShaderKind::Vertex => quote!(#G::pipeline::ShaderStageFlags::VERTEX),
        ShaderKind::Fragment => quote!(#G::pipeline::ShaderStageFlags::FRAGMENT),
        ShaderKind::Compute => quote!(#G::pipeline::ShaderStageFlags::COMPUTE),
        ShaderKind::Geometry => quote!(#G::pipeline::ShaderStageFlags::GEOMETRY),
        ShaderKind::TessControl => quote!(#G::pipeline::ShaderStageFlags::TESS_CONTROL),
        ShaderKind::TessEvaluation => quote!(#G::pipeline::ShaderStageFlags::TESS_EVAL),
        _ => unimplemented!("{:?}", sk),
    }
}

fn gen_descriptor_reflection_info(
    s: &Span,
    v: &spirv::ast::Variable,
    stage: ShaderKind,
    set: u32,
    binding: u32,
) -> TokenStream {
    let has_buffer_block_deco = v.has_buffer_block_decoration().is_some();
    let stage_flags = gen_stage_flags(stage);
    let a = spirv::DroplessArena::new();

    if v.storage == spirv::headers::StorageClass::Uniform
    /*&& has_block_deco*/
    {
        // uniform buffer (constant buffer) --------------------------------------------------------
        let ty = v.ty.pointee_type().expect("expected pointer type");
        let tyinfo = gen_type_info(ty);
        // FIXME may be something other than std140
        let tylayout = spirv::layout::Layout::std140(&a, ty);
        let tylayoutinfo = gen_layout_info(tylayout);
        quote! {
            #G::descriptor::ResourceBinding {
                set: Some(#set),
                index: #binding,
                ty: #G::descriptor::ResourceBindingType::ConstantBuffer,
                stage_flags: #stage_flags,
                count: 1,
                data_ty: Some(&#tyinfo),
                data_layout: Some(&#tylayoutinfo),
                data_format: #G::Format::UNDEFINED
            }
        }
    } else if (v.storage == spirv::headers::StorageClass::Uniform && has_buffer_block_deco)
        || (v.storage == spirv::headers::StorageClass::StorageBuffer)
    {
        // shader storage buffer (rwbuffer) --------------------------------------------------------
        let ty = v.ty.pointee_type().expect("expected pointer type");
        let tyinfo = gen_type_info(ty);
        // FIXME may be something other than std140
        let tylayout = spirv::layout::Layout::std140(&a, ty);
        let tylayoutinfo = gen_layout_info(tylayout);
        quote! {
            #G::descriptor::ResourceBinding {
                set: Some(#set),
                index: #binding,
                ty: #G::descriptor::ResourceBindingType::RwBuffer,
                stage_flags: #stage_flags,
                count: 1,
                data_ty: Some(&#tyinfo),
                data_layout: Some(&#tylayoutinfo),
                data_format: #G::Format::UNDEFINED
            }
        }
    } else if v.storage == spirv::headers::StorageClass::UniformConstant {
        if let &TypeDesc::Pointer(&TypeDesc::Image(ImageType {
            sampled_ty,
            format,
            dimensions,
        })) = v.ty
        {
            // image -------------------------------------------------------------------------------
            let sampled_ty = gen_type_info(sampled_ty);
            let dim = gen_resource_shape(dimensions);
            let format = gen_format_from_spirv(format);
            quote! {
                #G::descriptor::ResourceBinding {
                    set: Some(#set),
                    index: #binding,
                    ty: #G::descriptor::ResourceBindingType::RwImage(#dim),
                    stage_flags: #stage_flags,
                    count: 1,
                    data_ty: Some(&#sampled_ty),
                    data_layout: None,
                    data_format: #G::Format::#format
                }
            }
        } else if let &TypeDesc::Pointer(&TypeDesc::SampledImage(image_ty)) = v.ty {
            // sampled image -----------------------------------------------------------------------
            let sampled_ty = gen_type_info(image_ty.sampled_ty);
            let dim = gen_resource_shape(image_ty.dimensions);
            let format = gen_format_from_spirv(image_ty.format);
            quote! {
                #G::descriptor::ResourceBinding {
                    set: Some(#set),
                    index: #binding,
                    ty: #G::descriptor::ResourceBindingType::TextureSampler(#dim),
                    stage_flags: #stage_flags,
                    count: 1,
                    data_ty: Some(&#sampled_ty),
                    data_layout: None,
                    data_format: #G::Format::#format
                }
            }
        } else {
            s.unstable()
                .warning(format!("unsupported uniform constant type: {:?}", v))
                .emit();
            quote!()
        }
    } else {
        s.unstable()
            .warning(format!("unsupported shader interface: {:?}", v))
            .emit();
        quote!()
    }
}

fn gen_vertex_input_reflection_info(_s: &Span, v: &Variable, location: u32) -> TokenStream {
    let ty = gen_type_info(v.ty.pointee_type().expect("expected pointer type"));
    quote! {
        #G::pipeline::VertexInputAttributeDescription {
            location: Some(#location),
            ty: &#ty,
            semantic: None  // No match by semantic in GL or Vulkan
        }
    }
}

fn gen_fragment_output_reflection_info(_s: &Span, _v: &Variable, _location: u32) -> TokenStream {
    //let ty = gen_type_info(v.ty);
    quote! {
        #G::pipeline::FragmentOutputDescription {
            // TODO
        }
    }
}

pub fn generate_reflection_info(s: &Span, bytecode: &[u8], stage: ShaderKind) -> TokenStream {
    let a = spirv::DroplessArena::new();
    let m = spirv::Module::from_bytes(bytecode).expect("failed to parse generated SPIR-V module");
    let ast = spirv::ast::Ast::new(&a, &m);

    let mut descriptor_infos = Vec::new();
    let mut vtx_input_infos = Vec::new();
    let mut frag_output_infos = Vec::new();

    for (_, v) in ast.variables() {
        if let Some((_, set)) = v.descriptor_set_decoration() {
            // descriptor-backed interface ---------------------------------------------------------
            let (_, binding) = v.binding_decoration().expect("expected binding decoration");
            descriptor_infos.push(gen_descriptor_reflection_info(s, v, stage, set, binding));
        }

        if stage == ShaderKind::Vertex && v.storage == spirv::headers::StorageClass::Input {
            // vertex input interface --------------------------------------------------------------
            let (_, loc) = v
                .location_decoration()
                .expect("expected location decoration");
            vtx_input_infos.push(gen_vertex_input_reflection_info(s, v, loc));
        }

        if stage == ShaderKind::Fragment && v.storage == spirv::headers::StorageClass::Output {
            // fragment output interface -----------------------------------------------------------
            let (_, loc) = v
                .location_decoration()
                .expect("expected location decoration");
            frag_output_infos.push(gen_fragment_output_reflection_info(s, v, loc));
        }
    }

    let stage_flags = gen_stage_flags(stage);

    quote! {
        #G::pipeline::ShaderStageReflection {
            stage: #stage_flags,
            descriptors: &[#(#descriptor_infos,)*],
            vertex_input_attributes: &[#(#vtx_input_infos,)*],
            fragment_outputs: &[#(#frag_output_infos,)*],
        }
    }
}
