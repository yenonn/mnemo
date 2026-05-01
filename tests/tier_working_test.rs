use mnemo::tier::WorkingBuffer;

#[test]
fn test_ring_buffer_overflow() {
    let mut buf = WorkingBuffer::with_capacity(3);
    buf.push("m1", "User likes A");
    buf.push("m2", "User likes B");
    buf.push("m3", "User likes C");
    buf.push("m4", "User likes D"); // overflows, m1 dropped

    assert_eq!(buf.len(), 3);
    let entries = buf.entries();
    assert_eq!(entries[0].id, "m2");
    assert_eq!(entries[2].id, "m4");
}
