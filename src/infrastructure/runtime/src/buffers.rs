/// Keep ordinary stream buffers reusable without retaining an unusually large
/// frame for the lifetime of an idle connection. Call before yielding output.
pub fn reset_stream_buffer(buffer: &mut Vec<u8>) {
    const MAX_RETAINED_BYTES: usize = 64 * 1024;
    if buffer.capacity() > MAX_RETAINED_BYTES {
        *buffer = Vec::new();
    } else {
        buffer.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn large_frames_do_not_raise_the_retained_baseline_for_later_small_frames() {
        let mut buffer = Vec::with_capacity(8192);
        buffer.extend_from_slice(b"first line");
        reset_stream_buffer(&mut buffer);
        assert_eq!(buffer.capacity(), 8192);
        buffer.resize(1024 * 1024, b'x');
        reset_stream_buffer(&mut buffer);
        assert!(buffer.is_empty());
        assert!(buffer.capacity() <= 64 * 1024);
        buffer.extend_from_slice(b"next line");
        assert_eq!(buffer, b"next line");
        reset_stream_buffer(&mut buffer);
        assert!(buffer.capacity() <= 64 * 1024);
    }
}
