
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum SamplerAddressMode {
    Clamp,
    Mirror,
    Wrap,
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum SamplerFilter {
    Nearest,
    Linear,
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum SamplerMipmapMode {
    Nearest,
    Linear,
}

// 2D sampler
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub struct SamplerDesc {
    pub addr_u: SamplerAddressMode,
    pub addr_v: SamplerAddressMode,
    pub addr_w: SamplerAddressMode,
    pub min_filter: SamplerFilter,
    pub mag_filter: SamplerFilter,
    pub mipmap_mode: SamplerMipmapMode,
}