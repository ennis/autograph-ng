pub use gfx2_shader_macros::{static_shader, include_combined_shader};

pub trait Shader
{
    fn spirv(&self) -> &[u32];
}

pub struct StaticShader
{
    pub spirv: &'static [u32]
}

impl Shader for StaticShader
{
    fn spirv(&self) -> &[u32] {
        self.spirv
    }
}

pub struct CombinedShaders {
    pub vertex: Option<StaticShader>,
    pub fragment: Option<StaticShader>,
    pub geometry: Option<StaticShader>,
    pub tess_control: Option<StaticShader>,
    pub tess_eval: Option<StaticShader>,
    pub compute: Option<StaticShader>,
}
