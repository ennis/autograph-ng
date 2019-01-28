use crate::decode::decode_raw_instruction;
use crate::decode::DecodedInstruction;
use crate::IPtr;
use crate::Module;
use crate::Edit;

impl Module {
    pub fn edit_remove_instruction(&self, iptr: IPtr) {
        self.edits.borrow_mut().push(Edit::Remove(iptr.0));
    }

    pub fn edit_write_instruction<'a, T: DecodedInstruction<'a>>(&self, at: IPtr, t: &T) {
        let mut d = Vec::new();
        t.encode(&mut d);
        self.edits.borrow_mut().push(Edit::Insert(at.0, d))
    }

    pub fn into_vec_and_apply_edits(self) -> Vec<u32> {
        // sort descending so that lower iptrs are not invalidated
        // remove before inserts
        let mut edits = self.edits.into_inner();
        let mut data = self.data;
        edits.sort_by(|a, b| {
            let a = match a {
                Edit::Insert(p, _) => p*2,
                Edit::Remove(p) => p*2+1,    // higher, goes before insert at same iptr
            };
            let b = match b {
                Edit::Insert(p, _) => p*2,
                Edit::Remove(p) => p*2+1,
            };
            b.cmp(&a)
        });

        for e in edits {
            match e {
                Edit::Remove(i) => {
                    debug!("deleting iptr {}", i);
                    let size = {
                        let (inst, _) = decode_raw_instruction(&data[i..]).expect("invalid edit");
                        inst.word_count as usize
                    };
                    data.drain(i..(i + size));
                },
                Edit::Insert(i, words) => {
                    data.splice(i..i, words);
                }
            }
        }

        data
    }
}
