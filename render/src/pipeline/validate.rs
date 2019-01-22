use crate::descriptor::Descriptor;
use crate::descriptor::DescriptorSetLayoutBinding;
use crate::descriptor::DescriptorType;
use crate::pipeline::GraphicsPipelineCreateInfo;
use crate::pipeline::GraphicsPipelineCreateInfoTypeless;
use autograph_spirv as spirv;
use autograph_spirv::headers::StorageClass;
use autograph_spirv::TypeDesc;
use log::warn;
use std::error;
use std::fmt;

#[derive(Copy, Clone, Debug)]
pub enum InterfaceItem {
    Descriptor(u32, u32, DescriptorType),
    UnhandledDescriptor(u32, u32),
    VertexInput(u32),
}

#[derive(Debug)]
pub enum InterfaceMismatchError {
    NotFound(InterfaceItem),
    DescriptorTypeMismatch {
        interface: InterfaceItem,
        host: DescriptorType,
    },
    DataTypeMismatch {
        interface: InterfaceItem,
        shader_ty: String,
        host_ty: String,
    },
}

#[derive(Debug)]
pub enum ValidationError {
    InvalidSpirV(spirv::ParseError),
    InterfaceMismatch(Vec<InterfaceMismatchError>),
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ValidationError::InvalidSpirV(err) => write!(f, "invalid SPIR-V bytecode")?,
            ValidationError::InterfaceMismatch(errs) => {
                writeln!(f, "interface mismatch ({} errors):", errs.len())?;
                for err in errs.iter() {
                    writeln!(f, "{:?}", err)?;
                }
            }
        }
        Ok(())
    }
}

impl error::Error for ValidationError {}

fn unwrap_ptr_type<'a>(ptr: &'a TypeDesc<'a>) -> &'a TypeDesc<'a> {
    if let &TypeDesc::Pointer(ty) = ptr {
        ty
    } else {
        panic!("expected pointer type")
    }
}

fn validate_descriptor_type(
    host: DescriptorType,
    shader: DescriptorType,
    interface: InterfaceItem,
) -> Result<(), InterfaceMismatchError> {
    if host != shader {
        Err(InterfaceMismatchError::DescriptorTypeMismatch { interface, host })
    } else {
        Ok(())
    }
}

fn validate_data_type(
    host: &TypeDesc,
    shader: &TypeDesc,
    interface: InterfaceItem,
) -> Result<(), InterfaceMismatchError> {
    if host != shader {
        Err(InterfaceMismatchError::DataTypeMismatch {
            interface,
            shader_ty: format!("{:?}", shader),
            host_ty: format!("{:?}", host),
        })
    } else {
        Ok(())
    }
}

type DescriptorValidationMap<'a> = Vec<Vec<(&'a DescriptorSetLayoutBinding<'a>, bool)>>;

fn find_descriptor<'a>(
    map: &'_ mut DescriptorValidationMap<'a>,
    set: u32,
    binding: u32,
    interface: InterfaceItem,
) -> Result<&'a DescriptorSetLayoutBinding<'a>, InterfaceMismatchError> {
    let set = map.get_mut(set as usize);
    if let Some(set) = set {
        for (b, mut seen) in set.iter() {
            if b.binding == binding {
                seen = true;
                return Ok(b);
            }
        }
    }
    Err(InterfaceMismatchError::NotFound(interface))
}

fn validate_descriptor(
    map: &mut DescriptorValidationMap,
    set: u32,
    binding: u32,
    v: &spirv::ast::Variable,
) -> Result<(), InterfaceMismatchError> {
    let has_buffer_block_deco = v.has_buffer_block_decoration().is_some();

    if v.storage == StorageClass::Uniform
    /*&& has_block_deco*/
    {
        // uniform buffer --------------------------------------------------------------------------
        let interface = InterfaceItem::Descriptor(set, binding, DescriptorType::UniformBuffer);
        let desc = find_descriptor(map, set, binding, interface)?;
        validate_descriptor_type(
            desc.descriptor_type,
            DescriptorType::UniformBuffer,
            interface,
        )?;
        let shader_ty = unwrap_ptr_type(v.ty);
        if let Some(tydesc) = desc.tydesc {
            validate_data_type(tydesc, shader_ty, interface)?;
        }
        Ok(())
    } else if (v.storage == StorageClass::Uniform && has_buffer_block_deco)
        || (v.storage == StorageClass::StorageBuffer)
    {
        // shader storage buffer -------------------------------------------------------------------
        let interface = InterfaceItem::Descriptor(set, binding, DescriptorType::StorageBuffer);
        let desc = find_descriptor(map, set, binding, interface)?;
        validate_descriptor_type(
            desc.descriptor_type,
            DescriptorType::StorageBuffer,
            interface,
        )?;
        let shader_ty = unwrap_ptr_type(v.ty);
        if let Some(tydesc) = desc.tydesc {
            validate_data_type(tydesc, shader_ty, interface)?;
        }
        Ok(())
    } else if v.storage == StorageClass::UniformConstant {
        if let &TypeDesc::Pointer(&TypeDesc::Image(_, _)) = v.ty {
            // image -------------------------------------------------------------------------------
            let interface = InterfaceItem::Descriptor(set, binding, DescriptorType::StorageImage);
            let desc = find_descriptor(map, set, binding, interface)?;
            validate_descriptor_type(
                desc.descriptor_type,
                DescriptorType::StorageImage,
                interface,
            )?;
            Ok(())
        } else if let &TypeDesc::Pointer(&TypeDesc::SampledImage(_, _)) = v.ty {
            // sampled image -----------------------------------------------------------------------
            let interface = InterfaceItem::Descriptor(set, binding, DescriptorType::SampledImage);
            let desc = find_descriptor(map, set, binding, interface)?;
            validate_descriptor_type(
                desc.descriptor_type,
                DescriptorType::SampledImage,
                interface,
            )?;
            Ok(())
        } else {
            warn!("unhandled uniform constant type: {:?}", v);
            let interface = InterfaceItem::UnhandledDescriptor(set, binding);
            let _ = find_descriptor(map, set, binding, interface)?;
            Ok(())
        }
    } else {
        warn!("unhandled shader interface: {:?}", v);
        let interface = InterfaceItem::UnhandledDescriptor(set, binding);
        let _ = find_descriptor(map, set, binding, interface)?;
        Ok(())
    }
}

pub fn validate_graphics(
    create_info: &GraphicsPipelineCreateInfoTypeless,
) -> Result<(), ValidationError> {
    let a = spirv::ast::Arenas::new();
    // create SPIR-V modules
    let vert = spirv::Module::from_bytes(create_info.shader_stages.vertex.1)
        .map_err(|e| ValidationError::InvalidSpirV(e))?;
    // yay transpose
    let frag = create_info
        .shader_stages
        .fragment
        .map(|s| spirv::Module::from_bytes(s.1))
        .transpose()
        .map_err(|e| ValidationError::InvalidSpirV(e))?;
    let geom = create_info
        .shader_stages
        .geometry
        .map(|s| spirv::Module::from_bytes(s.1))
        .transpose()
        .map_err(|e| ValidationError::InvalidSpirV(e))?;
    let tese = create_info
        .shader_stages
        .tess_eval
        .map(|s| spirv::Module::from_bytes(s.1))
        .transpose()
        .map_err(|e| ValidationError::InvalidSpirV(e))?;
    let tesc = create_info
        .shader_stages
        .tess_control
        .map(|s| spirv::Module::from_bytes(s.1))
        .transpose()
        .map_err(|e| ValidationError::InvalidSpirV(e))?;

    // parse into structured ASTs
    let vert_ast = spirv::ast::Ast::new(&a, &vert);
    let frag_ast = frag.as_ref().map(|m| spirv::ast::Ast::new(&a, &m));
    let geom_ast = geom.as_ref().map(|m| spirv::ast::Ast::new(&a, &m));
    let tese_ast = tese.as_ref().map(|m| spirv::ast::Ast::new(&a, &m));
    let tesc_ast = tesc.as_ref().map(|m| spirv::ast::Ast::new(&a, &m));

    // to check:
    // - vertex inputs
    // - uniform buffers
    // - fragment outputs
    //
    // naming:
    // - shader item: interface item present in the SPIR-V bytecode
    // - host item: interface item expected by the host application
    // - output item: interface item that receives data (fragment outputs, storage buffers)
    // - input item: interface item that is only read by the shader
    //
    // matching:
    // - no match if not present at the expected location, or if it's the wrong definition
    //
    // behavior:
    // - an *output* host item with no matching shader item is an error
    // - a shader item (input *OR* output) with no matching host item is an error
    // this means that we allow input items that are not bound to anything in the shader
    //
    // - First, collect all shader interface items, in all stages
    //      - if the definition of an item does not match between two stages, ignore: this will be caught during linking anyway
    // -

    // build descriptor map
    let mut desc_map: DescriptorValidationMap = create_info
        .descriptor_set_layouts
        .iter()
        .map(|dsl| {
            dsl.bindings
                .iter()
                .map(|dslb| (dslb, false))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    // iterate over all variables
    let all_vars = vert_ast
        .variables()
        .chain(frag_ast.iter().flat_map(|ast| ast.variables()))
        .chain(geom_ast.iter().flat_map(|ast| ast.variables()))
        .chain(tese_ast.iter().flat_map(|ast| ast.variables()))
        .chain(tesc_ast.iter().flat_map(|ast| ast.variables()));

    let mut errors = Vec::new();

    for (_, v) in all_vars {
        if let Some((_, set)) = v.descriptor_set_decoration() {
            // descriptor-backed interface ---------------------------------------------------------
            let (_, binding) = v.binding_decoration().expect("expected binding decoration");
            let result = validate_descriptor(&mut desc_map, set, binding, v);
            if let Err(e) = result {
                errors.push(e);
            }
        } else {
            // TODO vertex inputs
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(ValidationError::InterfaceMismatch(errors))
    }
}
