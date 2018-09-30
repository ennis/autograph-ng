use super::*;

use sid_vec::ToIndex;

impl<'ctx> Frame<'ctx> {
    pub(super) fn dump<W: Write>(&self, w: &mut W) {
        // dump resources
        writeln!(w, "--- RESOURCES ---");
        for (i, r) in self.resources.images.iter().enumerate() {
            let name = r.name();
            let create_info = r.create_info();
            writeln!(w, "Image {}(#{})", name, i);
            writeln!(w, "  imageType ........ {:?}", create_info.image_type);
            writeln!(w, "  width ............ {}", create_info.extent.width);
            writeln!(w, "  height ........... {}", create_info.extent.height);
            writeln!(w, "  depth ............ {}", create_info.extent.depth);
            writeln!(w, "  format ........... {:?}", create_info.format);
            writeln!(w, "  usage ............ {:?}", create_info.usage);
            writeln!(w);
        }
        for (i, r) in self.resources.buffers.iter().enumerate() {
            let name = r.name();
            let create_info = r.create_info();
            writeln!(w, "Buffer {}(#{})", name, i);
            writeln!(w, "  size ............. {}", create_info.size);
            writeln!(w, "  usage ............ {:?}", create_info.usage);
            writeln!(w);
        }

        writeln!(w);

        // tasks
        writeln!(w, "--- TASKS ---");
        for n in self.graph.0.node_indices() {
            let t = self.graph.0.node_weight(n).unwrap();
            writeln!(w, "{} (#{})", t.name, n.index());
        }
        writeln!(w);

        // dependencies
        writeln!(w, "--- DEPS ---");
        for e in self.graph.0.edge_indices() {
            let (src, dst) = self.graph.0.edge_endpoints(e).unwrap();
            let src_task = self.graph.0.node_weight(src).unwrap();
            let dst_task = self.graph.0.node_weight(dst).unwrap();
            let d = self.graph.0.edge_weight(e).unwrap();

            match &d.barrier {
                &BarrierDetail::Image(ImageBarrier {
                    id,
                    dst_access_mask,
                    ..
                }) => {
                    writeln!(
                        w,
                        "IMAGE ACCESS {}(#{}) -> {}(#{})",
                        src_task.name,
                        src.index(),
                        dst_task.name,
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
                        src_task.name,
                        src.index(),
                        dst_task.name,
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
                        src_task.name,
                        src.index(),
                        dst_task.name,
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
