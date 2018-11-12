Scheduler
==================================

### resource allocation:
* separate for each queue
* do not alias memory for now, but re-use

### Graph
* Node = pass type (graphics or compute), callback function
callback function parameters:
* command buffer, within a subpass instance initialized with the correct attachments
* container object that holds all requested resources in their correct state (all images, sampled images, buffer, storage buffer, uniform buffers, etc.)
The callback just has to submit the draw command.
* Edge = dependency
    * toposort
    * check for concurrent read/write hazards (ok by API design)
    * infer usage of resources from graphs (mutate graph)
- schedule renderpasses
- reorder to minimize layout transitions
- allocate resources with aliasing
- group in render passes (draw commands with the same attachments; notably: chains without `sample` dependencies)
     - new dependency type: attachment input
     - at least one subpass for each different attachment?
     - minimize the number of attachments
     - a sample dependency always breaks the render pass
- heuristic for renderpasses:
     - schedule a pass (starting from the first one)
     - follow all output attachments
     - if no successor has a sample-dependency, and NO OTHER ATTACHMENTS, then merge the successors into the renderpass
     - schedule_renderpasses()
     - create dependencies between subpasses (e.g. output-to-input attachment)
     - user-provided hints?
- schedule renderpasses (dependencies between renderpasses: e.g. layout transitions)
- insert memory barriers
- insert layout transitions
Various graphs:
- initial graph
- graph with render passes
- graph with render passes and explicit layout transitions

All work on the GPU is done inside nodes in a frame.
- DEVICE: Submission queue: when allocating a transient resource:
     - find a block to allocate, try to sync on semaphore,
	- if not yet signalled, then allocate a new block
     - if failed (transient memory limit exceeded), then sync on a suitable block
- DEVICE: Submission queue: when importing a resource: semaphore sync (may be in use by another queue)
- UPLOADS: fence sync on frame N-max_frames_in_flight
When using a resource, what to sync on?
- Associate semaphore to resource
Sometimes, a group of resources (Buffers, Images, Memory blocks) can share the same semaphore:
- SyncResourceGroup
A resource can belong to a SyncGroup? Rc<SyncGroup>? SyncGroupId?
The SyncGroup is assigned when? on submission?
all resources used during the construction of a command buffer should be recorded
```
context.sync_group(frame, command_buffer, |sync_resources| {
	// if resource
	sync_resources.use(...)
})
```
Next step: insert queue barriers
- for each node
     - for each dependency
         - if cross-queue dependency
             - if already syncing on a resource of the same queue that is finished later: do nothing
             - otherwise, if finished earlier: remove old sync from task and put it on the later task (allocate semaphore if needed)
- tasks have list of semaphores to signal, and semaphores to wait
Next step:
- traverse each queue subgraph and regroup tasks into 'jobs' that do not have any dependency on other queues
- for each job, collect wait/signal semaphores
- don't forget to add semaphores for external dependencies
- this is handled by the import tasks
- import tasks: what do they do?
     - execute on the specified queue
     - synchronizes with the previous frame (semaphores in the resource): adds semaphore to the job
     - be careful not to synchronize with the semaphore from 2 or 3 frames before!
- should also have export/exit nodes?
     - exit nodes for external resources: signal resource ready
     - automatic insertion of exit tasks
- for each job: ring buffer of MAX_FRAMES_IN_FLIGHT semaphores?
     - no need, I think, can reuse the same semaphores
     - same for external resources, AS LONG AS the wait is on the same queue
         - if the wait changes queues, then all hope is lost...
             - e.g. if the queue is empty, then the GPU might execute it immediately, but frame N-2 might not have finished
                 and may signal the semaphore, which will launch the task on the new queue with old data
         - solution: have multiple semaphores for external resources, always wait on semaphore for frame N-1 exit
Synchronization of external resources:
- issue: can change queues (don't assume that they are all used on the same queue)
- can read on a queue, read on the other
- exit tasks: put a semaphore in the command stream, to be waited on by the entry (import) task
Limitation:
- cannot sequence (multiple) reads followed by a write!
- maybe it's possible: return another versioned handle from a read
- or modify graph: R(0) -> T1 -> W(0) -> T4 will add an implicit dependency on T4 to T2,T3
                   R(0) -> T2
                   R(0) -> T3
  -> this assumes breadth-first execution...
   t1.read(r0);     // pending access = [t1] (into r0 ref)
   t2.read(r0);     // pending access = [t1,t2]
   t3.read(r0);     // pending access = [t1,t2,t3]
   // now r0 has three readers: reclaim exclusive access
   let r0 = frame.sync(r0);      // r0 is a fresh reference without any R/W count, contains an implicit dependency on t1, t2, t3
   -> insert a virtual task that depends on the three nodes (i.e. not a resource dependency)
   // o1 means write after o1, but what about o2 and o3? => must detect R/W hazard
   t4.write(o1);             // will sync on t1, but it's not enough
   -> OPTION: could force sequencing of reads, in addition to writes
   -> to write a resource, must sync on all pending reads
   -> SOLUTION: add special "sequence" dependencies
Next step: build command buffers
- for each job, create command buffer, traverse graph
Put everything in the graph, including present operations
- some nodes should only execute on a given queue (e.g. the present queue)
Transfer queue:
- upload data immediately to upload buffer
- on schedule: to transfer queue: copy to resource
DONE Do away with dummy nodes for resource creation:
- clutters the graph with useless nodes, confuses scheduling.
- initialize to the correct state on first use.
DONE Decouple dependency edges and usage of the resource within the task.
- A resource can have multiple usages within the same task.
     - e.g. color attachment and input attachment
- Dependency = only pipeline barrier info
Implicit dependencies between tasks with ordering
- user submitted ordering is important
- write after read is not an error, but will insert a pipeline barrier automatically
- same for read after write
-> ordering is defined implicitly by the submission order.
-> benefits: less cluttered API

Images
====================================

Creating persistent images
High-level uses:
- Immutable + sampled (texture)
- Attachment + sampled (postproc)
- Attachment only
- CPU upload
- CPU readback

Low-level:
- usage flags
- queues


Memory types:

    VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT = 0x00000001
    VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT = 0x00000002
    VK_MEMORY_PROPERTY_HOST_COHERENT_BIT = 0x00000004
    VK_MEMORY_PROPERTY_HOST_CACHED_BIT = 0x00000008
    VK_MEMORY_PROPERTY_LAZILY_ALLOCATED_BIT = 0x00000010
    VK_MEMORY_PROPERTY_PROTECTED_BIT = 0x00000020
    
Queue flags:

    VK_QUEUE_GRAPHICS_BIT
    VK_QUEUE_COMPUTE_BIT
    VK_QUEUE_TRANSFER_BIT
    VK_QUEUE_SPARSE_BINDING_BIT
    VK_QUEUE_PROTECTED_BIT
    
Immutable => VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT
CPU upload => VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT(R) + VK_MEMORY_PROPERTY_HOST_COHERENT_BIT(P)

Q: exposed level of detail to the user
```
Dimensions::Dim2d { width: 1280, height: 720 },
MipmapsCount::One
HostAccess::NoAccess | Mappable
```

External API:
* `Context::create_image()` VS `Image::new(&context)`
    * Second one was preferred previously
    * Deallocation is a bit weird: `Image::destroy(&context)`
    * Benefits: less code in `Context`, more idiomatic, no need for different functions when creating specialized image types

Internal API:
* Image: no desc structure, but impl ImageDescription
* Image::new_uninitialized(): queue families
* Image::bind_memory()
* Q: should pools be exposed in the external API?
    * User might need them, and it forces a cleaner design for the internal API
* Q: How does allocating an image in a pool works?
    * Image becomes implicitly bound to the pool
    * Releasing the pool is unsafe
    * Options:
        * Pool strong-refs the image
        * Image strong-refs the pool
        * Count number of allocations and deallocations, panic if count is not zero at exit
        * Do nothing, deallocating the pool while images are still live is undefined behavior and unsafe
            * Cannot expose this API to the user
            * OR: dropping the pool does not release the memory
                * memory is released when the last image is deleted
* Q: How does allocating anything works?
    * vulkan spec says that all objects created with a device must be destroyed before destroying the device 
    * but the current API does not ensure that a resource will be destroyed before the device
        * Option 1: track number of allocated objects, panic if count not zero at drop time (**)
            * Gives no information about the leak...
            * Lightweight option
        * Option 2: extend lifetime of device with Arc<Device>

```
Image::new(..., Some(pool));
```
* Q: Does the image owns its allocation?

Note: the external API is quite high-level
* Still, make sure that the internal API is not too unsafe
* API issue: pooled resources
    * e.g. free all images at once
* Internal API issue: leaking owned handles
* The overhead of adding an Arc<Context> is negligible
    * still, don't add it if not absolutely necessary (prefer passing VkDevice or Context)
* The overhead for safety appears in other ways:
    * need to put something into the created object to ensure that it won't be deleted on the wrong parent object by mistake
        * marker indicating that it comes from some parent object
* Conclusion: putting a refcounted backpointer to the parent object is the easiest solution
    * must allocate context in an Arc
    * might as well rename context to device, for good measure
* To support polymorphism and strongly-typed resources, images should be Arc<Image>, and have an Image trait

Lifetime of memory allocations:
* Before deleting a pool, must be sure that all associated resources are destroyed, and not in use inside the pipeline, 
  and that no internal handles remain.
  
Aliasing of memory allocations:
* Can't alias a memory allocation if passed by value to the object
    * Optional reference to allocation
    
Basically, just copy vulkano (...) except that:
* all GPU commands are managed by a frame graph
    * notably, all resource access (except for initialization) must happen within the frame graph
* ???

Bikeshedding API
* Parameters vs structs
    * Use parameters as they support generics with less noise
* e.g. swapchains
* structs: indirection when using generic parameters

Images
* Base unsafe type, unbound memory
* Tagged image types
    * TransferSourceCapability
    * TransferDestinationCapability
    * SampleCapability
    * StorageCapability
    * ColorAttachmentCapability
    * DepthAttachmentCapability
    * AttachmentCapability
    * TransientAttachmentCapability
    * Format
    * SpecificFormat<T>
    * UnknownFormat
* Define tagged image types by combining capabilities
* impl on Image or on TagType?
    * on type tag, not instantiable
* image tags: 

 
```
image_tag!{
    pub type Transfer = TransferSourceCapability + TransferDestinationCapability;
}

fn new() -> Swapchain<ImageType> {
}
```

Sharing between queues:
* know in advance
* encode in type
* be conservative
* rule: do not expose sharing to the user
    * queue creation and scheduling is handled by the framework, with hints from the graph
* make choices:
    * presentation images are always EXCLUSIVE
    * persistent images are always CONCURRENT across all queue families (by default)

Image usage should be abstracted away:
* delayed allocation
* images implicitly bound to a frame graph?
* issue: delayed uploads

```
with_frame(|frame| {
    accum = Image(...); 
    
})

frame {
    image XXX { ... }
    image YYY { ... }
    
    pass A { attachment <- XXX }
}

```

Issue: what is the lifetime of a frame?
- recreated on every frame
    - can be costly (allocation, scheduling)
    - good fit for dynamic workloads (e.g stuff that is run once every two frames?)
- OR create once, execute many
    - less costly, reuse stuff
    - inflexible (borrows persistent resources forever)
    - need "input ports" for dynamic data (cannot borrow input data forever, avoid shared references)
    

Swapchain images **owned** by the swapchain
- acquire_swapchain_image() returns what?
    - Arc?
    - Borrow?
    - Value?

Question: how to store a reference to the image when used within a frame?
    - API: expects Arc, store Arc in frame, guard with fence
    - API: expects Image, but clone internal Arc
    - Might be a reference to an image, but also indirectly to a swapchain
    - Do not store a reference, just check for GPU use on drop
A: take a copy of a generic image, or just the raw handle
    
Q: expose Images through Arc<> or through naked objects?
    - naked objects are possible, with "frame borrows"
        - frame requests that image lives as long as the current frame index
        - when image is dropped
            - check that frame is finished through device backref
            - if not, move object into deferred deletion list in device
        - need non-owning images

Q: vkAcquireNextImageKHR should be called as late as possible. This raises an issue with the frame graph, which needs
   to call it back when generating the buffers.
   - Borrow the swapchain image
        - ergonomics loss
   - Turn the swapchain image into a generic "image reference"/"image proxy" that can be acquired at any moment in the future
        - impl IntoImageProxy for SwapchainImage
        - impl IntoImageProxy for GenericImage
        - Must decouple borrow (the resources must live for the current frame) from GPU lock 
            (wait for this semaphore before using the resource, signal this one when finished)
        - ImageProxies are just another name for a borrow...
        - Issue: cannot set some state into the borrowed resource during the frame
            - Notably, cannot remember the layout that the image is into when exiting a frame
            - Cannot remember anything across frames
                - Layout is one thing, but then again the initial layout has no reason to change across frames
                - anything else? except the data inside the image, can't think of anything
            - Other solution: remove image and imageproxies, just use single trait image, with impl FrameSynchronizedObject for Arc<Image>, borrow with Arcs
   
   - Special-case swapchain images in ImageResource
        - `fn swapchain(&self) -> Option<...>`
        - calls underlying swapchainimageproxy
        - remove ImageProxy trait (just query the image directly for non-swapchains)
        
   - (extreme) Build the command buffers on the fly
        - a.k.a do not pursue the frame graph approach
        
Q: FrameGraphs vs on-the-fly command buffer generation?
   - FrameGraphs: full knowledge of the structure and dependencies inside the frame. Can reorder and schedule.
   
   - On-the-fly: 
        - No reordering possible. 
        - Must schedule explicitly or schedule with incomplete information.
        - Aliasing of resources is still possible. 
        - May be faster (no scheduling, no graph allocation, commands directly put into buffers)
        - Just-in-time synchronization
   
   - This is (mostly) an internal aspect, and should not change the API much: keep FrameGraph approach for now.
        
        
Q: Scheduling
    - Scheduling now happens per-task: each task is responsible for scheduling itself
    - A task may output a command into a command buffer, or a queue operation directly (e.g. vkQueuePresentKHR), or both
        (e.g. layout transition + queue operation)
    - all passes that belong to the same renderpass must be scheduled in the same command buffer
    - guarantees when calling task::schedule
        - all resources are properly synchronized
    - tasks should signal the context that they expect
        - renderpass(index)
        - command buffer
        - queue
        - then tasks can get the context they want: queue(), command_buffer(), wait_semaphores() ...
    - operations:
        - TaskOperation::SubpassCommand()
        - TaskOperation::Command
        - TaskOperation::QueueSubmit(command buffer)
        - TaskOperation::QueuePresentKHR(...)
    - TaskContext:
        - CommandBuffer(...)
        - RenderPass(...)
        - Queue(...)
    - Expose a 'virtual queue' that makes no distinction between renderpass, command buffer, or queue ops
        - issue: cannot perform *any* synchronization within a task, even manual ones
            - is this OK?
            - no: provide raw access to queues
            
Q: texture uploads
    - should happen outside frames
    - problem: lifetime of staging buffer?
        - staging buffer should be frame-bound
        - but upload could happen outside a frame
    - problem: uploading very large amounts of texture data in one go:
        - upload blocks on frame finish, but the frame has not even started yet
        - can still upload in a frame, one time
    - solution: create "temporary" frame for upload
        - frames do not need to correspond one-to-one with frames on the screen
        - is that true?
            - what about frames in flight?
            - distinguish between visual frames & non-visual frames?
    - submit command buffer for initial upload to transfer queue, then set initial semaphore
    
Q: redesign image refs
    - more ergonomic: reference to image resource entry, with current state in the graph
        - issue: borrows the whole frame, must refcell everything
        - partial borrows would be nice
        
Target API:
    - simple
        - drop the need to store resource versions: use ordering of commands
        - Read-after-write scenarios
            - a task may call another task that modifies an input resource, and the calling task reads the new resource
            as if it was not modified
                - prevented by handle rename
                - can be prevented by read-only handles, or &mut ref
    - straightforward
    - familiar
    - concise
    - prevents wrong usage
    - use as few as possible rust-specific features
    - importantly: does not interfere with data-driven scenarios
        - e.g. create graph from a file
    - should be relatively low-level
        - higher level wrappers should be possible
    
Internal API for dependencies:
    - should be able to specify one side of a dependency
        - semaphores to wait for
        - pipeline stage to wait for 
    
Q: Expose render passes or not?
    - should not, probably
    - must have a grouping pass:
        - separate pass on the graph, or during scheduling?
            - schedule pass
            - if same renderpass tag
                - schedule as subpass
            - if not: terminate renderpass, start new one
            - next one: next tasks in topological order
                - evaluate renderpass merge candidates (does not use any of the previous attachments as sampled or storage images)
                - set renderpass index
                - try to schedule from given score
            

Schedule state: 
- schedule stack (which ones to try next)

API for graphics:
- Variant A:
    ```
    fn set_color_attachment(index, image, load, store) -> ImageRef
    fn set_depth_attachment(image, load, store) -> ImageRef
    fn set_input_attachment(index, image)
    ```
    - Issue (set_color_attachment validation): color attachment is valid only if not read, or read by input attachment of the same task
        However, no way of knowing that the read is from the same task

- Variant B:
    ```
    fn set_color_attachment(index, image, load, store) -> ImageRef
    fn set_color_input_attachment(index, image, input_index, store) -> ImageRef
    fn set_depth_attachment(image, load, store) -> ImageRef
    fn set_input_attachment(index, image)
    ```
    - set combined color+input attachment at the same time
    - advantage vs Variant A: no need to modify ImageRef
  
- Variant C (index-less):
    ```
    fn add_color_attachment(image, load, store) -> ImageRef
    fn set_depth_attachment(image, load, store) -> ImageRef
    fn add_input_attachment(image)
    ```
    - does not work with combined color+input attachment
    
- Variant D:
    ```
    fn set_color_attachments(index, [{image, load, store}]) -> ???
    fn set_depth_attachment(image, load, store) -> ImageRef
    fn set_input_attachments(index, [image])
    ```
    - Issue: how to return new versions of color attachments? 
    - Issue: see option A, color attachment validation
    
- Variant B' (cosmetic):
    ```
    fn color_attachment(index, image, load, store) -> ImageRef
    fn color_input_attachment(index, image, input_index, store) -> ImageRef
    fn depth_attachment(image, load, store) -> ImageRef
    fn input_attachment(index, image)
    ```
    
- Variant E (two-phase):
    ```
    fn attachment(image, load, store) -> (AttachmentId, ImageRef)
    fn color_attachment(index, attachment_id)
    fn depth_attachment(attachment_id)
    fn input_attachment(attachment_id)
    ```
    - Potential API misuse: store AttachmentId outside subpass
    
- Variant E' (two-phase across subpasses: "AttachmentRef"):
    ```
    fn load_attachment(image, load, store) -> AttachmentRef
    fn color_attachment(index, att_ref) -> AttachmentRef
    fn depth_attachment(att_ref) -> AttachmentRef
    fn input_attachment(att_ref)
    ```
    
- Variant E'+B (combined color+input, two-phase across subpasses: "AttachmentRef"):
    ```
    fn load_attachment(image, load, store) -> AttachmentRef
    fn color_attachment(index, att_ref) -> AttachmentRef
    fn depth_attachment(att_ref) -> AttachmentRef
    fn input_attachment(att_ref)
    ```
    - does not work very well with data-driven scenarios?
   