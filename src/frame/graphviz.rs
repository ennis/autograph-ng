//! Graphviz .dot generator.

use std::io::Write;

use ash::vk;

use super::{DependencyDetails, Frame};

fn format_pipeline_stage_mask(mask: vk::PipelineStageFlags) -> String {
    let mut out = String::new();
    if mask.subset(vk::PIPELINE_STAGE_TOP_OF_PIPE_BIT) {
        out.push_str("PIPELINE_STAGE_TOP_OF_PIPE_BIT<BR/>");
    }
    if mask.subset(vk::PIPELINE_STAGE_DRAW_INDIRECT_BIT) {
        out.push_str("PIPELINE_STAGE_DRAW_INDIRECT_BIT<BR/>");
    }
    if mask.subset(vk::PIPELINE_STAGE_VERTEX_INPUT_BIT) {
        out.push_str("PIPELINE_STAGE_VERTEX_INPUT_BIT<BR/>");
    }
    if mask.subset(vk::PIPELINE_STAGE_VERTEX_SHADER_BIT) {
        out.push_str("PIPELINE_STAGE_VERTEX_SHADER_BIT<BR/>");
    }
    if mask.subset(vk::PIPELINE_STAGE_TESSELLATION_CONTROL_SHADER_BIT) {
        out.push_str("PIPELINE_STAGE_TESSELLATION_CONTROL_SHADER_BIT<BR/>");
    }
    if mask.subset(vk::PIPELINE_STAGE_TESSELLATION_EVALUATION_SHADER_BIT) {
        out.push_str("PIPELINE_STAGE_TESSELLATION_EVALUATION_SHADER_BIT<BR/>");
    }
    if mask.subset(vk::PIPELINE_STAGE_GEOMETRY_SHADER_BIT) {
        out.push_str("PIPELINE_STAGE_GEOMETRY_SHADER_BIT<BR/>");
    }
    if mask.subset(vk::PIPELINE_STAGE_FRAGMENT_SHADER_BIT) {
        out.push_str("PIPELINE_STAGE_FRAGMENT_SHADER_BIT<BR/>");
    }
    if mask.subset(vk::PIPELINE_STAGE_EARLY_FRAGMENT_TESTS_BIT) {
        out.push_str("PIPELINE_STAGE_EARLY_FRAGMENT_TESTS_BIT<BR/>");
    }
    if mask.subset(vk::PIPELINE_STAGE_LATE_FRAGMENT_TESTS_BIT) {
        out.push_str("PIPELINE_STAGE_LATE_FRAGMENT_TESTS_BIT<BR/>");
    }
    if mask.subset(vk::PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT) {
        out.push_str("PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT<BR/>");
    }
    if mask.subset(vk::PIPELINE_STAGE_COMPUTE_SHADER_BIT) {
        out.push_str("PIPELINE_STAGE_COMPUTE_SHADER_BIT<BR/>");
    }
    if mask.subset(vk::PIPELINE_STAGE_TRANSFER_BIT) {
        out.push_str("PIPELINE_STAGE_TRANSFER_BIT<BR/>");
    }
    if mask.subset(vk::PIPELINE_STAGE_BOTTOM_OF_PIPE_BIT) {
        out.push_str("PIPELINE_STAGE_BOTTOM_OF_PIPE_BIT<BR/>");
    }
    if mask.subset(vk::PIPELINE_STAGE_HOST_BIT) {
        out.push_str("PIPELINE_STAGE_HOST_BIT<BR/>");
    }
    if mask.subset(vk::PIPELINE_STAGE_ALL_GRAPHICS_BIT) {
        out.push_str("PIPELINE_STAGE_ALL_GRAPHICS_BIT<BR/>");
    }
    if mask.subset(vk::PIPELINE_STAGE_ALL_COMMANDS_BIT) {
        out.push_str("PIPELINE_STAGE_ALL_COMMANDS_BIT<BR/>");
    }
    out
}

fn format_access_flags(flags: vk::AccessFlags) -> String {
    let mut out = String::new();
    if flags.subset(vk::ACCESS_INDIRECT_COMMAND_READ_BIT) {
        out.push_str("ACCESS_INDIRECT_COMMAND_READ_BIT<BR/>");
    }
    if flags.subset(vk::ACCESS_INDEX_READ_BIT) {
        out.push_str("ACCESS_INDEX_READ_BIT<BR/>");
    }
    if flags.subset(vk::ACCESS_VERTEX_ATTRIBUTE_READ_BIT) {
        out.push_str("ACCESS_VERTEX_ATTRIBUTE_READ_BIT<BR/>");
    }
    if flags.subset(vk::ACCESS_UNIFORM_READ_BIT) {
        out.push_str("ACCESS_UNIFORM_READ_BIT<BR/>");
    }
    if flags.subset(vk::ACCESS_INPUT_ATTACHMENT_READ_BIT) {
        out.push_str("ACCESS_INPUT_ATTACHMENT_READ_BIT<BR/>");
    }
    if flags.subset(vk::ACCESS_SHADER_READ_BIT) {
        out.push_str("ACCESS_SHADER_READ_BIT<BR/>");
    }
    if flags.subset(vk::ACCESS_SHADER_WRITE_BIT) {
        out.push_str("ACCESS_SHADER_WRITE_BIT<BR/>");
    }
    if flags.subset(vk::ACCESS_COLOR_ATTACHMENT_READ_BIT) {
        out.push_str("ACCESS_COLOR_ATTACHMENT_READ_BIT<BR/>");
    }
    if flags.subset(vk::ACCESS_COLOR_ATTACHMENT_WRITE_BIT) {
        out.push_str("ACCESS_COLOR_ATTACHMENT_WRITE_BIT<BR/>");
    }
    if flags.subset(vk::ACCESS_DEPTH_STENCIL_ATTACHMENT_READ_BIT) {
        out.push_str("ACCESS_DEPTH_STENCIL_ATTACHMENT_READ_BIT<BR/>");
    }
    if flags.subset(vk::ACCESS_DEPTH_STENCIL_ATTACHMENT_WRITE_BIT) {
        out.push_str("ACCESS_DEPTH_STENCIL_ATTACHMENT_WRITE_BIT<BR/>");
    }
    if flags.subset(vk::ACCESS_TRANSFER_READ_BIT) {
        out.push_str("ACCESS_TRANSFER_READ_BIT<BR/>");
    }
    if flags.subset(vk::ACCESS_TRANSFER_WRITE_BIT) {
        out.push_str("ACCESS_TRANSFER_WRITE_BIT<BR/>");
    }
    if flags.subset(vk::ACCESS_HOST_READ_BIT) {
        out.push_str("ACCESS_HOST_READ_BIT<BR/>");
    }
    if flags.subset(vk::ACCESS_HOST_WRITE_BIT) {
        out.push_str("ACCESS_HOST_WRITE_BIT<BR/>");
    }
    if flags.subset(vk::ACCESS_MEMORY_READ_BIT) {
        out.push_str("ACCESS_MEMORY_READ_BIT<BR/>");
    }
    if flags.subset(vk::ACCESS_MEMORY_WRITE_BIT) {
        out.push_str("ACCESS_MEMORY_WRITE_BIT<BR/>");
    }
    out
}

impl<'ctx> Frame<'ctx> {
    pub(crate) fn dump_graphviz<W: Write>(&self, w: &mut W) {
        writeln!(w, "digraph G {{");
        writeln!(
            w,
            "node [shape=box, style=filled, fontcolor=white, fontname=monospace];"
        );
        writeln!(w, "rankdir=LR;");
        //------------------ Resource nodes ------------------
        /*for (i,r) in self.resources.iter().enumerate() {
            match r.details {
                ResourceDetails::Image(ref r) => {
                    let name = self.get_resource_name(ResourceId(i as u32));
                    write!(w, "R_{} [fillcolor=navyblue,label=\"", i);
                    write!(w, "IMAGE {} ({:04})", name, i);
                    write!(w, "|{{imageType| {:?} }}", r.create_info.image_type);
                    write!(w, "|{{width | {} }}", r.create_info.extent.width);
                    write!(w, "|{{height | {} }}", r.create_info.extent.height);
                    write!(w, "|{{depth | {} }}", r.create_info.extent.depth);
                    write!(w, "|{{format | {:?} }}", r.create_info.format);
                    write!(w, "|{{usage | {:?} }}", r.create_info.usage);
                    writeln!(w, "\"];");
                },
                ResourceDetails::Buffer(ref r) => {
                    write!(w, "R_{} [fillcolor=red4 label=\"", i);
                    write!(w, "BUFFER {}", i);
                    write!(w, "|{{size | {:?} }}", r.create_info.size);
                    write!(w, "|{{usage | {:?} }}", r.create_info.usage);
                    writeln!(w, "\"];");
                }
            }
        }
        writeln!(w);*/

        //------------------ Tasks ------------------
        for n in self.graph.node_indices() {
            let t = self.graph.node_weight(n).unwrap();
            writeln!(
                w,
                "T_{} [shape=diamond, fontcolor=black, label=\"{} (#{})\"];",
                n.index(),
                t.name,
                n.index()
            );
        }
        writeln!(w);

        //------------------ Dependencies ------------------
        for e in self.graph.edge_indices() {
            let (src, dest) = self.graph.edge_endpoints(e).unwrap();
            let d = self.graph.edge_weight(e).unwrap();
            //let imported = self.

            let color_code = match &d.details {
                &DependencyDetails::Image { id, .. } => {
                    //let imported = self.images[id.0 as usize].is_imported();
                    if d.access_bits.subset(vk::ACCESS_SHADER_WRITE_BIT) {
                        "purple4"
                    } else {
                        "midnightblue"
                    }
                }
                &DependencyDetails::Attachment { id, .. } => {
                    // let imported = self.images[id.0 as usize].is_imported();
                    "darkgreen"
                }
                &DependencyDetails::Buffer { id, .. } => {
                    if d.access_bits.subset(vk::ACCESS_SHADER_WRITE_BIT) {
                        // let imported = self.images[id.0 as usize].is_imported();
                        "violetred4"
                    }
                    // written
                    else {
                        "red4"
                    } // read-only
                }
            };

            //------------------ Dependency edge ------------------
            writeln!(w, "T_{} -> D_{};", src.index(), e.index());
            writeln!(w, "D_{} -> T_{};", e.index(), dest.index());

            //------------------ Dependency node ------------------
            write!(
                w,
                "D_{} [shape=none,width=0,height=0,margin=0,label=<<FONT> \
                 <TABLE BGCOLOR=\"{}\" CELLSPACING=\"0\" ALIGN=\"LEFT\" >",
                e.index(),
                color_code
            );

            match &d.details {
                &DependencyDetails::Image { id, new_layout } => {
                    let name = self.images[id.0 as usize].name();
                    write!(
                        w,
                        "<TR><TD ALIGN=\"LEFT\" COLSPAN=\"2\"><B>IMAGE {} (#{})</B></TD></TR>",
                        name, id.0
                    );
                    write!(
                        w,
                        "<TR><TD ALIGN=\"LEFT\">accessBits</TD><TD ALIGN=\"RIGHT\">{}</TD></TR>",
                        format_access_flags(d.access_bits)
                    );
                    write!(
                        w,
                        "<TR><TD ALIGN=\"LEFT\">srcStageMask</TD><TD ALIGN=\"RIGHT\">{}</TD></TR>",
                        format_pipeline_stage_mask(d.src_stage_mask)
                    );
                    write!(
                        w,
                        "<TR><TD ALIGN=\"LEFT\">dstStageMask</TD><TD ALIGN=\"RIGHT\">{}</TD></TR>",
                        format_pipeline_stage_mask(d.dst_stage_mask)
                    );
                    write!(
                        w,
                        "<TR><TD ALIGN=\"LEFT\">newLayout</TD><TD ALIGN=\"RIGHT\">{:?}</TD></TR>",
                        new_layout
                    );
                }
                &DependencyDetails::Attachment {
                    id,
                    index,
                    ref description,
                } => {
                    let name = self.images[id.0 as usize].name();
                    write!(
                        w,
                        "<TR><TD ALIGN=\"LEFT\" COLSPAN=\"2\"><B>ATTACHMENT {} (#{})</B></TD></TR>",
                        name, id.0
                    );
                    write!(
                        w,
                        "<TR><TD ALIGN=\"LEFT\">index</TD><TD ALIGN=\"RIGHT\">{}</TD></TR>",
                        index
                    );
                    write!(
                        w,
                        "<TR><TD ALIGN=\"LEFT\">accessBits</TD><TD ALIGN=\"RIGHT\">{}</TD></TR>",
                        format_access_flags(d.access_bits)
                    );
                    write!(
                        w,
                        "<TR><TD ALIGN=\"LEFT\">srcStageMask</TD><TD ALIGN=\"RIGHT\">{}</TD></TR>",
                        format_pipeline_stage_mask(d.src_stage_mask)
                    );
                    write!(
                        w,
                        "<TR><TD ALIGN=\"LEFT\">dstStageMask</TD><TD ALIGN=\"RIGHT\">{}</TD></TR>",
                        format_pipeline_stage_mask(d.dst_stage_mask)
                    );
                    write!(
                        w,
                        "<TR><TD ALIGN=\"LEFT\">format</TD><TD ALIGN=\"RIGHT\">{:?}</TD></TR>",
                        description.format
                    );
                    write!(
                        w,
                        "<TR><TD ALIGN=\"LEFT\">loadOp</TD><TD ALIGN=\"RIGHT\">{:?}</TD></TR>",
                        description.load_op
                    );
                    write!(
                        w,
                        "<TR><TD ALIGN=\"LEFT\">storeOp</TD><TD ALIGN=\"RIGHT\">{:?}</TD></TR>",
                        description.store_op
                    );
                    write!(
                        w,
                        "<TR><TD ALIGN=\"LEFT\">finalLayout</TD><TD ALIGN=\"RIGHT\">{:?}</TD></TR>",
                        description.final_layout
                    );
                }
                &DependencyDetails::Buffer { id } => {
                    let name = self.buffers[id.0 as usize].name();
                    write!(
                        w,
                        "<TR><TD ALIGN=\"LEFT\" COLSPAN=\"2\"><B>BUFFER {} (#{})</B></TD></TR>",
                        name, id.0
                    );
                }
            }
            writeln!(w, "</TABLE></FONT>>];");
        }

        writeln!(w, "}}");
    }
}
