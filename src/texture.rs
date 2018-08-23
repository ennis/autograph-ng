use format::Format;

/// The dimensions of a texture.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum TextureDimensions {
    Tex1D,
    Tex2D,
    Tex3D,
    Tex1DArray,
    Tex2DArray,
    TexCube,
}

bitflags! {
    #[derive(Default)]
    pub struct TextureOptions: u8 {
        ///
        const SPARSE_STORAGE = 0b00000001;
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum MipMaps {
    Auto,
    Count(u8),
}

//2+2+4+4+4+2+2+1 = 21 bytes
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct TextureDesc {
    /// Texture dimensions.
    pub dimensions: TextureDimensions,
    /// Texture storage format.
    pub format: Format,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels, or array size of 1D texture arrays.
    pub height: u32,
    /// Depth in pixels, or array size of 2D texture arrays.
    pub depth: u32,
    /// Number of samples for multisample textures.
    /// 0 means that the texture will not be allocated with multisampling.
    pub sample_count: u8,
    /// Number of mipmap levels that should be allocated for this texture.
    /// See also: `get_texture_mip_map_count`
    pub mip_map_count: MipMaps,
    ///
    pub options: TextureOptions,
}

impl Default for TextureDesc {
    fn default() -> TextureDesc {
        TextureDesc {
            dimensions: TextureDimensions::Tex2D,
            format: Format::R8G8B8A8_UNORM,
            width: 0,
            height: 0,
            depth: 0,
            sample_count: 0,
            mip_map_count: MipMaps::Count(1),
            options: TextureOptions::empty(),
        }
    }
}
