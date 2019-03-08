# TODO list

### Meta
- (meta) Documentation

### Outstanding
- (spirv) Unit-tests for layout calculation
- (spirv) Parse constant values for array sizes (panic on specialization for now)
    - provide specialization constants when parsing AST
- (render/backend) backend calls: return Result<> instead of panicking
- (render) add an (unsafe) API to create a pipeline and skip validation
- (render/macros) error msg instead of panic on non-repr(C) structs
    - no need for error msg since we should abandon immediately on encountering a non-repr(C)
- (render-gl/util) unit tests for dropless arena
- (render/pipeline/validation) validate fragment outputs
- (render-gl) don't use config crate for configuration
    - just pass a struct to the backend
- (render-extra) load texture from file (OpenImageIO integration)
- (render) create texture and clear with color
    - maybe in render-extra?
    - trait ArenaExt
- (render) validation of ImageUsage flags when used as attachment or sampled image
- (render) image builders for convenience
DONE (render/validation) support booleans in structured buffer interfaces
    - it's not easy because OpTypeBool in spirv cannot be used in externally visible interfaces
    - and a bool in a repr(C) obviously does not satisfy the std140 rules...
    - we loose the "bool" type info somewhere
        -> don't support bool in structured buffers
        -> instead, create a BoolU32 type that is equivalent to PrimitiveType::UnsignedInt

- POSTPONED (render/pipeline/validation) Precise errors
    - the current output is the debug formatted TypeDesc, which is readable enough
    - maybe pinpoint the error instead of dumping the whole typedesc? 
        - not a priority: it's easy enough to compare two TypeDesc dumps visually

- (render-extra) post-proc stack
- (render) blitting
    
### Enhancements
- (render/validation) accept structs with single member in place of just the member
- (render/pipeline/args) support reuse of Arguments struct without ArgumentBlock indirection (paste copy of Arguments)

### Archived
- DONE (render/pipeline/validation) Check vertex input interfaces
- DONE (shader/macros) Cleanup autograph_shader_macros (keep only include_shader, nuke the preprocessor)
- DONE (shader/macros) include_str all includes so that shader is recompiled even if a header changed
- DONE (render) convenience methods `create_{vertex,fragment,...}_shader(_module)`
- DONE (render/pipeline) Interface items directly in PipelineInterface, without needing a separate DescriptorSetInterface
    - new pipeline interface
- DONE (spirv) Implement `std140_layout_and_size` for array of structs
- DONE (render/pipeline) FragmentOutputInterface, create framebuffers from that
- DONE (imageio) rethink read_to_vec: return own wrapper with read channels, can extract vec (read_to_buffer)
     - ImageBuffer: borrow as `&[u8]` slice
- OUTDATED (render) remove TypeId in create_info, replace with generic methods in the backend instance (can fake typeids)
    - caching now done in frontend
- DONE (imageio) subimage.{width,height} convenience methods
- OUTDATED (openimageio) next_subimage(), next_mipmap(), subimage() consumes imageinput/imageoutput
    - why consuming imageinput?
        - avoid unnecessary temporaries
            - can return a subimage+mipmap without needing a temporary
        - can use into_subimages()
    - or, iterators over subimages?
        - iterators don't work, because due to how they work, it's possible to have multiple subimages alive at the same time
            - must provide an ad-hoc API
    - OIIO 2.0 seems to be going more and more stateless
        - no need for seek_subimage
- WORKAROUND (shader/macros) investigate slow quoting of large bytecodes
      - possibly not our fault
          - report bug
      - try alternate solution: write bytecode to file, then include binary, or write a byte string

