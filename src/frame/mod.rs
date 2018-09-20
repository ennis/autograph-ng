use std::cell::{Cell, Ref, RefCell, RefMut};
use std::fs::File;
use std::io::{stdout, Write};
use std::ptr;

use ash::vk;
use downcast_rs::Downcast;
use petgraph::{graph::NodeIndex, visit::EdgeRef, Directed, Direction, Graph};

use context::Context;
use resource::*;

mod graphviz;
mod sched;

/// Identifies an image in the frame resource table.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ImageId(pub(crate) u32);

/// Identifies a buffer in the frame resource table.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct BufferId(pub(crate) u32);

/// Represents an operation in the frame graph.
#[derive(Debug)]
pub(crate) struct Task {
    /// Task name.
    pub(crate) name: String,
    /// On which queue this task is going to execute.
    /// If `None`, the task does not care.
    pub(crate) queue: Option<vk::Queue>,
    /*/// List of semaphores to wait for before executing the task.
    pub(crate) wait_semaphores: Vec<vk::Semaphore>,
    /// List of semaphores to signal after executing the task.
    pub(crate) signal_semaphores: Vec<vk::Semaphore>*/
}

impl Task {
    fn new<S: Into<String>>(name: S, queue: Option<vk::Queue>) -> Task {
        Task {
            name: name.into(),
            queue,
            /*wait_semaphores: Vec::new(),
            signal_semaphores: Vec::new()*/
        }
    }
}

/// Represents a dependency between tasks in the frame graph.
pub(crate) struct Dependency {
    /// How this resource is accessed by the dependent task.
    /// See vulkan docs for all possible flags.
    pub(crate) access_bits: vk::AccessFlags,
    /// What pipeline stage must have completed on the dependency.
    /// By default, this is BOTTOM_OF_PIPE.
    pub(crate) src_stage_mask: vk::PipelineStageFlags,
    /// What pipeline stage of this task (destination) is waiting on the dependency.
    /// By default, this is TOP_OF_PIPE.
    pub(crate) dst_stage_mask: vk::PipelineStageFlags,
    /// Details of the dependency specific to the usage and the type of resource.
    pub(crate) details: DependencyDetails,
}

impl Dependency {
    pub(crate) fn get_image_id(&self) -> Option<ImageId> {
        match self.details {
            DependencyDetails::Image { id, .. } => Some(id),
            _ => None,
        }
    }

    pub(crate) fn get_buffer_id(&self) -> Option<BufferId> {
        match self.details {
            DependencyDetails::Buffer { id, .. } => Some(id),
            _ => None,
        }
    }
}

pub(crate) struct AttachmentDependencyDetails {
    /// Index of the attachment.
    index: u32,
    /// Attachment description. note that some of the properties inside are filled
    /// During the scheduling passes.
    description: vk::AttachmentDescription,
}

/// Details of a dependency that is specific to the usage of the resource, and its
/// type.
pub(crate) enum DependencyDetails {
    /// Image dependency: either a sampled image or a storage image.
    /// This produces the image barrier.
    Image {
        /// The resource depended on.
        id: ImageId,
        /// The layout expected by the target.
        new_layout: vk::ImageLayout,
        /// Required usage bits.
        usage: vk::ImageUsageFlags,
        /// If the image is going to be used as an attachment.
        attachment: Option<AttachmentDependencyDetails>,
    },
    /// Details specific to buffer data.
    Buffer {
        /// The resource depended on.
        id: BufferId,
        /// Required usage bits.
        usage: vk::BufferUsageFlags,
    },
    /// Represents a sequencing constraint between tasks.
    /// Not associated to a particular resource.
    Sequence,
}

pub(crate) type TaskId = NodeIndex<u32>;
/// The frame graph type.
type FrameGraph = Graph<Task, Dependency, Directed, u32>;

//--------------------------------------------------------------------------------------------------

/// Represents one output of a task. This is used as part of the API to build dependencies between nodes.
pub struct TaskOutputRef<T> {
    /// ID of the resource in the frame resource table.
    id: T,
    /// Originating task.
    task: TaskId,
    /// What pipeline stage must have completed on the dependency.
    src_stage_mask: vk::PipelineStageFlags,
    /// Whether this resource has already been set as a read dependency.
    /// Prevents all writes.
    read: Cell<bool>,
    /// Whether this resource has already been set as a write dependency.
    /// Prevents all subsequent reads and writes.
    written: Cell<bool>,
}

impl<T> TaskOutputRef<T> {
    pub(crate) fn set_write(&self) {
        // TODO display more info about the conflict
        assert!(
            !(self.read.get() || self.written.get()),
            "read/write conflict"
        );
        self.written.set(true);
    }

    pub(crate) fn set_read(&self) {
        // TODO display more info about the conflict
        assert!(!self.written.get(), "read/write conflict");
        self.read.set(true);
    }
}

pub type ImageRef = TaskOutputRef<ImageId>;
pub type BufferRef = TaskOutputRef<BufferId>;

//--------------------------------------------------------------------------------------------------

pub(crate) struct ImageDesc {
    pub(crate) create_info: vk::ImageCreateInfo,
}

pub(crate) struct BufferDesc {
    pub(crate) create_info: vk::BufferCreateInfo,
}

/// A resource (image or buffer) used in a frame.
pub enum FrameResource<'imp, T: Resource, D> {
    Imported {
        resource: &'imp T,
    },
    Transient {
        name: String,
        description: D,
        resource: Option<T>,
    },
}

impl<'imp, T: Resource, D> FrameResource<'imp, T, D> {
    pub(crate) fn name(&self) -> &str {
        match self {
            FrameResource::Imported { resource } => resource.name(),
            FrameResource::Transient { ref name, .. } => name,
        }
    }

    pub(crate) fn is_imported(&self) -> bool {
        match self {
            FrameResource::Imported { .. } => true,
            _ => false,
        }
    }

    pub fn new_transient(name: String, description: D) -> FrameResource<'imp, T, D> {
        FrameResource::Transient {
            name,
            description,
            resource: None,
        }
    }

    pub fn new_imported(resource: &'imp T) -> FrameResource<'imp, T, D> {
        FrameResource::Imported { resource }
    }
}

type ImageFrameResource<'imp> = FrameResource<'imp, Image, ImageDesc>;
type BufferFrameResource<'imp> = FrameResource<'imp, Buffer, BufferDesc>;

impl<'imp> ImageFrameResource<'imp> {
    pub fn get_create_info(&self) -> &vk::ImageCreateInfo {
        match self {
            FrameResource::Imported { resource } => resource.create_info(),
            FrameResource::Transient {
                ref description, ..
            } => &description.create_info,
        }
    }
}

impl<'imp> BufferFrameResource<'imp> {
    pub fn get_create_info(&self) -> &vk::BufferCreateInfo {
        match self {
            FrameResource::Imported { resource } => resource.create_info(),
            FrameResource::Transient {
                ref description, ..
            } => &description.create_info,
        }
    }
}

//--------------------------------------------------------------------------------------------------

/// A frame: manages transient resources within a frame.
pub struct Frame<'ctx> {
    pub(crate) context: &'ctx mut Context,
    /// The DAG of tasks.
    pub(crate) graph: FrameGraph,
    /// Table of images used in this frame.
    pub(crate) images: Vec<ImageFrameResource<'ctx>>,
    /// Table of buffers used in this frame.
    pub(crate) buffers: Vec<BufferFrameResource<'ctx>>,
}

impl<'ctx> Frame<'ctx> {
    /// Creates a new frame.
    fn new(context: &'ctx mut Context) -> Frame<'ctx> {
        let mut graph = FrameGraph::new();
        let mut f = Frame {
            graph,
            context,
            images: Vec::new(),
            buffers: Vec::new(),
        };
        f
    }

    /// Creates a present task.
    /// The input must be an image of the swapchain.
    pub fn present(&mut self, img: &ImageRef) -> TaskId {
        /*let swapchain_index = {
            let img_entry = &self.images[img.id.0 as usize];
            match img_entry {
                FrameResource::Imported { resource } => {
                    if let Some(swapchain_index) = resource.swapchain_index {
                        swapchain_index
                    } else {
                        panic!("present target must be a swapchain image")
                    }
                }
                _ => panic!("present target must be an imported swapchain image"),
            }
        };*/
        let queue = self.context.present_queue;
        let task = self.create_task_on_queue("present", queue);
        // add a dependency on the image
        self.image_present_dependency(task, img);
        task
    }

    /// Creates a new task.
    /// Returns the ID to the newly created task.
    pub fn create_task<S: Into<String>>(&mut self, name: S) -> TaskId {
        /// Don't care about the queue.
        self.graph.add_node(Task::new(name, None))
    }

    /// Creates a new task that will execute on the specified queue.
    /// Returns the ID to the newly created task.
    pub(crate) fn create_task_on_queue<S: Into<String>>(
        &mut self,
        name: S,
        queue: vk::Queue,
    ) -> TaskId {
        /// Don't care about the queue.
        self.graph.add_node(Task::new(name, Some(queue)))
    }

    /// Updates the data contained in a texture. This creates a task in the graph.
    /// This does not synchronize: the data to be modified is first uploaded into a
    /// staging buffer first.
    fn update_image(&mut self, img: &ImageRef, data: ()) -> ImageRef {
        unimplemented!()
    }

    /// Returns information about a texture (Transient or Persistent)
    pub fn get_image_create_info(&self, img: &ImageRef) -> &vk::ImageCreateInfo {
        &self.images[img.id.0 as usize].get_create_info()
    }

    /// Returns information about a buffer (Transient or Persistent)
    pub fn get_buffer_create_info(&self, buf: &BufferRef) -> &vk::BufferCreateInfo {
        &self.buffers[buf.id.0 as usize].get_create_info()
    }

    /// Adds a sequencing constraint between two nodes.
    /// A sequencing constraint does not involve any resource.
    pub fn sequence_dependency(&mut self, src: TaskId, dst: TaskId) {
        self.graph.add_edge(
            src,
            dst,
            Dependency {
                access_bits: vk::AccessFlags::empty(),              // ignored
                src_stage_mask: vk::PIPELINE_STAGE_TOP_OF_PIPE_BIT, // ignored
                dst_stage_mask: vk::PIPELINE_STAGE_BOTTOM_OF_PIPE_BIT,
                details: DependencyDetails::Sequence,
            },
        );
    }

    /// Creates a task that has a dependency on all the specified tasks.
    fn make_sequence_task(&mut self, name: impl Into<String>, tasks: &[TaskId]) -> TaskId {
        // create the sync task
        let seq_task = self.create_task(name);
        // add a sequence dep to all of those
        for t in tasks.iter() {
            self.sequence_dependency(*t, seq_task);
        }
        seq_task
    }

    /// Waits for all reads to the specified resource to finish,
    /// and returns a virgin handle (no pending reads or writes)
    /// for reading and writing to this resource.
    pub fn sequence_image(&mut self, img: &ImageRef) -> ImageRef {
        // search for all tasks that read from img
        let tasks = self.collect_last_uses_of_image(img.id);
        let seq_task = self.make_sequence_task("sync", &tasks);
        // now we can return a virgin handle to the resource
        ImageRef {
            id: img.id,
            src_stage_mask: vk::PIPELINE_STAGE_BOTTOM_OF_PIPE_BIT, // FIXME we don't really know, but we can assume that it's one of the read stages
            task: seq_task,
            written: Cell::new(false),
            read: Cell::new(false),
        }
    }

    /// Adds a generic read dependency between two tasks.
    pub fn image_present_dependency(&mut self, task: TaskId, img: &ImageRef) {
        img.set_read();
        self.graph.add_edge(
            img.task,
            task,
            Dependency {
                access_bits: vk::AccessFlags::empty(), // ignored by present command
                src_stage_mask: vk::PIPELINE_STAGE_TOP_OF_PIPE_BIT, // ignored by present command
                dst_stage_mask: vk::PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT,
                details: DependencyDetails::Image {
                    id: img.id,
                    new_layout: vk::ImageLayout::PresentSrcKhr, // transition to presentation source layout
                    usage: vk::ImageUsageFlags::empty(),        // ignored
                    attachment: None,
                },
            },
        );
    }

    pub fn image_sample_dependency(&mut self, task: TaskId, img: &ImageRef) {
        /*// FIXME limitation to avoid toposort
        assert!(
            img.task < task,
            "Task cannot depend on the output a later task"
        );*/
        // increase read count
        img.set_read();
        // fetch info about the resource
        let image_info = self.get_image_create_info(img).clone();
        // don't know the layout yet
        // let old_layout = source.layout.expect("layout should be set for image resource reference");
        // Sampling is a shader read operation
        let access = vk::ACCESS_SHADER_READ_BIT;
        let src_stage_mask = img.src_stage_mask;
        // TODO: optimal dst_stage_mask knowing the pipeline associated to the task.
        // technically, the earliest point in the pipeline where we can sample a texture is the vertex
        // shader stage. But it could be accessed later, potentially allowing for better pipelining.
        let dst_stage_mask = vk::PIPELINE_STAGE_VERTEX_SHADER_BIT;
        // insert dependency into the graph
        self.graph.add_edge(
            img.task,
            task,
            Dependency {
                access_bits: access,
                src_stage_mask,
                dst_stage_mask,
                details: DependencyDetails::Image {
                    id: img.id,
                    // transfer to layout suited to shader access
                    new_layout: vk::ImageLayout::ShaderReadOnlyOptimal,
                    usage: vk::IMAGE_USAGE_SAMPLED_BIT,
                    attachment: None,
                },
            },
        );
    }

    /// Color attachment.
    /// TODO: maybe additional parameters, such as clear color?
    pub fn color_attachment_dependency(
        &mut self,
        task: TaskId,
        attachment_index: u32,
        img: &ImageRef,
    ) -> ImageRef {
        /*// FIXME limitation to avoid toposort
        assert!(
            img.task < task,
            "Task cannot depend on the output a later task"
        );*/
        // ensure exclusive access to resource.
        img.set_write();
        let image_info = self.get_image_create_info(img).clone();
        // don't know the layout
        // let old_layout = source.layout.expect("layout should be set for image resource reference");
        // TODO read access might not be necessary if not blending
        // TODO infer from shader pipeline?
        let access = vk::ACCESS_COLOR_ATTACHMENT_READ_BIT | vk::ACCESS_COLOR_ATTACHMENT_WRITE_BIT;
        let src_stage_mask = img.src_stage_mask;
        // FIXME not really sure what to put here
        let dst_stage_mask = vk::PIPELINE_STAGE_ALL_GRAPHICS_BIT;
        // build the attachment description for the input
        // will be used during the renderpass scheduling stage
        let attachment_desc = vk::AttachmentDescription {
            flags: vk::AttachmentDescriptionFlags::default(), // FIXME is aliasing possible?
            initial_layout: vk::ImageLayout::Undefined,       // determined later
            final_layout: vk::ImageLayout::ColorAttachmentOptimal, // FIXME final layout after the pass, not sure what to put here.
            format: image_info.format,
            load_op: vk::AttachmentLoadOp::Load, // FIXME eeeeh same as needing read access?
            store_op: vk::AttachmentStoreOp::Store, // FIXME pretty sure we need to store things anyway although might not be necessary if we don't plan to read the data in another pass?
            stencil_load_op: vk::AttachmentLoadOp::DontCare,
            stencil_store_op: vk::AttachmentStoreOp::DontCare,
            samples: image_info.samples,
        };
        // insert dependency into the graph
        self.graph.add_edge(
            img.task,
            task,
            Dependency {
                access_bits: access,
                src_stage_mask,
                dst_stage_mask,
                details: DependencyDetails::Image {
                    id: img.id,
                    usage: vk::IMAGE_USAGE_COLOR_ATTACHMENT_BIT,
                    new_layout: vk::ImageLayout::ColorAttachmentOptimal,
                    attachment: Some(AttachmentDependencyDetails {
                        index: attachment_index,
                        description: attachment_desc,
                    }),
                },
            },
        );
        // create the new resource descriptor: new version of this resource, originating
        // from this task
        ImageRef {
            id: img.id,
            // FIXME not sure, maybe PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT is sufficient?
            src_stage_mask: vk::PIPELINE_STAGE_ALL_GRAPHICS_BIT,
            task,
            read: Cell::new(false),
            written: Cell::new(false),
        }
    }

    /// Creates a transient 2D image associated with the specified task.
    /// The initial layout of the image is inferred from its usage in depending tasks.
    pub fn create_image_2d(&mut self, (width, height): (u32, u32), format: vk::Format) -> ImageRef {
        // create a task associated with this creation op
        let task = self.create_task("dummy");
        let desc = ImageDesc {
            create_info: vk::ImageCreateInfo {
                s_type: vk::StructureType::ImageCreateInfo,
                p_next: ptr::null(),
                flags: vk::ImageCreateFlags::default(),
                image_type: vk::ImageType::Type2d,
                format,
                extent: vk::Extent3D {
                    width,
                    height,
                    depth: 1,
                },
                mip_levels: 1,                    // FIXME
                array_layers: 1,                  // FIXME
                samples: vk::SAMPLE_COUNT_1_BIT,  // FIXME
                tiling: vk::ImageTiling::Optimal, // FIXME
                // inferred from the graph
                usage: vk::ImageUsageFlags::default(),
                sharing_mode: vk::SharingMode::Exclusive, // FIXME
                queue_family_index_count: 0,              // FIXME
                p_queue_family_indices: ptr::null(),
                initial_layout: vk::ImageLayout::Undefined, // inferred
            },
        };

        // get an index to generate a name for this resource.
        // It's not crucial that we get a unique one,
        // as the name of resources are here for informative purposes only.
        let naming_index = self.images.len();
        let new_id = self.add_image_resource(format!("IMG_{:04}", naming_index), desc);

        ImageRef {
            task,
            id: new_id,
            read: Cell::new(false),
            written: Cell::new(false),
            src_stage_mask: vk::PIPELINE_STAGE_TOP_OF_PIPE_BIT, // no need to sync, just created it
        }
    }

    /// Adds a transient buffer resource.
    pub(crate) fn add_buffer_resource(&mut self, name: String, desc: BufferDesc) -> BufferId {
        self.buffers
            .push(BufferFrameResource::new_transient(name, desc));
        BufferId((self.buffers.len() - 1) as u32)
    }

    /// Adds a transient image resource.
    pub(crate) fn add_image_resource(&mut self, name: String, desc: ImageDesc) -> ImageId {
        self.images
            .push(ImageFrameResource::new_transient(name, desc));
        ImageId((self.images.len() - 1) as u32)
    }

    /// Imports a persistent image for use in the frame graph.
    pub fn import_image(&mut self, img: &'ctx Image) -> ImageRef {
        let task = self.create_task("import");
        self.images.push(ImageFrameResource::new_imported(img));
        let id = ImageId((self.images.len() - 1) as u32);
        ImageRef {
            id,
            read: Cell::new(false),
            written: Cell::new(false),
            task,
            src_stage_mask: vk::PIPELINE_STAGE_BOTTOM_OF_PIPE_BIT, // FIXME too conservative?
        }
    }

    /// Collects all tasks using this resource but that do not produce another version of it.
    fn collect_last_uses_of_image(&self, img: ImageId) -> Vec<TaskId> {
        let uses = self
            .graph
            .node_indices()
            .filter(|n| {
                // is the resource used in an incoming dependency?
                let incoming = self
                    .graph
                    .edges_directed(*n, Direction::Incoming)
                    .any(|e| e.weight().get_image_id() == Some(img));
                // does not appear in any outgoing dependency
                let outgoing = self
                    .graph
                    .edges_directed(*n, Direction::Outgoing)
                    .any(|e| e.weight().get_image_id() == Some(img));

                incoming && !outgoing
            }).collect::<Vec<_>>();

        uses
    }

    /// Collects all tasks using this resource but that do not produce another version of it.
    fn collect_last_uses_of_buffer(&self, buf: BufferId) -> Vec<TaskId> {
        let uses = self
            .graph
            .node_indices()
            .filter(|n| {
                // is the resource used in an incoming dependency?
                let incoming = self
                    .graph
                    .edges_directed(*n, Direction::Incoming)
                    .any(|e| e.weight().get_buffer_id() == Some(buf));
                // does not appear in any outgoing dependency
                let outgoing = self
                    .graph
                    .edges_directed(*n, Direction::Outgoing)
                    .any(|e| e.weight().get_buffer_id() == Some(buf));

                incoming && !outgoing
            }).collect::<Vec<_>>();

        uses
    }

    /// Inserts 'exit tasks' for all external resources imported into the graph.
    fn insert_exit_tasks(&mut self) {
        // find last uses of each external resource
        let tasks_to_create = self
            .images
            .iter()
            .enumerate()
            .filter(|(_, img)| img.is_imported())
            .map(|(i, img)| {
                let i = ImageId(i as u32);
                (i, self.collect_last_uses_of_image(i))
            }).collect::<Vec<_>>();

        // add tasks
        for t in tasks_to_create.iter() {
            // on which queue?
            self.make_sequence_task("exit", &t.1);
        }
    }

    fn dump<W: Write>(&self, w: &mut W) {
        // dump resources
        writeln!(w, "--- RESOURCES ---");
        for (i, r) in self.images.iter().enumerate() {
            let name = r.name();
            let create_info = r.get_create_info();
            writeln!(w, "Image {}(#{})", name, i);
            writeln!(w, "  imageType ........ {:?}", create_info.image_type);
            writeln!(w, "  width ............ {}", create_info.extent.width);
            writeln!(w, "  height ........... {}", create_info.extent.height);
            writeln!(w, "  depth ............ {}", create_info.extent.depth);
            writeln!(w, "  format ........... {:?}", create_info.format);
            writeln!(w, "  usage ............ {:?}", create_info.usage);
            writeln!(w);
        }
        for (i, r) in self.buffers.iter().enumerate() {
            let name = r.name();
            let create_info = r.get_create_info();
            writeln!(w, "Buffer {}(#{})", name, i);
            writeln!(w, "  size ............. {}", create_info.size);
            writeln!(w, "  usage ............ {:?}", create_info.usage);
            writeln!(w);
        }

        writeln!(w);

        // tasks
        writeln!(w, "--- TASKS ---");
        for n in self.graph.node_indices() {
            let t = self.graph.node_weight(n).unwrap();
            writeln!(w, "{} (#{})", t.name, n.index());
        }
        writeln!(w);

        // dependencies
        writeln!(w, "--- DEPS ---");
        for e in self.graph.edge_indices() {
            let (src, dest) = self.graph.edge_endpoints(e).unwrap();
            let src_task = self.graph.node_weight(src).unwrap();
            let dest_task = self.graph.node_weight(dest).unwrap();
            let d = self.graph.edge_weight(e).unwrap();

            match &d.details {
                &DependencyDetails::Image {
                    id,
                    new_layout,
                    usage,
                    ref attachment,
                } => {
                    if attachment.is_some() {
                        writeln!(
                            w,
                            "ATTACHMENT {}(#{}) -> {}(#{})",
                            src_task.name,
                            src.index(),
                            dest_task.name,
                            dest.index()
                        );
                    } else {
                        writeln!(
                            w,
                            "IMAGE ACCESS {}(#{}) -> {}(#{})",
                            src_task.name,
                            src.index(),
                            dest_task.name,
                            dest.index()
                        );
                    }
                    writeln!(w, "  resource ......... {:08X}", id.0);
                    writeln!(w, "  access ........... {:?}", d.access_bits);
                    writeln!(w, "  srcStageMask ..... {:?}", d.src_stage_mask);
                    writeln!(w, "  dstStageMask ..... {:?}", d.dst_stage_mask);
                    writeln!(w, "  newLayout ........ {:?}", new_layout);
                    if let Some(ref attachment) = attachment {
                        writeln!(w, "  index ............ {}", attachment.index);
                        writeln!(
                            w,
                            "  format ........... {:?}",
                            attachment.description.format
                        );
                        writeln!(
                            w,
                            "  loadOp ........... {:?}",
                            attachment.description.load_op
                        );
                        writeln!(
                            w,
                            "  storeOp .......... {:?}",
                            attachment.description.store_op
                        );
                        writeln!(
                            w,
                            "  initialLayout .... {:?}",
                            attachment.description.initial_layout
                        );
                        writeln!(
                            w,
                            "  finalLayout ...... {:?}",
                            attachment.description.final_layout
                        );
                    }
                }
                &DependencyDetails::Buffer { id, .. } => {
                    writeln!(
                        w,
                        "BUFFER ACCESS {}(#{}) -> {}(#{})",
                        src_task.name,
                        src.index(),
                        dest_task.name,
                        dest.index()
                    );
                    writeln!(w, "  resource ......... {:08X}", id.0);
                    writeln!(w, "  access ........... {:?}", d.access_bits);
                    writeln!(w, "  srcStageMask ..... {:?}", d.src_stage_mask);
                    writeln!(w, "  dstStageMask ..... {:?}", d.dst_stage_mask);
                }
                &DependencyDetails::Sequence => {
                    writeln!(
                        w,
                        "SEQUENCE {}(#{}) -> {}(#{})",
                        src_task.name,
                        src.index(),
                        dest_task.name,
                        dest.index()
                    );
                }
            }
            writeln!(w);
        }
    }

    pub fn submit(mut self) {
        // TODO
        self.insert_exit_tasks();
        self.dump(&mut stdout());
        let mut dot = File::create("graph.dot").unwrap();
        let ordering = self.schedule();
        self.dump_graphviz(&mut dot, Some(&ordering), false);
    }
}

//--------------------------------------------------------------------------------------------------
// Context

impl Context {
    /// Creates a frame.
    /// The frame handles the scheduling of all GPU operations, synchronization between
    /// command queues, and synchronization with the CPU.
    pub fn new_frame(&mut self) -> Frame {
        Frame::new(self)
    }
}
