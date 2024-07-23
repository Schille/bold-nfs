use num_traits::ToPrimitive;

use super::nfs4_proto::FileAttr;

pub fn file_attrs_to_bitmap(file_attrs: &Vec<FileAttr>) -> Result<Vec<u32>, anyhow::Error> {
    let mut attrs = Vec::new();
    let mut idxs = file_attrs
        .iter()
        .map(|attr| {
            let idx = ToPrimitive::to_u32(attr).unwrap();
            idx
        })
        .collect::<Vec<u32>>();

    idxs.reverse();
    let mut segment = 0_u32;
    while !idxs.is_empty() {
        let idx = idxs.pop().unwrap();
        // println!("idx: {}", idx);
        // println!("idx.div_ceil(31) {:?}", idx.div_ceil(31));
        if (idx.div_ceil(31) as i16) - 1 > attrs.len() as i16 {
            attrs.push(segment);
            segment = 0_u32;
        }
        segment += 2_u32.pow(idx % 32);
    }
    attrs.push(segment);

    Ok(attrs)
}
