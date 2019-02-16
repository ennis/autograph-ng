# TODO list

### Meta
- (meta) Documentation

### Outstanding
- (spirv) Unit-tests for layout calculation
- (spirv) Parse constant values for array sizes (panic on specialization for now)
- (shader/macros) Cleanup autograph_shader_macros (keep only include_shader, nuke the preprocessor)
- (shader/macros) include_str all includes so that shader is recompiled even if a header changed
- (render/backend) backend calls: return Result<> instead of panicking
- (render/pipeline/validation) Check vertex input interfaces
- (render/pipeline/validation) Precise errors
- (render) add an (unsafe) API to create a pipeline and skip validation
- (render/macros) error msg instead of panic on non-repr(C) structs
- (render-gl/util) unit tests for dropless arena
- (render) convenience methods
    - create_{vertex,fragment,...}_shader(_module)

    
### Enhancements
- (render/validation) accept structs with single member in place of just the member
- (render/pipeline/args) support reuse of Arguments struct without ArgumentBlock indirection (paste copy of Arguments)

### Archived
- DONE (render/pipeline) Interface items directly in PipelineInterface, without needing a separate DescriptorSetInterface
    - new pipeline interface
- DONE (spirv) Implement `std140_layout_and_size` for array of structs
- DONE (render/pipeline) FragmentOutputInterface, create framebuffers from that
- DONE (imageio) rethink read_to_vec: return own wrapper with read channels, can extract vec (read_to_buffer)
     - ImageBuffer: borrow as `&[u8]` slice
- OUTDATED (render) remove TypeId in create_info, replace with generic methods in the backend instance (can fake typeids)
    - caching now done in frontend
- DONE (imageio) subimage.{width,height} convenience methods