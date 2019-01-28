# TODO list

### Meta
- (meta) Documentation

### Outstanding
- (spirv) Unit-tests for layout calculation
OK (spirv) Implement `std140_layout_and_size` for array of structs
- (spirv) Parse constant values for array sizes (panic on specialization for now)
- (shader-macros) Cleanup autograph_shader_macros (keep only include_shader, nuke the preprocessor)
- (render/pipeline) Interface items directly in PipelineInterface, without needing a separate DescriptorSetInterface
- (render/backend) backend calls: return Result<> instead of panicking
- (render/pipeline/validation) Check vertex input interfaces
- (render/pipeline/validation) Precise errors
- (render/pipeline) FragmentOutputInterface, create framebuffers from that
- (render) add an (unsafe) API to create a pipeline and skip validation
- (render/macros) error msg instead of panic on non-repr(C) structs

### Enhancements
- (render/validation) accept structs with single member in place of just the member