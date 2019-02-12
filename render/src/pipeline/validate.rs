//use crate::descriptor::Descriptor;
use crate::descriptor::DescriptorBinding;
use crate::descriptor::DescriptorType;
//use crate::pipeline::GraphicsPipelineCreateInfo;
use crate::pipeline::GraphicsPipelineCreateInfoTypeless;
use autograph_spirv as spirv;
use autograph_spirv::headers::StorageClass;
use autograph_spirv::TypeDesc;
use log::warn;
use std::error;
use std::fmt;
use crate::Backend;
use crate::pipeline::PipelineSignatureTypeless;
use crate::pipeline::PipelineSignatureDescription;
use crate::framebuffer::FragmentOutputDescription;
use crate::vertex::VertexLayout;
use crate::vertex::IndexFormat;

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
    InvalidRootSignature(Vec<String>)
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ValidationError::InvalidSpirV(_err) => write!(f, "invalid SPIR-V bytecode")?,
            ValidationError::InterfaceMismatch(errs) => {
                writeln!(f, "interface mismatch ({} errors):", errs.len())?;
                for err in errs.iter() {
                    writeln!(f, "{:?}", err)?;
                }
            }
            ValidationError::InvalidRootSignature(errs) => {
                writeln!(f, "invalid root signature:")?;
                for err in errs.iter() {
                    writeln!(f, "{:?}", err)?;
                }
            }
        }
        Ok(())
    }
}

impl error::Error for ValidationError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            ValidationError::InvalidSpirV(ref err) => Some(err),
            _ => None,
        }
    }
}

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


struct ValidationInfo<'a>
{
    descriptor_sets: Vec<Vec<(&'a DescriptorBinding<'a>, bool)>>,
    fragment_outputs: Vec<FragmentOutputDescription>,
    vertex_layouts: Vec<VertexLayout<'a>>,
    /// Encountered root sig for vertex inputs
    vertex_input_root: bool,
    /// Encountered index buffer
    index_format: Option<IndexFormat>,
    /// Encountered depth-stencil frag output
    depth_stencil_fragment_output: Option<FragmentOutputDescription>,
    /// Encountered root sig for fragment outputs
    fragment_output_root: bool,
}

impl<'a> ValidationInfo<'a>
{
    fn new() -> ValidationInfo<'a> {
        ValidationInfo {
            descriptor_sets: vec![],
            fragment_outputs: vec![],
            vertex_layouts: vec![],
            vertex_input_root: false,
            index_format: None,
            depth_stencil_fragment_output: None,
            fragment_output_root: false
        }
    }

    ///
    /// # Validation of root pipeline signatures
    ///
    fn build(&mut self, signature: &'a PipelineSignatureDescription<'a>) -> Result<(), ValidationError>
    {
        let mut errs = Vec::new();

        for &subsig in signature.sub_signatures {
            self.build(subsig);
        }

        // If this block has descriptors, then this block defines a new descriptor set
        if signature.descriptors.len() > 0 {
            self.descriptor_sets.push(signature.descriptors.iter().map(|d| (d, false)).collect());
        }

        self.fragment_outputs.extend(signature.fragment_outputs);
        self.vertex_layouts.extend(signature.vertex_layouts);

        // Vertex input root already encountered but adding new inputs
        if self.vertex_input_root && (signature.vertex_layouts.len() > 0 || signature.index_format.is_some()) {
            errs.push("additional vertex inputs outside of root signature".to_string());
        }

        // Fragment output root already encountered but adding new outputs
        if self.fragment_output_root && (signature.fragment_outputs.len() > 0 || signature.depth_stencil_fragment_output.is_some()) {
            errs.push("additional fragment outputs outside of root signature".to_string());
        }

        if signature.is_root_vertex_input_signature {
            if self.vertex_input_root {
                errs.push("multiple vertex input root signatures".to_string());
            } else {
                self.vertex_input_root = true;
            }
        }

        if signature.is_root_fragment_output_signature {
            if self.fragment_output_root {
                errs.push("multiple fragment output root signatures".to_string());
            } else {
                self.fragment_output_root = true;
            }
        }

        if let Some(index_format) = signature.index_format {
            if self.index_format.is_some() {
                errs.push("multiple index buffer descriptors".to_string());
            } else {
                self.index_format = Some(index_format);
            }
        }

        if let Some(depth_stencil_fragment_output) = signature.depth_stencil_fragment_output {
            if self.depth_stencil_fragment_output.is_some() {
                errs.push("multiple depth stencil render target descriptors".to_string());
            } else {
                self.depth_stencil_fragment_output = Some(depth_stencil_fragment_output);
            }
        }

        // we don't care about viewports

        if errs.len() > 0 {
            Err(ValidationError::InvalidRootSignature(errs))
        } else {
            Ok(())
        }
    }

    fn use_descriptor(
        &mut self,
        set: u32,
        binding: u32,
        interface: InterfaceItem,
    ) -> Result<&'a DescriptorBinding<'a>, InterfaceMismatchError> {
        let set = self.descriptor_sets.get_mut(set as usize);
        if let Some(set) = set {
            for (b, ref mut seen) in set.iter_mut() {
                if b.binding == binding as usize {
                    //let () = seen;
                    *seen = true;
                    return Ok(b);
                }
            }
        }
        Err(InterfaceMismatchError::NotFound(interface))
    }


    fn validate_descriptor(
        &mut self,
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
            let desc = self.use_descriptor(set, binding, interface)?;
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
            let desc = self.use_descriptor(set, binding, interface)?;
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
                let desc = self.use_descriptor(set, binding, interface)?;
                validate_descriptor_type(
                    desc.descriptor_type,
                    DescriptorType::StorageImage,
                    interface,
                )?;
                Ok(())
            } else if let &TypeDesc::Pointer(&TypeDesc::SampledImage(_, _)) = v.ty {
                // sampled image -----------------------------------------------------------------------
                let interface = InterfaceItem::Descriptor(set, binding, DescriptorType::SampledImage);
                let desc = self.use_descriptor(set, binding, interface)?;
                validate_descriptor_type(
                    desc.descriptor_type,
                    DescriptorType::SampledImage,
                    interface,
                )?;
                Ok(())
            } else {
                warn!("unhandled uniform constant type: {:?}", v);
                let interface = InterfaceItem::UnhandledDescriptor(set, binding);
                let _ = self.use_descriptor(set, binding, interface)?;
                Ok(())
            }
        } else {
            warn!("unhandled shader interface: {:?}", v);
            let interface = InterfaceItem::UnhandledDescriptor(set, binding);
            let _ = self.use_descriptor(set, binding, interface)?;
            Ok(())
        }
    }

}



/// Basic verification of graphics pipeline interfaces.
///
/// # TODO
/// - better error messages
///     - fine-grained type comparison
///     - member offset mismatch
/// - validate image/texture types
/// - vertex inputs
/// - fragment outputs
/// - validate host-side required outputs
/// - accept layout-equivalent types (?)
///
pub fn validate_spirv_graphics_pipeline(
    shader_stages: &SpirvGraphicsShaderStages,

    viewport_state: &ViewportState,
    rasterization_state: &RasterisationState,
    multisample_state: &MultisampleState,
    depth_stencil_state: &DepthStencilState,
    input_assembly_state: &InputAssemblyState,
    color_blend_state: &ColorBlendState,
    // traversal of the root signature in depth-first order
    signatures: &[&PipelineSignatureDescription],
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
    let mut vinfo = ValidationInfo::new();
    vinfo.build(create_info.root_signature.description());


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
            let result = vinfo.validate_descriptor(set, binding, v);
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
