pub fn convert_slice8_to_vec32(v8: &[u8]) -> Vec<u32> {
    let mut v32 = Vec::new();
    let iter = v8.chunks_exact(4);
    let r = iter.remainder();
    for e in iter {
        let w = [e[0], e[1], e[2], e[3]];
        v32.push(u32::from_le_bytes(w));
    }
    if !r.is_empty() {
        let mut w = [0; 4];
        w[..r.len()].copy_from_slice(r);
        v32.push(u32::from_le_bytes(w));
    }
    v32
}

pub fn convert_slice32_to_vec8(v32: &[u32]) -> Vec<u8> {
    let mut v8 = Vec::new();
    for e in v32.iter() {
        v8.extend(&e.to_le_bytes());
    }
    v8
}
