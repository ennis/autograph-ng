use std::cell::{Cell, Ref, RefCell, RefMut};
use std::fs::File;
use std::io::{stdout, Write};
use std::mem;
use std::ptr;

use ash::vk;
use downcast_rs::Downcast;
use petgraph::{graph::NodeIndex, visit::EdgeRef, Directed, Direction, Graph};

pub use self::sched::ScheduleOptimizationProfile;
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

/// Type of task.
#[derive(Copy, Clone, Debug)]
pub enum TaskType {
    /// A graphics task: must execute in a render pass, and has attachments.
    Graphics,
    /// A compute operation.
    Compute,
    /// A transfer operation (memory copy).
    Transfer,
    /// A present operation to a surface.
    Present,
    /// A ray-tracing task.
    RayTracing,
    /// Misc
    Other,
}

/// Represents an operation in the frame graph.
#[derive(Debug)]
pub(crate) struct Task {
    /// Task name.
    pub(crate) name: String,
    /// Type of workload
    pub(crate) task_type: TaskType,
    /// On which queue this task is going to execute.
    /// If `None`, the task does not care.
    pub(crate) queue: u32,
    /// List of "self-dependencies" on resources created for this task.
    pub(crate) self_dependencies: Vec<Dependency>,
}

impl Task {
    fn new(name: impl Into<String>, task_type: TaskType, queue: u32) -> Task {
        Task {
            name: name.into(),
            queue,
            self_dependencies: Vec::new(),
            task_type,
        }
    }

    /* fn new_compute(name: impl Into<String>, queue: u32) -> Task {
        Task {
            name: name.into(),
            queue,
            self_dependencies: Vec::new()
        }
    }*/

    fn add_self_dependency(&mut self, dep: Dependency) {
        self.self_dependencies.push(dep);
    }
}

/// Represents a dependency between tasks in the frame graph.
#[derive(Debug)]
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
    /// Estimated latency of the dependency (time for the resource to be usable by target once source is submitted).
    /// 0 for dummy nodes.
    pub(crate) latency: u32,
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

/// DOCUMENT
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum AttachmentIndex {
    Color(u32),
    DepthStencil,
}

/// DOCUMENT
#[derive(Debug)]
pub(crate) struct AttachmentDependencyDetails {
    /// Index of the attachment.
    index: AttachmentIndex,
    /// DOCUMENT
    load_op: vk::AttachmentLoadOp,
    /// DOCUMENT
    store_op: vk::AttachmentStoreOp,
    /// DOCUMENT
    stencil_load_op: vk::AttachmentLoadOp,
    /// DOCUMENT
    stencil_store_op: vk::AttachmentStoreOp,
}

/// Details of a dependency that is specific to the usage of the resource, and its
/// type.
#[derive(Debug)]
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

/// Represents one output of a task.
/// This is used as part of the API to build dependencies between nodes.
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
    /// Estimated time for the resource to be ready (can vary between different usages).
    latency: u32,
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
    pub(crate) flags: vk::ImageCreateFlags,
    pub(crate) image_type: vk::ImageType,
    pub(crate) format: vk::Format,
    pub(crate) extent: vk::Extent3D,
    pub(crate) mip_levels: u32,
    pub(crate) array_layers: u32,
    pub(crate) samples: vk::SampleCountFlags,
    pub(crate) tiling: vk::ImageTiling,
    pub(crate) usage: vk::ImageUsageFlags, // inferred
                                           //pub(crate) sharing_mode: SharingMode,
                                           //pub(crate) queue_family_index_count: uint32_t,    // inferred
                                           //pub(crate) p_queue_family_indices: *const uint32_t,
                                           //pub(crate) initial_layout: ImageLayout,   // inferred
}

pub(crate) struct BufferDesc {
    pub(crate) flags: vk::BufferCreateFlags,
    pub(crate) size: vk::DeviceSize,
    pub(crate) usage: vk::BufferUsageFlags,
    //pub(crate) sharing_mode: vk::SharingMode,
    //pub(crate) queue_family_index_count: uint32_t,
    //pub(crate) p_queue_family_indices: *const uint32_t,
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
    pub fn dimensions(&self) -> (u32, u32, u32) {
        match self {
            FrameResource::Imported { resource } => resource.dimensions(),
            FrameResource::Transient {
                ref description, ..
            } => (
                description.extent.width,
                description.extent.height,
                description.extent.depth,
            ),
        }
    }

    pub fn format(&self) -> vk::Format {
        match self {
            FrameResource::Imported { resource } => resource.format(),
            FrameResource::Transient {
                ref description, ..
            } => description.format,
        }
    }
}

impl<'imp> BufferFrameResource<'imp> {
    pub fn size(&self) -> vk::DeviceSize {
        match self {
            FrameResource::Imported { resource } => resource.size(),
            FrameResource::Transient {
                ref description, ..
            } => description.size,
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

/// Task builder.
pub struct TaskBuilder<'frame, 'ctx: 'frame> {
    frame: &'frame mut Frame<'ctx>,
    task: TaskId,
}

impl<'frame, 'ctx: 'frame> TaskBuilder<'frame, 'ctx> {
    fn get_task_mut(&mut self) -> &mut Task {
        self.frame.graph.node_weight_mut(self.task).unwrap()
    }

    /// Adds the specified as an image sample dependency on the task.
    pub fn sample_image(&mut self, img: &ImageRef) {
        // increase read count
        img.set_read();
        // insert dependency into the graph
        self.frame.graph.add_edge(
            img.task,
            self.task,
            Dependency {
                access_bits: vk::ACCESS_SHADER_READ_BIT,
                src_stage_mask: img.src_stage_mask,
                dst_stage_mask: vk::PIPELINE_STAGE_VERTEX_SHADER_BIT,
                details: DependencyDetails::Image {
                    id: img.id,
                    // transfer to layout suited to shader access
                    new_layout: vk::ImageLayout::ShaderReadOnlyOptimal,
                    usage: vk::IMAGE_USAGE_SAMPLED_BIT,
                    attachment: None,
                },
                latency: img.latency,
            },
        );
    }

    /// Adds the specified as a color attachment dependency on the task.
    pub fn attachment(&mut self, index: AttachmentIndex, img: &mut ImageRef) {
        // ensure exclusive access to resource
        img.set_write();
        // insert dependency into the graph
        self.frame.graph.add_edge(
            img.task,
            self.task,
            Dependency {
                access_bits: vk::ACCESS_COLOR_ATTACHMENT_READ_BIT
                    | vk::ACCESS_COLOR_ATTACHMENT_WRITE_BIT,
                src_stage_mask: img.src_stage_mask,
                dst_stage_mask: vk::PIPELINE_STAGE_ALL_GRAPHICS_BIT, // FIXME not really sure what to put here
                details: DependencyDetails::Image {
                    id: img.id,
                    usage: vk::IMAGE_USAGE_COLOR_ATTACHMENT_BIT,
                    new_layout: vk::ImageLayout::ColorAttachmentOptimal,
                    attachment: Some(AttachmentDependencyDetails {
                        index,
                        load_op: vk::AttachmentLoadOp::Load, // FIXME eeeeh same as needing read access?
                        store_op: vk::AttachmentStoreOp::Store, // FIXME pretty sure we need to store things anyway although might not be necessary if we don't plan to read the data in another pass?
                        stencil_load_op: vk::AttachmentLoadOp::DontCare,
                        stencil_store_op: vk::AttachmentStoreOp::DontCare,
                    }),
                },
                latency: img.latency,
            },
        );

        // create the new resource descriptor: new version of this resource, originating
        // from this task
        *img = ImageRef {
            id: img.id,
            src_stage_mask: vk::PIPELINE_STAGE_ALL_GRAPHICS_BIT, // FIXME not sure, maybe PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT is sufficient?
            task: self.task,
            read: Cell::new(false),
            written: Cell::new(false),
            latency: 1, // FIXME better estimate
        };
    }

    /// Adds a generic read dependency on the specified image.
    pub fn generic_read(&mut self, img: &ImageRef) {
        img.set_read();
        self.frame.graph.add_edge(
            img.task,
            self.task,
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
                latency: 1, // FIXME ???
            },
        );
    }

    /// Adds a sequencing constraint between two nodes.
    /// A sequencing constraint does not involve any resource.
    pub fn sequence(&mut self, source: TaskId) {
        self.frame.graph.add_edge(
            source,
            self.task,
            Dependency {
                access_bits: vk::AccessFlags::empty(),              // ignored
                src_stage_mask: vk::PIPELINE_STAGE_TOP_OF_PIPE_BIT, // ignored
                dst_stage_mask: vk::PIPELINE_STAGE_BOTTOM_OF_PIPE_BIT,
                details: DependencyDetails::Sequence,
                latency: 0, // not sure...
            },
        );
    }

    /// Creates a new image that will be used as a color attachment by the task.
    pub fn create_attachment(
        &mut self,
        index: AttachmentIndex,
        (width, height): (u32, u32),
        format: vk::Format,
    ) -> ImageRef {
        let img = self.frame.create_image_2d((width, height), format);

        // insert self-dependency
        self.get_task_mut().add_self_dependency(Dependency {
            access_bits: vk::ACCESS_COLOR_ATTACHMENT_READ_BIT
                | vk::ACCESS_COLOR_ATTACHMENT_WRITE_BIT,
            src_stage_mask: vk::PIPELINE_STAGE_TOP_OF_PIPE_BIT, // FIXME ignored
            dst_stage_mask: vk::PIPELINE_STAGE_ALL_GRAPHICS_BIT, // FIXME not really sure what to put here
            details: DependencyDetails::Image {
                id: img,
                usage: vk::IMAGE_USAGE_COLOR_ATTACHMENT_BIT,
                new_layout: vk::ImageLayout::ColorAttachmentOptimal,
                attachment: Some(AttachmentDependencyDetails {
                    index,
                    load_op: vk::AttachmentLoadOp::Load, // FIXME eeeeh same as needing read access?
                    store_op: vk::AttachmentStoreOp::Store, // FIXME pretty sure we need to store things anyway although might not be necessary if we don't plan to read the data in another pass?
                    stencil_load_op: vk::AttachmentLoadOp::DontCare,
                    stencil_store_op: vk::AttachmentStoreOp::DontCare,
                }),
            },
            latency: 0,
        });

        ImageRef {
            task: self.task,
            id: img,
            read: Cell::new(false),
            written: Cell::new(false),
            src_stage_mask: vk::PIPELINE_STAGE_TOP_OF_PIPE_BIT, // no need to sync, just created it
            latency: 0,
        }
    }
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
        let queue = self.context.present_queue;
        self.create_task_on_queue("present", TaskType::Present, 1, |t| {
            t.generic_read(img);
        }).0
    }

    /// Creates a new task.
    /// Returns the ID to the newly created task.
    pub fn create_graphics_task<S, R, F>(&mut self, name: S, setup: F) -> (TaskId, R)
    where
        S: Into<String>,
        F: FnOnce(&mut TaskBuilder) -> R,
    {
        self.create_task_on_queue(name, TaskType::Graphics, 0, setup)
    }

    /// Creates a new task that will execute on the specified queue.
    /// Returns the ID to the newly created task.
    pub fn create_task_on_queue<S, R, F>(
        &mut self,
        name: S,
        task_type: TaskType,
        queue: u32,
        setup: F,
    ) -> (TaskId, R)
    where
        S: Into<String>,
        F: FnOnce(&mut TaskBuilder) -> R,
    {
        // Don't care about the queue.
        let task = self.graph.add_node(Task::new(name, task_type, queue));
        let mut task_builder = TaskBuilder { frame: self, task };
        (task, setup(&mut task_builder))
    }

    /// Updates the data contained in a texture. This creates a task in the graph.
    /// This does not synchronize: the data to be modified is first uploaded into a
    /// staging buffer first.
    fn update_image(&mut self, img: &ImageRef, data: ()) -> ImageRef {
        unimplemented!()
    }

    /// Gets the dimensions of the image (width, height, depth).
    pub fn get_image_dimensions(&self, img: &ImageRef) -> (u32, u32, u32) {
        self.images[img.id.0 as usize].dimensions()
    }

    /// Gets the dimensions of the image.
    pub fn get_image_format(&self, img: &ImageRef) -> vk::Format {
        self.images[img.id.0 as usize].format()
    }

    /// Creates a task that has a dependency on all the specified tasks.
    fn make_sequence_task(&mut self, name: impl Into<String>, tasks: &[TaskId]) -> TaskId {
        // create the sync task
        self.create_task_on_queue(name, TaskType::Other, 0, |t| {
            // add a sequence dep to all of those
            for task in tasks.iter() {
                t.sequence(*task);
            }
        }).0
    }

    /* /// Waits for all reads to the specified resource to finish,
    /// and returns a virgin handle (no pending reads or writes)
    /// for reading and writing to this resource.
    fn sequence_image(&mut self, img: &ImageRef) -> ImageRef {
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
            latency: 0,
        }
    }*/

    /// Creates a transient 2D image associated with the specified task.
    /// The initial layout of the image is inferred from its usage in depending tasks.
    fn create_image_2d(&mut self, (width, height): (u32, u32), format: vk::Format) -> ImageId {
        let desc = ImageDesc {
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
            usage: vk::ImageUsageFlags::default(), // inferred from the graph
                                              //sharing_mode: vk::SharingMode::Exclusive, // FIXME
                                              //queue_family_index_count: 0,              // FIXME
                                              //p_queue_family_indices: ptr::null(),
                                              //initial_layout: vk::ImageLayout::Undefined, // inferred
        };

        // get an index to generate a name for this resource.
        // It's not crucial that we get a unique one,
        // as the name of resources are here for informative purposes only.
        let naming_index = self.images.len();
        self.add_image_resource(format!("IMG_{:04}", naming_index), desc)
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
        let (task, ()) = self.create_task_on_queue("import", TaskType::Transfer, 0, |_| {}); // FIXME
        self.images.push(ImageFrameResource::new_imported(img));
        let id = ImageId((self.images.len() - 1) as u32);
        ImageRef {
            id,
            read: Cell::new(false),
            written: Cell::new(false),
            task,
            src_stage_mask: vk::PIPELINE_STAGE_BOTTOM_OF_PIPE_BIT, // FIXME too conservative?
            latency: 0,
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
            let (width, height, depth) = r.dimensions();
            let format = r.format();
            writeln!(w, "Image {}(#{})", name, i);
            //writeln!(w, "  imageType ........ {:?}", create_info.image_type);
            writeln!(w, "  width ............ {}", width);
            writeln!(w, "  height ........... {}", height);
            writeln!(w, "  depth ............ {}", depth);
            writeln!(w, "  format ........... {:?}", format);
            //writeln!(w, "  usage ............ {:?}", create_info.usage);
            writeln!(w);
        }
        for (i, r) in self.buffers.iter().enumerate() {
            let name = r.name();
            let size = r.size();
            writeln!(w, "Buffer {}(#{})", name, i);
            writeln!(w, "  size ............. {}", size);
            //writeln!(w, "  usage ............ {:?}", create_info.usage);
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
                        writeln!(w, "  index ............ {:?}", attachment.index);
                        writeln!(w, "  loadOp ........... {:?}", attachment.load_op);
                        writeln!(w, "  storeOp .......... {:?}", attachment.store_op);
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
        let ordering = self.schedule(ScheduleOptimizationProfile::MaximizeAliasing);
        let mut dot = File::create("graph.dot").unwrap();
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
