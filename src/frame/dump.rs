use super::*;

impl<'ctx> Frame<'ctx> {
    pub(super) fn dump<W: Write>(&self, w: &mut W) {
        // dump resources
        writeln!(w, "--- RESOURCES ---");
        for (i, r) in self.images.iter().enumerate() {
            let name = r.name();
            let (width, height, depth) = r.dimensions();
            let format = r.format();
            writeln!(w, "Image {}(#{})", name, i);
            //writeln!(w, "  imageType ........ {:?}", create_info.image_type);
            writeln!(w, "  width ............ {}", width);
            writeln!(w, "  height ........... {}", height);
            writeln!(w, "  depth ............ {}", depth);
            writeln!(w, "  format ........... {:?}", format);
            //writeln!(w, "  usage ............ {:?}", create_info.usage);
            writeln!(w);
        }
        for (i, r) in self.buffers.iter().enumerate() {
            let name = r.name();
            let size = r.size();
            writeln!(w, "Buffer {}(#{})", name, i);
            writeln!(w, "  size ............. {}", size);
            //writeln!(w, "  usage ............ {:?}", create_info.usage);
            writeln!(w);
        }

        writeln!(w);

        // tasks
        writeln!(w, "--- TASKS ---");
        for n in self.graph.node_indices() {
            let t = self.graph.node_weight(n).unwrap();
            writeln!(w, "{} (#{})", t.name, n.index());
        }
        writeln!(w);

        // dependencies
        writeln!(w, "--- DEPS ---");
        for e in self.graph.edge_indices() {
            let (src, dst) = self.graph.edge_endpoints(e).unwrap();
            let src_task = self.graph.node_weight(src).unwrap();
            let dst_task = self.graph.node_weight(dst).unwrap();
            let d = self.graph.edge_weight(e).unwrap();

            match &d.resource {
                &DependencyResource::Image(id) => {
                    writeln!(
                        w,
                        "IMAGE ACCESS {}(#{}) -> {}(#{})",
                        src_task.name,
                        src.index(),
                        dst_task.name,
                        dst.index()
                    );

                    writeln!(w, "  resource ......... {:08X}", id.0);
                    writeln!(w, "  access ........... {:?}", d.access_bits);
                    writeln!(w, "  srcStageMask ..... {:?}", d.src_stage_mask);
                    writeln!(w, "  dstStageMask ..... {:?}", d.dst_stage_mask);
                    //writeln!(w, "  newLayout ........ {:?}", new_layout);
                   /* if let Some(ref attachment) = attachment {
                        writeln!(w, "  index ............ {:?}", attachment.index);
                        writeln!(w, "  loadOp ........... {:?}", attachment.load_op);
                        writeln!(w, "  storeOp .......... {:?}", attachment.store_op);
                    }*/                }
                &DependencyResource::Buffer(id) => {
                    writeln!(
                        w,
                        "BUFFER ACCESS {}(#{}) -> {}(#{})",
                        src_task.name,
                        src.index(),
                        dst_task.name,
                        dst.index()
                    );
                    writeln!(w, "  resource ......... {:08X}", id.0);
                    writeln!(w, "  access ........... {:?}", d.access_bits);
                    writeln!(w, "  srcStageMask ..... {:?}", d.src_stage_mask);
                    writeln!(w, "  dstStageMask ..... {:?}", d.dst_stage_mask);
                }
                &DependencyResource::Sequence => {
                    writeln!(
                        w,
                        "SEQUENCE {}(#{}) -> {}(#{})",
                        src_task.name,
                        src.index(),
                        dst_task.name,
                        dst.index()
                    );
                }
            }
            writeln!(w);
        }
    }
}
