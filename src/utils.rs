use std::convert::identity;

pub fn chunk_with_size(s: &str) -> Vec<String> {
    const MAX_CHUNK_SIZE: usize = 2048;

    let mut chunks = vec!["".to_owned()];
    for line in s.lines() {
        if chunks.last().unwrap().len() + line.len() < MAX_CHUNK_SIZE {
            chunks
                .last_mut()
                .unwrap()
                .push_str(format!("{}\n", line).as_str());
        } else {
            chunks.push(format!("{}\n", line))
        }
    }
    chunks
}
