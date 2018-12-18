use super::{IPtr, Module, decode::{DecodedInstruction, decode_raw_instruction}, inst::RawInstruction};

impl Module {

    pub fn edit_remove_instruction(&self, iptr: IPtr) {
        self.removals.borrow_mut().push(iptr.0);
    }

    pub fn edit_write_instruction<'a, T: DecodedInstruction<'a>>(&self, t: &T) {
        t.encode(&mut self.adds.borrow_mut());
    }

    pub fn into_vec_and_apply_edits(mut self) -> Vec<u32> {
        // sort descending so that lower iptrs are not invalidated
        let mut removals = self.removals.into_inner();
        let adds = self.adds.into_inner();
        let mut data = self.data;
        removals.sort_by(|a,b| b.cmp(a));
        for &i in removals.iter() {
            debug!("deleting iptr {}", i);
            let size = {
                let (inst,_) = decode_raw_instruction(&data[i..]).expect("invalid edit");
                inst.word_count as usize
            };
            data.drain(i..(i+size));
        }
        for &d in adds.iter() {
            data.push(d);
        }
        data
    }
}