use crate::{
    descriptor::{ResourceBinding, ResourceBindingType},
    pipeline::FragmentOutputDescription,
    pipeline::{GraphicsPipelineCreateInfo, Scissors, SignatureDescription, Viewports},
    vertex::{IndexFormat, VertexLayout, VertexLayoutElement},
    Backend,
};
use autograph_spirv::{headers::StorageClass, TypeDesc};
use log::warn;
use std::{error, fmt};
/*
#[derive(Copy, Clone, Debug)]
pub enum Interface {
    Descriptor(u32, u32, ResourceBindingType),
    UnhandledDescriptor(u32, u32),
    VertexInput(u32),
    FragmentOutput(u32),
}

#[derive(Debug)]
pub enum ValidationError {
    InvalidSpirV(spirv::ParseError),
    InterfaceNotFound(Interface),
    DescriptorTypeMismatch {
        interface: Interface,
        host: ResourceBindingType,
    },
    TypeError {
        interface: Interface,
        details: TypeError,
    },
    VertexInputsOutsideRootSignature,
    FragmentOutputsOutsideRootSignature,
    MultipleVertexRootSignatures,
    MultipleFragmentRootSignatures,
    MultipleIndexBufferDescriptors,
    MultipleDepthStencilRenderTargetDescriptors,
    DynamicViewportsDisabled,
    DynamicScissorsDisabled,
    ViewportCountMismatch {
        num_viewports: usize,
        num_scissors: usize,
    },
    InvalidRootSignature(String),
}

#[derive(Debug)]
pub enum TypeError {
    /// The types do not match.
    TypeMismatch {
        shader_ty: String,
        host_ty: String,
        //details: Option<Box<TypeError>>
    },
    /// The type of a member does not match.
    MemberTypeMismatch {
        shader_member_index: usize,
        shader_ty: String,
        host_member_index: usize,
        host_ty: String,
        details: Option<Box<TypeError>>,
    },
    /// The offset of a member does not match.
    MemberOffsetMismatch {
        shader_member_index: usize,
        shader_member_offset: usize,
        //shader_ty: String,
        host_member_index: usize,
        host_member_offset: usize,
        //host_ty: String,
        //details:
    },
}
/*
fn format_ty(ty: &TypeDesc) -> String
{
    match ty {
        TypeDesc::Primitive(p) => {
            match p {
                PrimitiveType::Int => "int".to_string(),
                PrimitiveType::UnsignedInt => "uint".to_string(),
                PrimitiveType::Half => "half".to_string(),
                PrimitiveType::Float => "float".to_string(),
                PrimitiveType::Double => "double".to_string(),
                PrimitiveType::Bool => "bool".to_string(),
            }
        }
        TypeDesc::Vector(p, len) => {
            format!("<vector {} x {}>", len, format_ty(&TypeDesc::Primitive(*p)))
        },
        TypeDesc::Struct(layout) => {
            format!("<struct>"),
        }
    }
}

fn compare_types(shader_ty: &TypeDesc, host_ty: &TypeDesc) -> Result<(), TypeError> {
    match (shader_ty, host_ty) {
        /////////////////////////////////////////////////////////
        (&TypeDesc::Primitive(a), &TypeDesc::Primitive(b)) => {
            if a != b {
                return Err(TypeError::TypeMismatch {
                    shader_ty: format!("{:?}", shader_ty),
                    host_ty: format!("{:?}", host_ty),
                    //details: None,
                })
            }
        }
        /////////////////////////////////////////////////////////
        (
            &TypeDesc::Vector(shader_comp_ty, shader_num_comp),
            &TypeDesc::Vector(host_comp_ty, host_num_comp),
        ) => {
            compare_types(
                &TypeDesc::Primitive(shader_comp_ty),
                &TypeDesc::Primitive(host_comp_ty),
            ).context(format!(
                "type mismatch: {:?} (shader) and {:?} (host)",
                shader_ty, host_ty
            ))?;
            if shader_num_comp != host_num_comp {
                bail!(
                    "vector size mismatch: {} (shader) and {} (host)",
                    shader_num_comp,
                    host_num_comp
                )
            }
        }
        /////////////////////////////////////////////////////////
        (
            &TypeDesc::Matrix(shader_ty, shader_rows, shader_cols),
            &TypeDesc::Matrix(host_ty, host_rows, host_cols),
        ) => {
            compare_types(
                &TypeDesc::Primitive(shader_ty),
                &TypeDesc::Primitive(host_ty),
            ).context(format!(
                "type mismatch: {:?} (shader) and {:?} (host)",
                shader_ty, host_ty
            ))?;
            if !(shader_rows == host_rows && shader_cols == host_cols) {
                bail!(
                    "matrix size mismatch: {}x{} (shader) and {}x{} (host)",
                    shader_rows,
                    shader_cols,
                    host_rows,
                    host_cols
                )
            }
        }
        /////////////////////////////////////////////////////////
        (&TypeDesc::Struct(ref shader), &TypeDesc::Struct(ref host)) => {
            let mut shader_member_index = 0;
            let mut host_member_index = 0;

            loop {
                let host_member = host.get(host_member_index);
                let shader_member = shader.get(shader_member_index);
                // TODO ignore padding fields
                match (host_member, shader_member) {
                    (Some(host_ty), Some(shader_ty)) => {
                        compare_types(&host_ty.1, &shader_ty.1).context(format!("member type mismatch: #{}({}) (shader) and #{}({}) (host)",
                                                                                shader_member_index, "<unnamed>",
                                                                                host_member_index, "<unnamed>"))?;
                        let shader_offset = shader_ty.0;
                        let host_offset = host_ty.0;
                        if host_ty.0 != shader_ty.0 {
                            bail!("member offset mismatch: #{}({}) @ {} (shader) and #{}({}) @ {} (host)",
                                    shader_member_index, "<unnamed>",
                                    shader_offset,
                                    host_member_index, "<unnamed>",
                                    host_offset);
                        }
                    },
                    (None, None) => { break },
                    _ => bail!("shader and host structs do not have the same number of non-padding members")
                }
                host_member_index += 1;
                shader_member_index += 1;
            }
        }
        _ => bail!(
            "type mismatch: {:?} (shader) and {:?} (host)",
            shader_ty,
            host_ty
        ),
    }
    Ok(())
}*/

type ValidationResult<T> = Result<T, ValidationError>;

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ValidationError::InvalidSpirV(err) => write!(f, "invalid SPIR-V bytecode: {:?}", err)?,
            ValidationError::InvalidRootSignature(err) => {
                write!(f, "invalid root signature: {}", err)?;
            }
            ValidationError::InterfaceNotFound(interface) => {
                write!(f, "interface not found in shader: {:#?}", interface)?;
            }
            ValidationError::DescriptorTypeMismatch { interface, host } => {
                let (set, binding, ty) = if let Interface::Descriptor(set, binding, ty) = interface
                {
                    (set, binding, ty)
                } else {
                    panic!()
                };
                write!(f, "descriptor type does not match: (set,binding)=({},{}), {:?} (host) vs. {:?} (shader)", set, binding, host, ty)?;
            }
            ValidationError::TypeError { interface, details } => {
                write!(f, "type error ({:?}): {:#?}", interface, details)?;
            }
            ValidationError::VertexInputsOutsideRootSignature => {
                write!(f, "additional vertex inputs outside of root signature")?;
            }
            ValidationError::FragmentOutputsOutsideRootSignature => {
                write!(f, "additional fragment outputs outside of root signature")?;
            }
            ValidationError::MultipleVertexRootSignatures => {
                write!(f, "multiple vertex input root signatures")?;
            }
            ValidationError::MultipleFragmentRootSignatures => {
                write!(f, "multiple fragment output root signatures")?;
            }
            ValidationError::MultipleIndexBufferDescriptors => {
                write!(f, "multiple index buffer descriptors")?;
            }
            ValidationError::MultipleDepthStencilRenderTargetDescriptors => {
                write!(f, "multiple depth stencil render target descriptors")?;
            }
            ValidationError::DynamicViewportsDisabled => {
                write!(f, "root signature contains viewport entries but dynamic viewports are not enabled for this pipeline")?;
            }
            ValidationError::DynamicScissorsDisabled => {
                write!(f, "root signature contains scissor entries but dynamic viewports are not enabled for this pipeline")?;
            }
            ValidationError::ViewportCountMismatch {
                num_viewports,
                num_scissors,
            } => {
                write!(f, "the number of scissor entries does not match the number of viewports (num_scissors={}, num_viewports={})", num_scissors, num_viewports)?;
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
    host: ResourceBindingType,
    shader: ResourceBindingType,
    interface: Interface,
) -> ValidationResult<()> {
    if host != shader {
        Err(ValidationError::DescriptorTypeMismatch { interface, host })
    } else {
        Ok(())
    }
}

fn validate_data_type(
    host: &TypeDesc,
    shader: &TypeDesc,
    interface: Interface,
) -> ValidationResult<()> {
    if host != shader {
        Err(ValidationError::TypeError {
            interface,
            details: TypeError::TypeMismatch {
                shader_ty: format!("{:?}", shader),
                host_ty: format!("{:?}", host),
            },
        })
    } else {
        Ok(())
    }
}

struct ValidationInfo<'a> {
    descriptor_sets: Vec<Vec<(&'a ResourceBinding<'a>, bool)>>,
    fragment_outputs: Vec<FragmentOutputDescription>,
    vertex_layouts: Vec<VertexLayout<'a>>,
    vertex_attributes: Vec<TypedVertexInputAttributeDescription<'a>>,
    /// Encountered root sig for vertex inputs
    vertex_input_root: bool,
    /// Encountered index buffer
    index_format: Option<IndexFormat>,
    /// Encountered depth-stencil frag output
    depth_stencil_fragment_output: Option<FragmentOutputDescription>,
    /// Encountered root sig for fragment outputs
    fragment_output_root: bool,
    /// Number of viewport entries
    num_viewports: usize,
    /// Number of scissor entries
    num_scissors: usize,
}

impl<'a> ValidationInfo<'a> {
    fn new() -> ValidationInfo<'a> {
        ValidationInfo {
            descriptor_sets: vec![],
            fragment_outputs: vec![],
            vertex_layouts: vec![],
            vertex_attributes: vec![],
            vertex_input_root: false,
            index_format: None,
            depth_stencil_fragment_output: None,
            fragment_output_root: false,
            num_viewports: 0,
            num_scissors: 0,
        }
    }

    ///
    /// # Validation of root pipeline signatures
    ///
    fn build(
        &mut self,
        signature: &'a SignatureDescription<'a>,
    ) -> Result<(), Vec<ValidationError>> {
        let mut errs = Vec::new();

        for &subsig in signature.inherited {
            self.build(subsig)?;
        }

        // If this block has descriptors, then this block defines a new descriptor set
        if signature.descriptors.len() > 0 {
            self.descriptor_sets
                .push(signature.descriptors.iter().map(|d| (d, false)).collect());
        }

        self.fragment_outputs.extend(signature.fragment_outputs);
        self.vertex_layouts.extend(signature.vertex_layouts);
        self.vertex_attributes
            .extend(signature.vertex_layouts.iter().flat_map(|l| l.elements));

        // Vertex input root already encountered but adding new inputs
        if self.vertex_input_root
            && (signature.vertex_layouts.len() > 0 || signature.index_format.is_some())
        {
            errs.push(ValidationError::VertexInputsOutsideRootSignature);
        }

        // Fragment output root already encountered but adding new outputs
        if self.fragment_output_root
            && (signature.fragment_outputs.len() > 0
                || signature.depth_stencil_fragment_output.is_some())
        {
            errs.push(ValidationError::FragmentOutputsOutsideRootSignature);
        }

        if signature.is_root_vertex_input_signature {
            if self.vertex_input_root {
                errs.push(ValidationError::MultipleVertexRootSignatures);
            } else {
                self.vertex_input_root = true;
            }
        }

        if signature.is_root_fragment_output_signature {
            if self.fragment_output_root {
                errs.push(ValidationError::MultipleFragmentRootSignatures);
            } else {
                self.fragment_output_root = true;
            }
        }

        if let Some(index_format) = signature.index_format {
            if self.index_format.is_some() {
                errs.push(ValidationError::MultipleIndexBufferDescriptors);
            } else {
                self.index_format = Some(index_format);
            }
        }

        if let Some(depth_stencil_fragment_output) = signature.depth_stencil_fragment_output {
            if self.depth_stencil_fragment_output.is_some() {
                errs.push(ValidationError::MultipleDepthStencilRenderTargetDescriptors);
            } else {
                self.depth_stencil_fragment_output = Some(depth_stencil_fragment_output);
            }
        }

        self.num_viewports += signature.num_viewports;
        self.num_scissors += signature.num_scissors;

        if errs.len() > 0 {
            Err(errs)
        } else {
            Ok(())
        }
    }

    fn use_descriptor(
        &mut self,
        set: u32,
        binding: u32,
        interface: Interface,
    ) -> ValidationResult<&'a ResourceBinding<'a>> {
        let set = self.descriptor_sets.get_mut(set as usize);
        if let Some(set) = set {
            for (b, ref mut seen) in set.iter_mut() {
                if b.index == binding as usize {
                    //let () = seen;
                    *seen = true;
                    return Ok(b);
                }
            }
        }
        Err(ValidationError::InterfaceNotFound(interface))
    }

    fn validate_vertex_input(
        &mut self,
        location: u32,
        v: &spirv::ast::Variable,
    ) -> ValidationResult<()> {
        let attr = self
            .vertex_attributes
            .get(location as usize)
            .ok_or_else(|| ValidationError::InterfaceNotFound(Interface::VertexInput(location)))?;

        if attr.ty != unwrap_ptr_type(v.ty) {
            Err(ValidationError::TypeError {
                interface: Interface::VertexInput(location),
                details: TypeError::TypeMismatch {
                    host_ty: format!("{:?}", attr.ty),
                    shader_ty: format!("{:?}", v.ty),
                },
            })
        } else {
            Ok(())
        }
    }

    fn validate_descriptor(
        &mut self,
        set: u32,
        binding: u32,
        v: &spirv::ast::Variable,
    ) -> ValidationResult<()> {
        let has_buffer_block_deco = v.has_buffer_block_decoration().is_some();

        if v.storage == StorageClass::Uniform
        /*&& has_block_deco*/
        {
            // uniform buffer --------------------------------------------------------------------------
            let interface = Interface::Descriptor(set, binding, ResourceBindingType::UniformBuffer);
            let desc = self.use_descriptor(set, binding, interface)?;
            validate_descriptor_type(
                desc.ty,
                ResourceBindingType::UniformBuffer,
                interface,
            )?;
            let shader_ty = unwrap_ptr_type(v.ty);
            if let Some(tydesc) = desc.data_ty {
                validate_data_type(tydesc, shader_ty, interface)?;
            }
            Ok(())
        } else if (v.storage == StorageClass::Uniform && has_buffer_block_deco)
            || (v.storage == StorageClass::StorageBuffer)
        {
            // shader storage buffer -------------------------------------------------------------------
            let interface = Interface::Descriptor(set, binding, ResourceBindingType::StorageBuffer);
            let desc = self.use_descriptor(set, binding, interface)?;
            validate_descriptor_type(
                desc.ty,
                ResourceBindingType::StorageBuffer,
                interface,
            )?;
            let shader_ty = unwrap_ptr_type(v.ty);
            if let Some(tydesc) = desc.data_ty {
                validate_data_type(tydesc, shader_ty, interface)?;
            }
            Ok(())
        } else if v.storage == StorageClass::UniformConstant {
            if let &TypeDesc::Pointer(&TypeDesc::Image(_, _)) = v.ty {
                // image -------------------------------------------------------------------------------
                let interface = Interface::Descriptor(set, binding, ResourceBindingType::StorageImage);
                let desc = self.use_descriptor(set, binding, interface)?;
                validate_descriptor_type(
                    desc.ty,
                    ResourceBindingType::StorageImage,
                    interface,
                )?;
                Ok(())
            } else if let &TypeDesc::Pointer(&TypeDesc::SampledImage(_, _)) = v.ty {
                // sampled image -----------------------------------------------------------------------
                let interface = Interface::Descriptor(set, binding, ResourceBindingType::SampledImage);
                let desc = self.use_descriptor(set, binding, interface)?;
                validate_descriptor_type(
                    desc.ty,
                    ResourceBindingType::SampledImage,
                    interface,
                )?;
                Ok(())
            } else {
                warn!("unhandled uniform constant type: {:?}", v);
                let interface = Interface::UnhandledDescriptor(set, binding);
                let _ = self.use_descriptor(set, binding, interface)?;
                Ok(())
            }
        } else {
            warn!("unhandled shader interface: {:?}", v);
            let interface = Interface::UnhandledDescriptor(set, binding);
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
pub fn validate_spirv_graphics_pipeline<B: Backend>(
    signature_desc: &SignatureDescription,
    create_info: &GraphicsPipelineCreateInfo<B>,
) -> Result<(), Vec<ValidationError>> {
    let a = spirv::ast::Arenas::new();
    // create SPIR-V modules
    let vert = spirv::Module::from_bytes(create_info.shader_stages.vertex.1)
        .map_err(|e| vec![ValidationError::InvalidSpirV(e)])?;
    // yay transpose
    let frag = create_info
        .shader_stages
        .fragment
        .map(|s| spirv::Module::from_bytes(s.1))
        .transpose()
        .map_err(|e| vec![ValidationError::InvalidSpirV(e)])?;
    let geom = create_info
        .shader_stages
        .geometry
        .map(|s| spirv::Module::from_bytes(s.1))
        .transpose()
        .map_err(|e| vec![ValidationError::InvalidSpirV(e)])?;
    let tese = create_info
        .shader_stages
        .tess_eval
        .map(|s| spirv::Module::from_bytes(s.1))
        .transpose()
        .map_err(|e| vec![ValidationError::InvalidSpirV(e)])?;
    let tesc = create_info
        .shader_stages
        .tess_control
        .map(|s| spirv::Module::from_bytes(s.1))
        .transpose()
        .map_err(|e| vec![ValidationError::InvalidSpirV(e)])?;

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
    vinfo.build(signature_desc)?;

    let mut errors = Vec::new();

    // check viewports & scissors
    let num_viewports = match create_info.viewport_state.viewports {
        Viewports::Static(vp) => {
            if vinfo.num_viewports != 0 {
                errors.push(ValidationError::DynamicViewportsDisabled);
            }
            vp.len()
        }
        Viewports::Dynamic => vinfo.num_viewports,
    };

    let num_scissors = match create_info.viewport_state.scissors {
        Scissors::Static(s) => {
            if vinfo.num_scissors != 0 {
                errors.push(ValidationError::DynamicScissorsDisabled);
            }
            s.len()
        }
        Scissors::Dynamic => vinfo.num_scissors,
    };

    if num_viewports != num_scissors {
        errors.push(ValidationError::ViewportCountMismatch {
            num_viewports,
            num_scissors,
        });
    }

    // iterate over all variables
    let all_vars = vert_ast
        .variables()
        .chain(frag_ast.iter().flat_map(|ast| ast.variables()))
        .chain(geom_ast.iter().flat_map(|ast| ast.variables()))
        .chain(tese_ast.iter().flat_map(|ast| ast.variables()))
        .chain(tesc_ast.iter().flat_map(|ast| ast.variables()));

    for (_, v) in all_vars {
        if let Some((_, set)) = v.descriptor_set_decoration() {
            // descriptor-backed interface ---------------------------------------------------------
            let (_, binding) = v.binding_decoration().expect("expected binding decoration");
            let result = vinfo.validate_descriptor(set, binding, v);
            if let Err(e) = result {
                errors.push(e);
            }
        }
    }

    for (_, v) in vert_ast.variables() {
        if v.storage == StorageClass::Input {
            // vertex input interface --------------------------------------------------------------
            let (_, loc) = v
                .location_decoration()
                .expect("expected location decoration");
            let result = vinfo.validate_vertex_input(loc, v);
            if let Err(e) = result {
                errors.push(e);
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
*/
