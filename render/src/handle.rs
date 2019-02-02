use std::marker::PhantomData;

/// Trait implemented by backend swapchain objects.
#[repr(transparent)]
#[derive(Copy,Clone,Debug)]
pub struct Swapchain<'a>(pub usize, pub PhantomData<&'a ()>);

/// Trait implemented by backend buffer objects.
#[repr(transparent)]
#[derive(Copy,Clone,Debug)]
pub struct Buffer<'a>(pub usize, pub PhantomData<&'a ()>);

/// Trait implemented by backend image objects.
#[repr(transparent)]
#[derive(Copy,Clone,Debug)]
pub struct Image<'a>(pub usize, pub PhantomData<&'a ()>);

/// Trait implemented by backend shader module objects.
#[repr(transparent)]
#[derive(Copy,Clone,Debug)]
pub struct ShaderModule<'a>(pub usize, pub PhantomData<&'a ()>);

/// Trait implemented by backend graphics pipeline objects.
#[repr(transparent)]
#[derive(Copy,Clone,Debug)]
pub struct GraphicsPipeline<'a>(pub usize, pub PhantomData<&'a ()>);

#[repr(transparent)]
#[derive(Copy,Clone,Debug)]
pub struct PipelineSignature<'a>(pub usize, pub PhantomData<&'a ()>);

#[repr(transparent)]
#[derive(Copy,Clone,Debug)]
pub struct PipelineArguments<'a>(pub usize, pub PhantomData<&'a ()>);

/// A reference to host data that is used in pipeline arguments.
#[repr(transparent)]
#[derive(Copy,Clone,Debug)]
pub struct HostReference<'a>(pub usize, pub PhantomData<&'a ()>);

#[repr(transparent)]
#[derive(Copy,Clone,Debug)]
pub struct Arena<'a>(pub usize, pub PhantomData<&'a ()>);
