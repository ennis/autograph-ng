use super::*;

use sid_vec::ToIndex;

impl<'id> Frame<'id> {
    pub fn dump<W: Write>(&self, w: &mut W) {
        // dump resources
        writeln!(w, "--- RESOURCES ---");
        for (i, r) in self.images.iter().enumerate() {
            let name = r.name();
            writeln!(w, "Image {}(#{})", name, i);
            writeln!(w, "  dimensions........ {:?}", r.dimensions());
            writeln!(w, "  format ........... {:?}", r.format());
            writeln!(w, "  usage ............ {:?}", r.usage());
            writeln!(w);
        }
        for (i, r) in self.buffers.iter().enumerate() {
            let name = r.name();
            writeln!(w, "Buffer {}(#{})", name, i);
            writeln!(w, "  size ............. {}", r.size());
            writeln!(w, "  usage ............ {:?}", r.usage());
            writeln!(w);
        }

        writeln!(w);

        // tasks
        writeln!(w, "--- TASKS ---");
        for n in self.graph.node_indices() {
            let t = self.graph.node_weight(n).unwrap();
            writeln!(w, "{} (#{})", t.name(), n.index());
        }
        writeln!(w);

        // dependencies
        writeln!(w, "--- DEPS ---");
        for e in self.graph.edge_indices() {
            let (src, dst) = self.graph.edge_endpoints(e).unwrap();
            let src_task = self.graph.node_weight(src).unwrap();
            let dst_task = self.graph.node_weight(dst).unwrap();
            let d = self.graph.edge_weight(e).unwrap();

            match &d.barrier {
                &BarrierDetail::Image(ImageBarrier {
                    id,
                    dst_access_mask,
                    ..
                }) => {
                    writeln!(
                        w,
                        "IMAGE ACCESS {}(#{}) -> {}(#{})",
                        src_task.name(),
                        src.index(),
                        dst_task.name(),
                        dst.index()
                    );

                    writeln!(w, "  resource ......... {:08X}", id.to_index());
                    writeln!(w, "  dstAccessMask..... {:?}", dst_access_mask);
                    writeln!(w, "  srcStageMask ..... {:?}", d.src_stage_mask);
                    writeln!(w, "  dstStageMask ..... {:?}", d.dst_stage_mask);
                    //writeln!(w, "  newLayout ........ {:?}", new_layout);
                    /* if let Some(ref attachment) = attachment {
                        writeln!(w, "  index ............ {:?}", attachment.index);
                        writeln!(w, "  loadOp ........... {:?}", attachment.load_op);
                        writeln!(w, "  storeOp .......... {:?}", attachment.store_op);
                    }*/                }
                &BarrierDetail::Buffer(BufferBarrier {
                    id,
                    dst_access_mask,
                    ..
                }) => {
                    writeln!(
                        w,
                        "BUFFER ACCESS {}(#{}) -> {}(#{})",
                        src_task.name(),
                        src.index(),
                        dst_task.name(),
                        dst.index()
                    );
                    writeln!(w, "  resource ......... {:08X}", id.to_index());
                    writeln!(w, "  dstAccessMask .... {:?}", dst_access_mask);
                    writeln!(w, "  srcStageMask ..... {:?}", d.src_stage_mask);
                    writeln!(w, "  dstStageMask ..... {:?}", d.dst_stage_mask);
                }
                &BarrierDetail::Subpass(SubpassBarrier {
                    id,
                    dst_access_mask,
                    ..
                }) => {
                    writeln!(
                        w,
                        "SUBPASS {}(#{}) -> {}(#{})",
                        src_task.name(),
                        src.index(),
                        dst_task.name(),
                        dst.index()
                    );

                    writeln!(w, "  resource ......... {:08X}", id.to_index());
                    writeln!(w, "  dstAccessMask .... {:?}", dst_access_mask);
                    writeln!(w, "  srcStageMask ..... {:?}", d.src_stage_mask);
                    writeln!(w, "  dstStageMask ..... {:?}", d.dst_stage_mask);
                }
                &BarrierDetail::Sequence => {
                    writeln!(
                        w,
                        "SEQUENCE {}(#{}) -> {}(#{})",
                        src_task.name(),
                        src.index(),
                        dst_task.name(),
                        dst.index()
                    );
                }
            }
            writeln!(w);
        }
    }
}
