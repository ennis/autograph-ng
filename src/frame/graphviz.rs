//! Graphviz .dot generator.

use std::io::Write;

use ash::vk;

use super::*;

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

impl<'id> Frame<'id> {
    pub fn dump_graphviz<W: Write>(
        &self,
        w: &mut W,
        ordering: Option<&[PassId]>,
        show_details: bool,
    ) {
        writeln!(w, "digraph G {{");
        writeln!(
            w,
            "node [shape=box, style=filled, fontcolor=black, fontname=monospace];"
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
        // filter tasks by assigned queues

        // present queue subgraph
        writeln!(w, "subgraph present {{");
        writeln!(w, "fontname=monospace;");
        writeln!(w, "label=\"Present queue\";");
        writeln!(w, "labeljust=\"r\";");
        writeln!(
            w,
            "node [shape=diamond, fontcolor=black, style=filled, fillcolor=\"brown1\"];"
        );
        self.graph
            .node_indices()
            .map(|n| (n.index(), self.graph.node_weight(n).unwrap()))
            .filter(|(_, t)| t.kind() == TaskKind::Present)
            .for_each(|(i, t)| {
                writeln!(w, "T_{} [label=\"{} (ID:{})\"];", i, t.name(), i);
            });
        writeln!(w, "}}");

        // graphics queue subgraph
        writeln!(w, "subgraph default {{");
        writeln!(w, "fontname=monospace;");
        writeln!(w, "label=\"Default queue\";");
        writeln!(w, "labeljust=\"r\";");
        writeln!(
            w,
            "node [shape=diamond, fontcolor=black, style=filled, fillcolor=\"goldenrod1\"];"
        );
        self.graph
            .node_indices()
            .map(|n| (n.index(), self.graph.node_weight(n).unwrap()))
            .filter(|(_, t)| t.kind() == TaskKind::Graphics)
            .for_each(|(i, t)| {
                writeln!(w, "T_{} [label=\"{} (ID:{})\"];", i, t.name(), i);
            });
        writeln!(w, "}}");

        // async compute subgraph
        writeln!(w, "subgraph compute {{");
        writeln!(w, "fontname=monospace;");
        writeln!(w, "label=\"Async compute\";");
        writeln!(w, "labeljust=\"r\";");
        writeln!(
            w,
            "node [shape=diamond, fontcolor=black, style=filled, fillcolor=\"palegreen\"];"
        );
        self.graph
            .node_indices()
            .map(|n| (n.index(), self.graph.node_weight(n).unwrap()))
            .filter(|(_, t)| t.kind() == TaskKind::Compute)
            .for_each(|(i, t)| {
                writeln!(w, "T_{} [label=\"{} (ID:{})\"];", i, t.name(), i);
            });
        writeln!(w, "}}");

        //------------------ Ordering ------------------
        if let Some(ordering) = ordering {
            for t in ordering.windows(2) {
                writeln!(w, "T_{} -> T_{} [style=invis];", t[0].index(), t[1].index());
            }
        }

        //------------------ Dependencies ------------------
        for e in self.graph.edge_indices() {
            let (src, dest) = self.graph.edge_endpoints(e).unwrap();
            let d = self.graph.edge_weight(e).unwrap();
            //let imported = self.

            let color_code = match &d.barrier {
                &BarrierDetail::Image(ImageBarrier {
                    id,
                    dst_access_mask,
                    ..
                }) => {
                    let transient = self.images[id].is_transient();
                    if !transient {
                        if dst_access_mask.intersects(
                            vk::ACCESS_COLOR_ATTACHMENT_READ_BIT
                                | vk::ACCESS_COLOR_ATTACHMENT_WRITE_BIT,
                        ) {
                            "mediumpurple2"
                        } else if dst_access_mask.intersects(vk::ACCESS_SHADER_WRITE_BIT) {
                            "mediumpurple1"
                        } else {
                            "plum"
                        }
                    } else {
                        if dst_access_mask.intersects(
                            vk::ACCESS_COLOR_ATTACHMENT_READ_BIT
                                | vk::ACCESS_COLOR_ATTACHMENT_WRITE_BIT,
                        ) {
                            "lightblue3"
                        } else if dst_access_mask.intersects(vk::ACCESS_SHADER_WRITE_BIT) {
                            "mediumpurple1"
                        } else {
                            "lightcyan1"
                        }
                    }
                }
                &BarrierDetail::Buffer(BufferBarrier {
                    id,
                    dst_access_mask,
                    ..
                }) => {
                    if dst_access_mask.intersects(vk::ACCESS_SHADER_WRITE_BIT) {
                        // let imported = self.images[id.0 as usize].is_imported();
                        "violetred4"
                    } else {
                        "mediumpurple1"
                    }
                }
                _ => "",
            };

            //------------------ Dependency edge ------------------
            match &d.barrier {
                &BarrierDetail::Sequence => {
                    // no associated resource
                    writeln!(
                        w,
                        "T_{} -> T_{} [constrain=false, style=dotted];",
                        src.index(),
                        dest.index()
                    );
                }
                _ => {
                    // there is an associated resource
                    writeln!(w, "T_{} -> D_{} [constrain=false];", src.index(), e.index());
                    writeln!(w, "D_{} -> T_{};", e.index(), dest.index());
                    write!(
                        w,
                        "D_{} [shape=none,width=0,height=0,margin=0,label=<<FONT> \
                <TABLE BORDER=\"0\" CELLBORDER=\"1\" BGCOLOR=\"{}\" CELLSPACING=\"0\" ALIGN=\"LEFT\" ><TR><TD>I</TD></TR>",
                        e.index(),
                        color_code
                    );

                    /*write!(
                            w,
                            "D_{} [shape=none,width=0,height=0,margin=0,label=<<FONT> \
                    <TABLE BORDER=\"0\" CELLBORDER=\"1\" BGCOLOR=\"{}\" CELLSPACING=\"0\" ALIGN=\"LEFT\" >",
                            e.index(),
                            color_code
                        );
                    
                        //------------------ Dependency node ------------------
                        match &d.details {
                            &DependencyDetails::Image {
                                id,
                                new_layout,
                                usage,
                                ref attachment,
                            } => {
                                let img = &self.images[id.0 as usize];
                                if let Some(_) = attachment {
                                    write!(
                                        w,
                                        "<TR><TD ALIGN=\"LEFT\" COLSPAN=\"2\">Attachment {} (ID:{})<BR/>{}</TD></TR>",
                                        img.name(), id.0, if img.is_imported() { "Imported" } else { "" }
                                    );
                                } else {
                                    write!(
                                        w,
                                        "<TR><TD ALIGN=\"LEFT\" COLSPAN=\"2\">Image {} (ID:{})<BR/>{}</TD></TR>",
                                        img.name(), id.0, if img.is_imported() { "Imported" } else { "" }
                                    );
                                }
                                if show_details {
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
                                    if let Some(ref attachment) = attachment {
                                        write!(
                                            w,
                                            "<TR><TD ALIGN=\"LEFT\">format</TD><TD ALIGN=\"RIGHT\">{:?}</TD></TR>",
                                            attachment.description.format
                                        );
                                        write!(
                                            w,
                                            "<TR><TD ALIGN=\"LEFT\">loadOp</TD><TD ALIGN=\"RIGHT\">{:?}</TD></TR>",
                                            attachment.description.load_op
                                        );
                                        write!(
                                            w,
                                            "<TR><TD ALIGN=\"LEFT\">storeOp</TD><TD ALIGN=\"RIGHT\">{:?}</TD></TR>",
                                            attachment.description.store_op
                                        );
                                        write!(
                                            w,
                                            "<TR><TD ALIGN=\"LEFT\">finalLayout</TD><TD ALIGN=\"RIGHT\">{:?}</TD></TR>",
                                            attachment.description.final_layout
                                        );
                                    }
                                }
                            }
                            &DependencyDetails::Buffer { id, .. } => {
                                let name = self.buffers[id.0 as usize].name();
                                write!(
                                    w,
                                    "<TR><TD ALIGN=\"LEFT\" COLSPAN=\"2\">Buffer {} (ID:{})</TD></TR>",
                                    name, id.0
                                );
                            }
                            _ => unreachable!(),
                        }*/
                    writeln!(w, "</TABLE></FONT>>];");
                }
            }
        }

        writeln!(w, "}}");
    }
}
