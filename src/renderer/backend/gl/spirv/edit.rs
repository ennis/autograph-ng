use super::{IPtr, Module, DecodedInstruction};

impl Module {

    pub fn remove_instruction(&self, iptr: IPtr) {
        self.removals.borrow_mut().push(iptr.0);
    }

    pub fn write_instruction<'a, T: DecodedInstruction<'a>>(&self, t: &T) {
        t.encode(&mut self.adds.borrow_mut());
    }

    pub fn into_vec_and_apply_edits(mut self) -> Vec<u32> {
        // sort descending so that lower iptrs are not invalidated
        let mut removals = self.removals.into_inner();
        let adds = self.adds.into_inner();
        let mut data = self.data;
        removals.sort_by(|a,b| b.cmp(a));
        for &i in removals.iter() {
            data.remove(i);
        }
        for &d in adds.iter() {
            data.push(d);
        }
        data
    }
}