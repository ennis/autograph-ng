/*use crate::{
    api::{types::*, Gl},
    format::GlFormatInfo,
};
use autograph_render::pipeline::VertexInputAttributeDescription;

pub fn create_vertex_array_object(gl: &Gl, attribs: &[VertexInputAttributeDescription]) -> GLuint {
    let mut vao = 0;
    unsafe {
        gl.CreateVertexArrays(1, &mut vao);
    }

    for a in attribs.iter() {
        unsafe {
            gl.EnableVertexArrayAttrib(vao, a.location);
            let fmtinfo = a.format.get_format_info();
            let normalized = fmtinfo.is_normalized() as u8;
            let size = fmtinfo.num_components() as i32;
            let glfmt = GlFormatInfo::from_format(a.format);
            let ty = glfmt.upload_ty;

            gl.VertexArrayAttribFormat(vao, a.location, size, ty, normalized, a.offset);
            gl.VertexArrayAttribBinding(vao, a.location, a.binding);
        }
    }

    vao
}
*/
