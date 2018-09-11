use std::io::{stdout, Write};
use std::cell::Cell;
use std::ptr;
use std::fs::File;

use ash::vk;
use petgraph::{Graph, Direction, Directed, graph::NodeIndex};
use downcast_rs::Downcast;
use std::cell::{RefCell, RefMut, Ref};

use resource::{Resource, BufferResource, ImageResource};
use context::Context;

/*
/// An untyped resource index, with one bit that indicates whether the resource is transient
#[derive(Copy,Clone,Debug,Eq,PartialEq,Ord,PartialOrd,Hash)]
pub struct ResourceIndex(u32);

impl ResourceIndex {
    pub fn new_transient(index: usize) -> ResourceIndex {
        assert!(index <= (1u32 << 31));
        ResourceIndex(index & (1u32 << 31))
    }

    pub fn is_transient(&self) -> bool {
        return (self.0 & (1u32 << 31)) != 0;
    }

    pub fn index(&self) -> usize {
        return (self.0 & !(1u32 << 31)) as usize;
    }
}*/

#[derive(Copy,Clone,Debug,Eq,PartialEq,Ord,PartialOrd,Hash)]
pub struct ImageId(pub(crate) u32);

#[derive(Copy,Clone,Debug,Eq,PartialEq,Ord,PartialOrd,Hash)]
pub struct BufferId(pub(crate) u32);

/*bitflags! {
    /// What is the dependency going to be used for?
    struct Flags: u32 {
        const SAMPLED_IMAGE = 0b0000_0001;
        const STORAGE_IMAGE = 0b0000_0010;
        const ATTACHMENT_INPUT = 0b0000_0100;
        const DEPTH_STENCIL_ATTACHMENT = 0b0000_1000;
        const COLOR_ATTACHMENT = 0b0001_0000;
        const UNIFORM_BUFFER = 0b0010_0000;
        const STORAGE_BUFFER = 0b0100_0000;
    }
}*/

/// Represents an operation in the frame graph.
#[derive(Debug)]
pub(crate) struct Task
{
    /// Task name.
    pub(crate) name: String,
    /*/// Resources that this node writes to.
    pub(crate) writes: Vec<ResourceId>,
    /// Resources that should be created for this node.
    pub(crate) creates: Vec<ResourceId>*/
}

impl Task
{
    fn new<S: Into<String>>(name: S) -> Task {
        Task {
            name: name.into(),
            //creates: Vec::new(),
            //writes: Vec::new(),
        }
    }
}

/// Represents a dependency between tasks in the frame graph.
pub(crate) struct Dependency
{
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
    pub(crate) details: DependencyDetails
}


/// Details of a dependency that is specific to the usage of the resource, and its
/// type.
pub(crate) enum DependencyDetails
{
    /// Image dependency: either a sampled image or a storage image.
    /// This produces the image barrier.
    Image {
        /// The resource depended on.
        id: ImageId,
        /// The layout expected by the target.
        new_layout: vk::ImageLayout,
    },
    /// Image used as an attachment.
    Attachment {
        /// The resource depended on.
        id: ImageId,
        /// Index of the attachment.
        index: u32,
        /// Attachment description. note that some of the properties inside are filled
        /// During the scheduling passes.
        description: vk::AttachmentDescription,
    },
    /// Details specific to buffer data.
    Buffer {
        /// The resource depended on.
        id: BufferId,
    }
}


pub(crate) type TaskId = NodeIndex<u32>;
type FrameGraph = Graph<Task, Dependency, Directed, u32>;

/// Represents one output of a task. This is used as part of the API to build dependencies between nodes.
pub struct TaskOutputRef<T>
{
    ///
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

impl<T> TaskOutputRef<T>
{
    pub(crate) fn set_read(&self) {
        // TODO display more info about the conflict
        assert!(!(self.read.get() || self.written.get()), "read/write conflict");
        self.written.set(true);
    }

    pub(crate) fn set_write(&self) {
        // TODO display more info about the conflict
        assert!(!self.written.get(), "read/write conflict");
        self.read.set(true);
    }
}

pub type ImageRef = TaskOutputRef<ImageId>;
pub type BufferRef = TaskOutputRef<BufferId>;

pub enum FrameResource<T: Resource>
{
    /// Persistent resource imported into the graph.
    Imported {
        id: usize,  // FIXME: T::Id
    },
    /// Transient resource: own the resource.
    Transient {
        resource: T,
    }
}

impl<T: Resource> FrameResource<T>
{
    pub(crate) fn name(&self) -> &str {
        match self {
            &FrameResource::Imported { .. } => { unimplemented!() },
            &FrameResource::Transient { ref resource } => { resource.name() }
        }
    }
}

type ImageFrameResource = FrameResource<ImageResource>;
type BufferFrameResource = FrameResource<BufferResource>;

impl ImageFrameResource
{
    pub fn get_create_info(&self) -> &vk::ImageCreateInfo {
        match self {
            &FrameResource::Imported { .. } => { unimplemented!() },
            &FrameResource::Transient { ref resource } => { &resource.create_info }
        }
    }
}

impl BufferFrameResource
{
    pub fn get_create_info(&self) -> &vk::BufferCreateInfo {
        match self {
            &FrameResource::Imported { .. } => { unimplemented!() },
            &FrameResource::Transient { ref resource } => { &resource.create_info }
        }
    }
}

/// A frame: manages transient resources within a frame.
pub struct Frame<'ctx> {
    pub(crate) context: &'ctx mut Context,
    /// The DAG of tasks.
    pub(crate) graph: FrameGraph,
    /// Table of images used in this frame.
    pub(crate) images: Vec<ImageFrameResource>,
    /// Table of buffers used in this frame.
    pub(crate) buffers: Vec<BufferFrameResource>,
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

    /// Creates a new task.
    /// Returns the ID to the newly created task.
    pub fn create_task<S: Into<String>>(&mut self, name: S) -> TaskId {
        self.graph.add_node(Task::new(name))
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

    pub fn image_sample_dependency(&mut self, task: TaskId, img: &ImageRef)
    {
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
        self.graph.add_edge(img.task, task, Dependency {
            access_bits: access,
            src_stage_mask,
            dst_stage_mask,
            details: DependencyDetails::Image {
                id: img.id,
                // transfer to layout suited to shader access
                new_layout: vk::ImageLayout::ShaderReadOnlyOptimal
            }
        });
    }

    /// Color attachment.
    /// TODO: maybe additional parameters, such as clear color?
    pub fn color_attachment_dependency(&mut self, task: TaskId, attachment_index: u32, img: &ImageRef) -> ImageRef
    {
        // ensure exclusive access to resource.
        img.set_write();
        let image_info = self.get_image_create_info(img).clone();
        // don't know the layout
        // let old_layout = source.layout.expect("layout should be set for image resource reference");
        // TODO read access might not be necessary if not blending
        // TODO infer from shader pipeline?
        let access = vk::ACCESS_COLOR_ATTACHMENT_READ_BIT | vk::ACCESS_COLOR_ATTACHMENT_WRITE_BIT ;
        let src_stage_mask = img.src_stage_mask;
        // FIXME not really sure what to put here
        let dst_stage_mask = vk::PIPELINE_STAGE_ALL_GRAPHICS_BIT;
        // build the attachment description for the input
        // will be used during the renderpass scheduling stage
        let attachment_desc = vk::AttachmentDescription {
            flags: vk::AttachmentDescriptionFlags::default(),    // FIXME is aliasing possible?
            initial_layout: vk::ImageLayout::Undefined,             // determined later
            final_layout: vk::ImageLayout::ColorAttachmentOptimal,       // FIXME final layout after the pass, not sure what to put here.
            format: image_info.format,
            load_op: vk::AttachmentLoadOp::Load,        // FIXME eeeeh same as needing read access?
            store_op: vk::AttachmentStoreOp::Store,     // FIXME pretty sure we need to store things anyway although might not be necessary if we don't plan to read the data in another pass?
            stencil_load_op: vk::AttachmentLoadOp::DontCare,
            stencil_store_op: vk::AttachmentStoreOp::DontCare,
            samples: image_info.samples,
        };
        // insert dependency into the graph
        self.graph.add_edge(img.task, task, Dependency {
            access_bits: access,
            src_stage_mask,
            dst_stage_mask,
            details: DependencyDetails::Attachment {
                id: img.id,
                index: attachment_index,
                description: attachment_desc
            }
        });
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
    pub fn create_image_2d(&mut self, (width, height): (u32,u32), format: vk::Format) -> ImageRef {
        // create a task associated with this creation op
        let task = self.create_task("dummy");
        let image_create_info = vk::ImageCreateInfo {
            s_type: vk::StructureType::ImageCreateInfo,
            p_next: ptr::null(),
            flags: vk::ImageCreateFlags::default(),
            image_type: vk::ImageType::Type2d,
            format,
            extent: vk::Extent3D { width, height, depth: 1 },
            mip_levels: 1,      // FIXME
            array_layers: 1,    // FIXME
            samples: vk::SAMPLE_COUNT_1_BIT,        // FIXME
            tiling: vk::ImageTiling::Optimal,       // FIXME
            // inferred from the graph
            usage: vk::ImageUsageFlags::default(),
            sharing_mode: vk::SharingMode::Exclusive,   // FIXME
            queue_family_index_count: 0,                // FIXME
            p_queue_family_indices: ptr::null(),
            initial_layout: vk::ImageLayout::Undefined, // inferred
        };

        // get an index to generate a name for this resource.
        // It's not crucial that we get a unique one,
        // as the name of resources are here for informative purposes only.
        let naming_index = self.images.len();
        let new_id = self.add_image_resource(ImageResource::new(format!("IMG_{:04}", naming_index), &image_create_info));

        ImageRef {
            task,
            id: new_id,
            read: Cell::new(false),
            written: Cell::new(false),
            src_stage_mask: vk::PIPELINE_STAGE_TOP_OF_PIPE_BIT  // no need to sync, just created it
        }
    }

    /*
    pub(crate) fn get_resource_name(&self, r: ResourceId) -> &str {
        &self.resources[r.0 as usize].name
    }
    */

    /// Adds a transient buffer resource.
    pub(crate) fn add_buffer_resource(&mut self, resource: BufferResource) -> BufferId {
        self.buffers.push(FrameResource::Transient { resource });
        BufferId((self.buffers.len()-1) as u32)
    }

    /// Adds a transient image resource.
    pub(crate) fn add_image_resource(&mut self, resource: ImageResource) -> ImageId {
        self.images.push(FrameResource::Transient { resource });
        ImageId((self.images.len()-1) as u32)
    }

    fn dump<W: Write>(&self, w: &mut W)
    {
        // dump resources
        writeln!(w, "--- RESOURCES ---");
        for (i,r) in self.images.iter().enumerate() {
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
        for (i,r) in self.buffers.iter().enumerate() {
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
                &DependencyDetails::Image { id, new_layout } => {
                    writeln!(w, "IMAGE ACCESS {}(#{}) -> {}(#{})", src_task.name, src.index(), dest_task.name, dest.index());
                    writeln!(w, "  resource ......... {:08X}", id.0);
                    writeln!(w, "  access ........... {:?}", d.access_bits);
                    writeln!(w, "  srcStageMask ..... {:?}", d.src_stage_mask);
                    writeln!(w, "  dstStageMask ..... {:?}", d.dst_stage_mask);
                    writeln!(w, "  newLayout ........ {:?}", new_layout);
                },
                &DependencyDetails::Attachment { id, index, ref description } => {
                    writeln!(w, "ATTACHMENT {}(#{}) -> {}(#{})", src_task.name, src.index(), dest_task.name, dest.index());
                    writeln!(w, "  resource ......... {:08X}", id.0);
                    writeln!(w, "  access ........... {:?}", d.access_bits);
                    writeln!(w, "  srcStageMask ..... {:?}", d.src_stage_mask);
                    writeln!(w, "  dstStageMask ..... {:?}", d.dst_stage_mask);
                    writeln!(w, "  index ............ {}", index);
                    writeln!(w, "  format ........... {:?}", description.format);
                    writeln!(w, "  loadOp ........... {:?}", description.load_op);
                    writeln!(w, "  storeOp .......... {:?}", description.store_op);
                    writeln!(w, "  initialLayout .... {:?}", description.initial_layout);
                    writeln!(w, "  finalLayout ...... {:?}", description.final_layout);
                },
                &DependencyDetails::Buffer { id } => {
                    writeln!(w, "BUFFER ACCESS {}(#{}) -> {}(#{})", src_task.name, src.index(), dest_task.name, dest.index());
                    writeln!(w, "  resource ......... {:08X}", id.0);
                    writeln!(w, "  access ........... {:?}", d.access_bits);
                    writeln!(w, "  srcStageMask ..... {:?}", d.src_stage_mask);
                    writeln!(w, "  dstStageMask ..... {:?}", d.dst_stage_mask);
                },
            }
            writeln!(w);
        }
    }


    pub fn submit(mut self)
    {
        // TODO
        self.dump(&mut stdout());
        let mut dot = File::create("graph.dot").unwrap();
        self.dump_graphviz(&mut dot);
    }
}

impl Context {
    /// Creates a frame.
    /// The frame handles the scheduling of all GPU operations, synchronization between
    /// command queues, and synchronization with the CPU.
    pub fn new_frame(&mut self) -> Frame
    {
        Frame::new(self)
    }
}