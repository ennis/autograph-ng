use frame::graph::TaskId;
use frame::resource::{BufferId, ImageId};
use frame::LifetimeId;

use sid_vec::{Id, IdVec};

use vulkano::framebuffer::LayoutAttachmentDescription;

//--------------------------------------------------------------------------------------------------
pub struct RenderPassTag;
pub type RenderPassId = Id<RenderPassTag, u32>;

pub struct RenderPassRef<'id> {
    _lifetime: LifetimeId<'id>,
    renderpass: RenderPassId,
}

//--------------------------------------------------------------------------------------------------
pub struct AttachmentTag;
pub type AttachmentIndex = Id<AttachmentTag, u32>;

pub struct RenderPass {
    attachments: IdVec<AttachmentIndex, ImageId>,
    attachments_desc: IdVec<AttachmentIndex, LayoutAttachmentDescription>,
    tasks: Vec<TaskId>,
}

impl RenderPass {
    pub fn new() -> RenderPass {
        RenderPass {
            attachments: IdVec::new(),
            attachments_desc: IdVec::new(),
            tasks: Vec::new(),
        }
    }

    pub fn add_attachment(
        &mut self,
        img: ImageId,
        desc: LayoutAttachmentDescription,
    ) -> AttachmentIndex {
        self.attachments.push(img);
        self.attachments_desc.push(desc)
    }
}
