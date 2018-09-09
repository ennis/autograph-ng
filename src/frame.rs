use std::io::{stdout, Write};
use std::cell::Cell;
use std::ptr;

use ash::vk;
use petgraph::{Graph, Direction, Directed, graph::NodeIndex};

use context::Context;

#[derive(Copy,Clone,Debug,Eq,PartialEq,Ord,PartialOrd,Hash)]
pub struct ResourceId(pub(crate) u32);

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
    /// Resources that this node writes to.
    pub(crate) writes: Vec<ResourceId>,
    /// Resources that should be created for this node.
    pub(crate) creates: Vec<ResourceId>
}

/// Represents a dependency between tasks in the frame graph.
pub(crate) struct Dependency
{
    /// The resource depended on.
    resource: ResourceId,
    /// How this resource is accessed by the dependent task.
    /// See vulkan docs for all possible flags.
    access_bits: vk::AccessFlags,
    /// What pipeline stage must have completed on the dependency.
    /// By default, this is BOTTOM_OF_PIPE.
    src_stage_mask: vk::PipelineStageFlags,
    /// What pipeline stage of this task (destination) is waiting on the dependency.
    /// By default, this is TOP_OF_PIPE.
    dst_stage_mask: vk::PipelineStageFlags,
    /// Details of the dependency specific to the usage and the type of resource.
    details: DependencyDetails
}

impl Dependency
{
    fn dump<W: Write>(&self, w: &mut W) {
        writeln!(w, "  resource ......... {:08X}", self.resource.0);
        writeln!(w, "  access ........... {:?}", self.access_bits);
        writeln!(w, "  srcStageMask ..... {:?}", self.src_stage_mask);
        writeln!(w, "  dstStageMask ..... {:?}", self.dst_stage_mask);
        match &self.details {
            &DependencyDetails::Image { old_layout, new_layout } => {
                writeln!(w, "  oldLayout ........ {:?}", old_layout);
                writeln!(w, "  newLayout ........ {:?}", new_layout);
            },
            &DependencyDetails::Attachment { ref attachment_description } => {
                writeln!(w, "  format ........... {:?}", attachment_description.format);
                writeln!(w, "  loadOp ........... {:?}", attachment_description.load_op);
                writeln!(w, "  storeOp .......... {:?}", attachment_description.store_op);
                writeln!(w, "  initialLayout .... {:?}", attachment_description.initial_layout);
                writeln!(w, "  finalLayout ...... {:?}", attachment_description.final_layout);
            },
            &DependencyDetails::Buffer {  } => {
            },
        }
    }
}

/*impl Dependency
{
    pub(crate) fn default_sampled_image(resource: &ResourceRef, )
}*/

/// Details of a dependency that is specific to the usage of the resource, and its
/// type.
pub(crate) enum DependencyDetails
{
    /// Image dependency: either a sampled image or a storage image.
    /// This produces the image barrier.
    Image {
        old_layout: vk::ImageLayout,
        new_layout: vk::ImageLayout,
    },
    /// Image used as an attachment.
    Attachment {
        attachment_description: vk::AttachmentDescription,
    },
    /// Details specific to buffer data.
    Buffer {}
}


pub(crate) type TaskId = NodeIndex<u32>;
type FrameGraph = Graph<Task, Dependency, Directed, u32>;

/// Represents one output of a task. This is used as part of the API to build dependencies between nodes.
pub struct ResourceRef
{
    /// Originating task.
    task: TaskId,
    /// Which resource.
    resource: ResourceId,
    /// What pipeline stage must have completed on the dependency.
    src_stage_mask: vk::PipelineStageFlags,
    /// Whether this resource has already been set as a read dependency.
    /// Prevents all writes.
    read: Cell<bool>,
    /// Whether this resource has already been set as a write dependency.
    /// Prevents all subsequent reads and writes.
    written: Cell<bool>,
    /// If it's an image resource, the current layout of the image.
    layout: Option<vk::ImageLayout>,
}

impl ResourceRef
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


/*
/// A transient resource that lives only during one frame and is reclaimed after.
/// The attached storage can be aliased between resources if the system determines that
/// no overlap is possible.
pub(crate) enum TransientResource
{
    Texture(Texture),
    Buffer(Buffer),
}*/

/*impl TransientResource
{
}*/

pub(crate) enum TransientResource
{
    Buffer(BufferResource),
    Image(ImageResource)
}

impl TransientResource
{
    fn new_image(create_info: &vk::ImageCreateInfo) -> TransientResource {
        TransientResource::Image(ImageResource { create_info: create_info.clone() })
    }

    fn new_buffer(create_info: &vk::BufferCreateInfo) -> TransientResource {
        TransientResource::Buffer(BufferResource { create_info: create_info.clone() })
    }

    fn as_image(&self) -> Option<&ImageResource> {
        match self {
            TransientResource::Image(ref img) => Some(img),
            TransientResource::Buffer(_) => None
        }
    }

    fn as_image_mut(&mut self) -> Option<&mut ImageResource> {
        match self {
            TransientResource::Image(ref mut img) => Some(img),
            TransientResource::Buffer(_) => None
        }
    }

    fn as_buffer(&self) -> Option<&BufferResource> {
        match self {
            TransientResource::Image(_) => None,
            TransientResource::Buffer(ref buf) => Some(buf)
        }
    }

    fn as_buffer_mut(&mut self) -> Option<&mut BufferResource> {
        match self {
            TransientResource::Image(_) => None,
            TransientResource::Buffer(ref mut buf) => Some(buf)
        }
    }
}


pub struct BufferResource
{
    create_info: vk::BufferCreateInfo,
    // TODO allocation
}

pub struct ImageResource
{
    create_info: vk::ImageCreateInfo,
}


/*/// Helper to write graphs
///
graph
    .node()
    .attribute()
    .attribute()
    .build();*/
/*pub struct GraphvizOutput<'a, W: Write>
{
    writer: &'a mut W,
}

pub struct GraphvizNodeBuilder<'a, W:Write>
{
    writer: &'a mut W,
}

impl GraphvizNodeBuilder
{
    fn at
}*/

/// A frame: manages transient resources within and across frames.
pub struct Frame<'ctx> {
    context: &'ctx mut Context,
    /// The DAG of tasks.
    graph: FrameGraph,
    /// Table of transient resources for this frame.
    resources: Vec<TransientResource>,
    /// The root node from which all transient resources originates.
    /// This is just here to avoid an Option<> into ResourceRefs
    transient_root: TaskId,
}

impl<'ctx> Frame<'ctx> {
    /// Creates a new frame.
    fn new(context: &'ctx mut Context) -> Frame<'ctx> {
        let mut graph = FrameGraph::new();
        // create a dummy task for the transient root.
        let transient_root = graph.add_node(Task {
            creates: Vec::new(),
            writes: Vec::new(),
        });
        let mut f = Frame {
            graph,
            context,
            resources: Vec::new(),
            transient_root
        };
        f
    }

    /// Creates a new task.
    /// Returns the ID to the newly created task.
    pub fn create_task(&mut self) -> TaskId {
        self.graph.add_node(Task {
            creates: Vec::new(),
            writes: Vec::new()
        })
    }

    /// Updates the data contained in a texture. This creates a task in the graph.
    /// This does not synchronize: the data to be modified is first uploaded into a
    /// staging buffer first.
    fn update_image(&mut self, image: &ResourceRef, data: ()) -> ResourceRef {
        unimplemented!()
    }

    /// Returns information about a texture (Transient or Persistent)
    /// Panics if the reference does not point to a texture.
    pub fn get_image_create_info<'a>(&'a self, rref: &ResourceRef) -> &'a vk::ImageCreateInfo {
        &self.resources[rref.resource.0 as usize].as_image().expect("not an image resource").create_info
    }

    /// Returns information about a buffer (Transient or Persistent)
    /// Panics if the reference does not point to a buffer.
    pub fn get_buffer_create_info<'a>(&'a self, rref: &ResourceRef) -> &'a vk::BufferCreateInfo {
        &self.resources[rref.resource.0 as usize].as_buffer().expect("not a buffer resource").create_info
    }

    pub fn image_sample_dependency(&mut self, task: TaskId, source: &ResourceRef)
    {
        // increase read count
        source.set_read();
        // fetch info about the resource
        let image_info = self.get_image_create_info(source).clone();
        let old_layout = source.layout.expect("layout should be set for image resource reference");
        // Sampling is a shader read operation
        let access = vk::ACCESS_SHADER_READ_BIT;
        let src_stage_mask = source.src_stage_mask;
        // TODO: optimal dst_stage_mask knowing the pipeline associated to the task.
        // technically, the earliest point in the pipeline where we can sample a texture is the vertex
        // shader stage. But it could be accessed later, potentially allowing for better pipelining.
        let dst_stage_mask = vk::PIPELINE_STAGE_VERTEX_SHADER_BIT;
        // insert dependency into the graph
        self.graph.add_edge(source.task, task, Dependency {
            resource: source.resource,
            access_bits: access,
            src_stage_mask,
            dst_stage_mask,
            details: DependencyDetails::Image {
                old_layout,
                // transfer to layout suited to shader access
                new_layout: vk::ImageLayout::ShaderReadOnlyOptimal
            }
        });
    }

    /// Color attachment.
    /// TODO: maybe additional parameters, such as clear color?
    pub fn color_attachment_dependency(&mut self, task: TaskId, source: &ResourceRef) -> ResourceRef
    {
        // ensure exclusive access to resource.
        source.set_write();
        let image_info = self.get_image_create_info(source).clone();
        let old_layout = source.layout.expect("layout should be set for image resource reference");
        // TODO read access might not be necessary if not blending
        // TODO infer from shader pipeline?
        let access = vk::ACCESS_COLOR_ATTACHMENT_READ_BIT | vk::ACCESS_COLOR_ATTACHMENT_WRITE_BIT ;
        let src_stage_mask = source.src_stage_mask;
        // FIXME not really sure what to put here
        let dst_stage_mask = vk::PIPELINE_STAGE_ALL_GRAPHICS_BIT;
        // build the attachment description for the input
        // will be used during the renderpass scheduling stage
        let attachment_desc = vk::AttachmentDescription {
            flags: vk::AttachmentDescriptionFlags::default(),    // FIXME is aliasing possible?
            initial_layout: old_layout,
            final_layout: vk::ImageLayout::ColorAttachmentOptimal,       // FIXME final layout after the pass, not sure what to put here.
            format: image_info.format,
            load_op: vk::AttachmentLoadOp::Load,        // FIXME eeeeh same as needing read access?
            store_op: vk::AttachmentStoreOp::Store,     // FIXME pretty sure we need to store things anyway although might not be necessary if we don't plan to read the data in another pass?
            stencil_load_op: vk::AttachmentLoadOp::DontCare,
            stencil_store_op: vk::AttachmentStoreOp::DontCare,
            samples: image_info.samples,
        };
        // insert dependency into the graph
        self.graph.add_edge(source.task, task, Dependency {
            resource: source.resource,
            access_bits: access,
            src_stage_mask,
            dst_stage_mask,
            details: DependencyDetails::Attachment {
                attachment_description: attachment_desc
            }
        });
        // create the new resource descriptor: new version of this resource, originating
        // from this task
        ResourceRef {
            // FIXME not sure, maybe PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT is sufficient?
            src_stage_mask: vk::PIPELINE_STAGE_ALL_GRAPHICS_BIT,
            resource: source.resource,
            task,
            read: Cell::new(false),
            written: Cell::new(false),
            // FIXME this should be decided during scheduling
            layout: Some(vk::ImageLayout::ColorAttachmentOptimal)
        }
    }

    /// Creates a transient 2D image associated with the specified task.
    /// The initial layout of the image is inferred from its usage in depending tasks.
    pub fn image_2d(&mut self, task: TaskId, (width, height): (u32,u32), format: vk::Format) -> ResourceRef {
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
            // FIXME
            sharing_mode: vk::SharingMode::Exclusive,
            // FIXME eeeeh...
            queue_family_index_count: 0,
            p_queue_family_indices: ptr::null(),
            // inferred
            initial_layout: vk::ImageLayout::Undefined,
        };

        let resource = self.add_resource(TransientResource::new_image(&image_create_info));

        ResourceRef {
            task,
            resource,
            read: Cell::new(false),
            written: Cell::new(false),
            // inferred during scheduling
            layout: Some(vk::ImageLayout::Undefined),
            src_stage_mask: vk::PIPELINE_STAGE_TOP_OF_PIPE_BIT  // no need to sync, just created it
        }
    }

    fn get_resource_name(&self, r: ResourceId) -> String {
        match self.resources[r.0 as usize] {
            TransientResource::Buffer(_) => { format!("BUF_{:08X}", r.0) },
            TransientResource::Image(_) => { format!("IMG_{:08X}", r.0) },
        }
    }

    fn add_resource(&mut self, r: TransientResource) -> ResourceId {
        self.resources.push(r);
        ResourceId((self.resources.len()-1) as u32)
    }

    fn dump<W: Write>(&self, w: &mut W)
    {
        // dump resources
        writeln!(w, "--- RESOURCES ---");
        for (i,r) in self.resources.iter().enumerate() {
            match r {
                &TransientResource::Image(ref r) => {
                    writeln!(w, "Image R_{}", i);
                    writeln!(w, "  imageType ........ {:?}", r.create_info.image_type);
                    writeln!(w, "  width ............ {}", r.create_info.extent.width);
                    writeln!(w, "  height ........... {}", r.create_info.extent.height);
                    writeln!(w, "  depth ............ {}", r.create_info.extent.depth);
                    writeln!(w, "  format ........... {:?}", r.create_info.format);
                    writeln!(w, "  usage ............ {:?}", r.create_info.usage);
                },
                &TransientResource::Buffer(ref r) => {
                    writeln!(w, "Buffer R_{}", i);
                    writeln!(w, "  size ............. {}", r.create_info.size);
                    writeln!(w, "  usage ............ {:?}", r.create_info.usage);
                }
            }
            writeln!(w);
        }
        writeln!(w);

        // tasks
        writeln!(w, "--- TASKS ---");
        for n in self.graph.node_indices() {
            writeln!(w, "T_{}", n.index());
        }
        writeln!(w);

        // dependencies
        writeln!(w, "--- DEPS ---");
        for e in self.graph.edge_indices() {
            let (src, dest) = self.graph.edge_endpoints(e).unwrap();
            let d = self.graph.edge_weight(e).unwrap();
            writeln!(w, "T_{} -> T_{}", src.index(), dest.index());
            d.dump(w);
            writeln!(w);
        }
    }

   /* fn dump_nodes<W: Write>(&self, w: &mut W)
    {
        // dump resources
        writeln!(w, "--- RESOURCES ---");
        for (i,r) in self.resources.iter().enumerate() {
            match r {
                &TransientResource::Image(ref r) => {
                    write!(w, "{} [fillcolor=navyblue label=\"", self.get_resource_name(ResourceId(i)));
                    write!(w, "|Image R_{}\\n", i);
                    write!(w, "|{{imageType| {:?} }}\\n", r.create_info.image_type);
                    write!(w, "|{{width    | {}   }}\\n", r.create_info.extent.width);
                    write!(w, "|{{height   | {}   }}\\n", r.create_info.extent.height);
                    write!(w, "|{{depth    | {}   }}\\n", r.create_info.extent.depth);
                    write!(w, "|{{format   | {:?} }}\\n", r.create_info.format);
                    write!(w, "|{{usage    | {:?} }}\\n", r.create_info.usage);
                    writeln!(w, "|\"]");
                },
                &TransientResource::Buffer(ref r) => {
                    write!(w, "R_{} [fillcolor=red4 label=\"", i);
                    write!(w, "|Buffer R_{}\\n", i);
                    write!(w, "|{{size   | {:?} }}\\n", r.create_info.size);
                    write!(w, "|{{usage  | {}   }}\\n", r.create_info.usage);
                    writeln!(w, "|\"]");
                }
            }
            writeln!(w);
        }
        writeln!(w);

        // tasks
        //writeln!(w, "--- TASKS ---");
        for n in self.graph.node_indices() {
            writeln!(w, "T_{}", n.index());
        }
        writeln!(w);

        // dependencies
        writeln!(w, "--- DEPS ---");
        for e in self.graph.edge_indices() {
            let (src, dest) = self.graph.edge_endpoints(e).unwrap();
            let d = self.graph.edge_weight(e).unwrap();

            let color_code = match &d.details {
                &DependencyDetails::Image { .. } => {
                    if d.access_bits & vk::ACCESS_SHADER_WRITE_BIT { "purple4" }  // written image
                    else { "midnightblue" } // read-only image
                },
                &DependencyDetails::Attachment { .. } => { "darkgreen" }
                &DependencyDetails::Buffer {  } => {
                    if d.access_bits & vk::ACCESS_SHADER_WRITE_BIT { "violetred4" }  // written
                    else { "red4" } // read-only
                },
            };

            //let resource_name =

            writeln!(w, "T_{} -> T_{}", src.index(), dest.index());

            write!(w, "DEP_{} [fillcolor={} label=\"|{} ", i, depcolor,);
            writeln!(w, "", i);

            writeln!(w, "  resource ......... {:08X}", self.resource.0);
            writeln!(w, "  access ........... {:?}", self.access_bits);
            writeln!(w, "  srcStageMask ..... {:?}", self.src_stage_mask);
            writeln!(w, "  dstStageMask ..... {:?}", self.dst_stage_mask);
            match &d.details {
                &DependencyDetails::Image { old_layout, new_layout } => {
                    write!(w, "|Image R_{}\\n", d.resource.0);
                    write!(w, "|{{access       | {}   }}\\n", r.create_info.extent.width);
                    write!(w, "|{{srcStageMask | {}   }}\\n", r.create_info.extent.height);
                    write!(w, "|{{dstStageMask | {}   }}\\n", r.create_info.extent.depth);
                    write!(w, "|{{format       | {:?} }}\\n", r.create_info.format);
                    write!(w, "|{{usage        | {:?} }}\\n", r.create_info.usage);

                    writeln!(w, "  oldLayout ........ {:?}", old_layout);
                    writeln!(w, "  newLayout ........ {:?}", new_layout);
                },
                &DependencyDetails::Attachment { ref attachment_description } => {
                    writeln!(w, "  format ........... {:?}", attachment_description.format);
                    writeln!(w, "  loadOp ........... {:?}", attachment_description.load_op);
                    writeln!(w, "  storeOp .......... {:?}", attachment_description.store_op);
                    writeln!(w, "  initialLayout .... {:?}", attachment_description.initial_layout);
                    writeln!(w, "  finalLayout ...... {:?}", attachment_description.final_layout);
                },
                &DependencyDetails::Buffer {  } => {
                },
            }
            d.dump(w);
            writeln!(w);
        }
    }*/

    // Blue: sampled image
    // Violet: R/W image
    // Dark green: color attachment
    // Light green: depth attachment
    // Orange: buffer
    // Dark red: R/W buffer
    // Green nodes: graphics
    // Yellow nodes: compute


    pub fn submit(mut self)
    {
        // TODO
        self.dump(&mut stdout());
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