use super::*;

//--------------------------------------------------------------------------------------------------

// three things:
// - RenderPassBuilder (attachment only)
// - SubpassBuilder (descriptors only)
// - GraphicsPassBuilder (all)

/// DOCUMENT
#[derive(Copy, Clone, Debug)]
pub enum AttachmentIndex {
    Color(u32),
    DepthStencil,
}

/// DOCUMENT
#[derive(Copy, Clone, Debug)]
pub struct AttachmentLoadStore {
    /// DOCUMENT
    pub load_op: vk::AttachmentLoadOp,
    /// DOCUMENT
    pub store_op: vk::AttachmentStoreOp,
    /// DOCUMENT
    pub stencil_load_op: vk::AttachmentLoadOp,
    /// DOCUMENT
    pub stencil_store_op: vk::AttachmentStoreOp,
}

impl AttachmentLoadStore {
    /// "Read and forget": the contents of the attachment may not be written after the end of the pass.
    pub fn forget() -> Self {
        AttachmentLoadStore {
            load_op: vk::AttachmentLoadOp::Load,
            store_op: vk::AttachmentStoreOp::DontCare,
            stencil_load_op: vk::AttachmentLoadOp::Load,
            stencil_store_op: vk::AttachmentStoreOp::DontCare,
        }
    }

    /// The contents are written to the resource after the pass.
    pub fn preserve() -> Self {
        AttachmentLoadStore {
            load_op: vk::AttachmentLoadOp::Load,
            store_op: vk::AttachmentStoreOp::Store,
            stencil_load_op: vk::AttachmentLoadOp::Load,
            stencil_store_op: vk::AttachmentStoreOp::Store,
        }
    }

    /// The contents before the pass are ignored, instead returning a clear value.
    pub fn clear() -> Self {
        AttachmentLoadStore {
            load_op: vk::AttachmentLoadOp::Clear,
            store_op: vk::AttachmentStoreOp::Store,
            stencil_load_op: vk::AttachmentLoadOp::Clear,
            stencil_store_op: vk::AttachmentStoreOp::Store,
        }
    }

    /// The contents before the pass are ignored, and not written at the end of the pass.
    pub fn transient() -> Self {
        AttachmentLoadStore {
            load_op: vk::AttachmentLoadOp::Clear,
            store_op: vk::AttachmentStoreOp::DontCare,
            stencil_load_op: vk::AttachmentLoadOp::Clear,
            stencil_store_op: vk::AttachmentStoreOp::DontCare,
        }
    }

    /// blah blah.
    pub fn write_only() -> Self {
        AttachmentLoadStore {
            load_op: vk::AttachmentLoadOp::Clear,
            store_op: vk::AttachmentStoreOp::DontCare,
            stencil_load_op: vk::AttachmentLoadOp::Clear,
            stencil_store_op: vk::AttachmentStoreOp::DontCare,
        }
    }
}

impl Default for AttachmentLoadStore {
    fn default() -> Self {
        // default is preserve
        AttachmentLoadStore::preserve()
    }
}

#[derive(Debug)]
pub(crate) struct GraphicsTask {
    renderpass: RenderPassId,
    color_attachments: Vec<vk::AttachmentReference>,
    input_attachments: Vec<vk::AttachmentReference>,
    resolve_attachments: Vec<vk::AttachmentReference>,
    depth_attachment: Option<vk::AttachmentReference>,
    shader_images: Vec<ImageId>,
}

impl GraphicsTask {
    /*fn is_used_as_shader_image(&self, img: ImageId) -> Result<(), ()> {
        if self.shader_images.contains(&img) {
            // already used as shader-accessible resource
            Err(())
        } else {
            Ok(())
        }
    }

    fn get_attachment_image_id(&self, index: u32) -> ImageId {
        self.attachments[index as usize]
    }

    fn get_attachment_desc(&self, index: u32) -> &vk::AttachmentDescription {
        &self.attachments_desc[index as usize]
    }*/
}

//--------------------------------------------------------------------------------------------------

/// Task builder specifically for graphics
pub struct GraphicsTaskBuilder<'frame, 'ctx: 'frame> {
    frame: &'frame mut Frame<'ctx>,
    task: TaskId,
    graphics_task: GraphicsTask,
}

impl<'frame, 'ctx: 'frame> GraphicsTaskBuilder<'frame, 'ctx> {

    pub(super) fn new(
        frame: &'frame mut Frame<'ctx>,
        name: impl Into<String>,
        renderpass: RenderPassId,
    ) -> GraphicsTaskBuilder<'frame, 'ctx> {
        // create a dummy node in the graph that we will fill up later.
        // this avoids looking into the graph every time we modify something,
        // and still allows us to create dependencies in the graph
        let task = frame.create_task_on_queue(name, 0, TaskDetails::Other);
        GraphicsTaskBuilder {
            frame,
            task,
            graphics_task: GraphicsTask {
                renderpass,
                shader_images: Vec::new(),
                color_attachments: Vec::new(),
                input_attachments: Vec::new(),
                resolve_attachments: Vec::new(),
                depth_attachment: None,
            },
        }
    }

    /// Adds the specified as an image sample dependency on the task.
    pub fn sample_image(&mut self, img: &ImageRef) {
        img.set_read().expect("R/W conflict");

        self.frame
            .add_or_check_image_usage(img.id, vk::IMAGE_USAGE_SAMPLED_BIT);

        self.frame.add_dependency(
            img.task,
            self.task,
            Dependency {
                src_stage_mask: img.src_stage_mask,
                dst_stage_mask: vk::PIPELINE_STAGE_VERTEX_SHADER_BIT,
                barrier: BarrierDetail::Image(ImageBarrier {
                    id: img.id,
                    old_layout: vk::ImageLayout::Undefined,
                    new_layout: vk::ImageLayout::ShaderReadOnlyOptimal,
                    src_access_mask: vk::AccessFlags::empty(),
                    dst_access_mask: vk::ACCESS_SHADER_READ_BIT,
                }),
                latency: img.latency,
            },
        );

        self.graphics_task.shader_images.push(img.id);
    }

    /*/// Specifies a depth attachment.
    pub fn set_depth_attachment(&mut self, depth_attachment: AttachmentReference) {
        self.graphics_task.pass_depth_attachment = Some(depth_attachment.vk_ref.clone());
        if let Some(dependency) = depth_attachment.dependency {
            self.frame.add_dependency_access_flags(
                dependency,
                vk::ACCESS_DEPTH_STENCIL_ATTACHMENT_READ_BIT
                    | vk::ACCESS_DEPTH_STENCIL_ATTACHMENT_WRITE_BIT,
            );
        }
        let img = self
            .graphics_task
            .get_attachment_image_id(depth_attachment.vk_ref.attachment);
        self.frame
            .add_or_check_image_usage(img, vk::IMAGE_USAGE_DEPTH_STENCIL_ATTACHMENT_BIT);
    }*/

    //----------------------------------------------------------------------------------------------
    // BIND ATTACHMENTS

    pub fn set_depth_attachment(&mut self, depth_attachment: &AttachmentRef)
    {
        self.graphics_task.depth_attachment = Some(vk::AttachmentReference {
            attachment: depth_attachment.id.index,
            layout: vk::ImageLayout::DepthStencilAttachmentOptimal  // FIXME may be read only
        });

        self.frame
            .add_or_check_image_usage(depth_attachment.id.img, vk::IMAGE_USAGE_DEPTH_STENCIL_ATTACHMENT_BIT);

        /*
        if let Some(dependency) = depth_attachment.dependency {
            self.frame.add_dependency_access_flags(
                dependency,
                vk::ACCESS_DEPTH_STENCIL_ATTACHMENT_READ_BIT
                    | vk::ACCESS_DEPTH_STENCIL_ATTACHMENT_WRITE_BIT,
            );
        }*/
    }

    /// Specifies input attachments for the pass.
    pub fn set_input_attachments(&mut self, input_attachments: &[&AttachmentRef])
    {
        self.graphics_task.input_attachments = input_attachments
            .iter()
            .map(|a| vk::AttachmentReference {
                attachment: a.id.index,
                layout: vk::ImageLayout::ColorAttachmentOptimal  // FIXME should not be changed?
            })
            .collect::<Vec<_>>();

        for i in input_attachments {
            /*if let Some(d) = i.dependency {
                self.frame
                    .add_dependency_access_flags(d, vk::ACCESS_INPUT_ATTACHMENT_READ_BIT);
            }*/

            // update usage bits of the resource
            self.frame.add_or_check_image_usage(i.id.img, vk::IMAGE_USAGE_INPUT_ATTACHMENT_BIT);
        }
    }

    /// Specifies the color attachments for the pass.
    pub fn set_color_attachments(&mut self, color_attachments: &[&AttachmentRef]) {

        self.graphics_task.color_attachments = color_attachments
            .iter()
            .map(|a| vk::AttachmentReference {
                attachment: a.id.index,
                layout: vk::ImageLayout::ColorAttachmentOptimal
            })
            .collect::<Vec<_>>();

        // update access bits of the dependency
        for c in color_attachments {
            /*if let Some(dependency) = c.dependency {
                let load_op = self
                    .graphics_task
                    .get_attachment_desc(c.vk_ref.attachment)
                    .load_op;

                let access = if load_op == vk::AttachmentLoadOp::Load {
                    vk::ACCESS_COLOR_ATTACHMENT_READ_BIT | vk::ACCESS_COLOR_ATTACHMENT_WRITE_BIT
                } else {
                    vk::ACCESS_COLOR_ATTACHMENT_WRITE_BIT
                };

                self.frame.add_dependency_access_flags(dependency, access);
            }*/

            self.frame
                .add_or_check_image_usage(c.id.img, vk::IMAGE_USAGE_COLOR_ATTACHMENT_BIT);
        }

    }

    //----------------------------------------------------------------------------------------------
    // ATTACHMENT LOAD/STORE/CREATE

    /// Imports a resource to be used as an attachment in the subpass.
    pub fn load_attachment(
        &mut self,
        img: &ImageRef,
        load_op: vk::AttachmentLoadOp,
    ) -> AttachmentRef
    {
        let attachment_index = self
            .frame
            .add_renderpass_attachment(
                self.graphics_task.renderpass,
                img.id,
                vk::AttachmentDescription {
                    flags: vk::AttachmentDescriptionFlags::empty(),
                    format: self.frame.get_image_format(img.id),
                    samples: vk::SAMPLE_COUNT_1_BIT,    // FIXME
                    load_op,
                    store_op: vk::AttachmentStoreOp::DontCare,
                    stencil_load_op: load_op,
                    stencil_store_op: vk::AttachmentStoreOp::DontCare,
                    initial_layout: vk::ImageLayout::Undefined,
                    final_layout: vk::ImageLayout::Undefined,
                },
            );

        // create reference
        AttachmentRef {
            task: self.task,
            id: AttachmentId {
                renderpass: self.graphics_task.renderpass,
                index: attachment_index,
                img: img.id
            },
            read: Cell::new(false),
            written: Cell::new(false),
            src_stage_mask: vk::PIPELINE_STAGE_TOP_OF_PIPE_BIT, // no need to sync, just created it
            latency: 0,
        }
    }

    /// TODO DOCUMENT
    pub fn store_attachment(&mut self,
                            attachment_ref: AttachmentRef,
                            store_op: vk::AttachmentStoreOp) -> ImageRef
    {
        {
            let (img, desc) = self.frame.get_renderpass_attachment_mut(
                self.graphics_task.renderpass, attachment_ref.id.index);
            desc.stencil_store_op = store_op;
        }

        ImageRef {
            id: attachment_ref.id.img,
            task: self.task,
            src_stage_mask: vk::PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT,
            read: Cell::new(false),
            written: Cell::new(false),
            latency: 0,     // FIXME better estimate
        }
    }

    /// Creates a new image that will be used as an attachment by the task.
    pub fn create_attachment(
        &mut self,
        name: impl Into<String>,
        (width, height): (u32, u32),
        format: vk::Format,
        samples: vk::SampleCountFlags,
        load_op: vk::AttachmentLoadOp, // Clear or DontCare, basically
    ) -> AttachmentRef {
        // declare image resource
        let desc = ImageDesc {
            flags: vk::ImageCreateFlags::default(),
            image_type: vk::ImageType::Type2d,
            format,
            extent: vk::Extent3D {
                width,
                height,
                depth: 1,
            },
            mip_levels: 1, // FIXME ?
            array_layers: 1,
            samples,
            tiling: vk::ImageTiling::Optimal,
            usage: vk::ImageUsageFlags::empty(), // added on use
        };
        let img = self.frame.create_image(name, desc);

        // declare attachment
        let attachment_index = self
            .frame
            .add_renderpass_attachment(
                self.graphics_task.renderpass,
                img,
                vk::AttachmentDescription {
                    flags: vk::AttachmentDescriptionFlags::empty(),
                    format,
                    samples,
                    load_op,
                    store_op: vk::AttachmentStoreOp::DontCare,
                    stencil_load_op: load_op,
                    stencil_store_op: vk::AttachmentStoreOp::DontCare,
                    initial_layout: vk::ImageLayout::Undefined,
                    final_layout: vk::ImageLayout::Undefined,
                },
            );

        // create reference
        let new_ref = AttachmentRef {
            task: self.task,
            id: AttachmentId {
                renderpass: self.graphics_task.renderpass,
                index: attachment_index,
                img
            },
            read: Cell::new(false),
            written: Cell::new(false),
            src_stage_mask: vk::PIPELINE_STAGE_TOP_OF_PIPE_BIT, // no need to sync, just created it
            latency: 0,
        };

        new_ref
    }

    pub(super) fn finish(mut self) -> TaskId {
        self.frame.graph.node_weight_mut(self.task).unwrap().details =
            TaskDetails::Graphics(self.graphics_task);
        self.task
    }
}
