use super::*;

//--------------------------------------------------------------------------------------------------

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
    attachments: Vec<ImageId>,
    attachments_desc: Vec<vk::AttachmentDescription>,
    pass_color_attachments: Vec<vk::AttachmentReference>,
    pass_input_attachments: Vec<vk::AttachmentReference>,
    pass_depth_attachment: Option<vk::AttachmentReference>,
    shader_images: Vec<ImageId>,
}

#[derive(Clone, Debug)]
pub struct AttachmentReference {
    vk_ref: vk::AttachmentReference,
    dependency: Option<DependencyId>,
}

impl GraphicsTask {
    pub(crate) fn new() -> GraphicsTask {
        GraphicsTask {
            attachments: Vec::new(),
            attachments_desc: Vec::new(),
            shader_images: Vec::new(),
            pass_color_attachments: Vec::new(),
            pass_input_attachments: Vec::new(),
            pass_depth_attachment: None,
        }
    }

    fn is_used_as_shader_image(&self, img: ImageId) -> Result<(), ()> {
        if self.shader_images.contains(&img) {
            // already used as shader-accessible resource
            Err(())
        } else {
            Ok(())
        }
    }

    fn add_attachment(&mut self, img: ImageId, desc: vk::AttachmentDescription) -> Result<u32, ()> {
        self.is_used_as_shader_image(img)?;
        self.attachments.push(img);
        self.attachments_desc.push(desc);
        Ok((self.attachments.len() - 1) as u32)
    }

    fn get_attachment_image_id(&self, index: u32) -> ImageId {
        self.attachments[index as usize]
    }

    fn get_attachment_desc(&self, index: u32) -> &vk::AttachmentDescription {
        &self.attachments_desc[index as usize]
    }
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
    ) -> GraphicsTaskBuilder<'frame, 'ctx> {
        // create a dummy node in the graph that we will fill up later.
        // this avoids looking into the graph everytime we modify something,
        // and still allows us to create dependencies in the graph
        let task = frame.create_task_on_queue(name, 0, TaskDetails::Other);
        GraphicsTaskBuilder {
            frame,
            task,
            graphics_task: GraphicsTask::new(),
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
                access_bits: vk::ACCESS_SHADER_READ_BIT,
                src_stage_mask: img.src_stage_mask,
                dst_stage_mask: vk::PIPELINE_STAGE_VERTEX_SHADER_BIT,
                resource: img.id.into(),
                latency: img.latency,
            },
        );

        self.graphics_task.shader_images.push(img.id);
    }

    /// Specifies a depth attachment.
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
    }

    /// Specifies input attachments for the pass.
    pub fn set_input_attachments(&mut self, input_attachments: &[AttachmentReference]) {
        self.graphics_task.pass_input_attachments = input_attachments
            .iter()
            .map(|a| a.vk_ref.clone())
            .collect::<Vec<_>>();

        for i in input_attachments {
            if let Some(d) = i.dependency {
                self.frame
                    .add_dependency_access_flags(d, vk::ACCESS_INPUT_ATTACHMENT_READ_BIT);
            }

            let img = self
                .graphics_task
                .get_attachment_image_id(i.vk_ref.attachment);
            // update usage bits of the resource
            self.frame
                .add_or_check_image_usage(img, vk::IMAGE_USAGE_INPUT_ATTACHMENT_BIT);
        }
    }

    /// Specifies the color attachments for the pass.
    pub fn set_color_attachments(&mut self, color_attachments: &[AttachmentReference]) {
        self.graphics_task.pass_color_attachments = color_attachments
            .iter()
            .map(|a| a.vk_ref.clone())
            .collect::<Vec<_>>();

        // update access bits of the dependency
        for c in color_attachments {
            if let Some(dependency) = c.dependency {
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
            }
            let img = self
                .graphics_task
                .get_attachment_image_id(c.vk_ref.attachment);
            self.frame
                .add_or_check_image_usage(img, vk::IMAGE_USAGE_COLOR_ATTACHMENT_BIT);
        }
    }

    /// Adds the specified image as a color attachment dependency on the task.
    /// Returns the new version of the resource.
    pub fn attachment(
        &mut self,
        img: &ImageRef,
        load_store_ops: &AttachmentLoadStore,
    ) -> (ImageRef, AttachmentReference) {
        img.set_write().expect("R/W conflict");

        self.frame
            .add_or_check_image_usage(img.id, vk::IMAGE_USAGE_COLOR_ATTACHMENT_BIT);

        let dependency = self.frame.add_dependency(
            img.task,
            self.task,
            Dependency {
                access_bits: vk::AccessFlags::empty(), // added later
                src_stage_mask: img.src_stage_mask,
                dst_stage_mask: vk::PIPELINE_STAGE_ALL_GRAPHICS_BIT, // FIXME
                latency: img.latency,
                resource: img.id.into(),
            },
        );

        let attachment_index = self
            .graphics_task
            .add_attachment(
                img.id,
                vk::AttachmentDescription {
                    flags: vk::AttachmentDescriptionFlags::empty(),
                    format: self.frame.get_image_format(img.id),
                    samples: vk::SAMPLE_COUNT_1_BIT, // FIXME blah blah blah
                    load_op: load_store_ops.load_op,
                    store_op: load_store_ops.store_op,
                    stencil_load_op: load_store_ops.stencil_load_op,
                    stencil_store_op: load_store_ops.stencil_store_op,
                    initial_layout: vk::ImageLayout::Undefined,
                    final_layout: vk::ImageLayout::Undefined,
                },
            ).expect("could not add attachment");

        let att = AttachmentReference {
            vk_ref: vk::AttachmentReference {
                attachment: attachment_index,
                layout: vk::ImageLayout::General,
            },
            dependency: Some(dependency),
        };

        let new_ref = ImageRef {
            id: img.id,
            src_stage_mask: vk::PIPELINE_STAGE_ALL_GRAPHICS_BIT, // FIXME not sure, maybe PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT is sufficient?
            task: self.task,
            read: Cell::new(false),
            written: Cell::new(false),
            latency: 1, // FIXME better estimate
        };

        (new_ref, att)
    }

    /// Creates a new image that will be used as a color attachment by the task.
    pub fn create_attachment(
        &mut self,
        (width, height): (u32, u32),
        format: vk::Format,
        load_store_ops: &AttachmentLoadStore,
    ) -> (ImageRef, AttachmentReference) {
        let img = self.frame.create_image_2d((width, height), format);

        let attachment_index = self
            .graphics_task
            .add_attachment(
                img,
                vk::AttachmentDescription {
                    flags: vk::AttachmentDescriptionFlags::empty(),
                    format: self.frame.get_image_format(img),
                    samples: vk::SAMPLE_COUNT_1_BIT, // FIXME blah blah blah
                    load_op: load_store_ops.load_op,
                    store_op: load_store_ops.store_op,
                    stencil_load_op: load_store_ops.stencil_load_op,
                    stencil_store_op: load_store_ops.stencil_store_op,
                    initial_layout: vk::ImageLayout::Undefined,
                    final_layout: vk::ImageLayout::Undefined,
                },
            ).expect("could not add attachment");

        let att = AttachmentReference {
            vk_ref: vk::AttachmentReference {
                attachment: attachment_index,
                layout: vk::ImageLayout::General,
            },
            dependency: None,
        };

        let new_ref = ImageRef {
            task: self.task,
            id: img,
            read: Cell::new(false),
            written: Cell::new(false),
            src_stage_mask: vk::PIPELINE_STAGE_TOP_OF_PIPE_BIT, // no need to sync, just created it
            latency: 0,
        };

        (new_ref, att)
    }

    pub(super) fn finish(mut self) -> TaskId {
        self.frame.graph.node_weight_mut(self.task).unwrap().details =
            TaskDetails::Graphics(self.graphics_task);
        self.task
    }
}
