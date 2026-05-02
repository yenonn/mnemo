use mnemo::tier::{WorkingBuffer, WorkingEntry};

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

#[test]
fn test_working_buffer_empty() {
    let buf = WorkingBuffer::with_capacity(5);
    assert!(buf.is_empty());
    assert_eq!(buf.len(), 0);
}

#[test]
fn test_working_buffer_clear() {
    let mut buf = WorkingBuffer::with_capacity(5);
    buf.push("m1", "content");
    assert_eq!(buf.len(), 1);
    buf.clear();
    assert!(buf.is_empty());
    assert_eq!(buf.len(), 0);
}

#[test]
fn test_working_buffer_drain() {
    let mut buf = WorkingBuffer::with_capacity(5);
    buf.push("m1", "content1");
    buf.push("m2", "content2");

    let drained: Vec<WorkingEntry> = buf.drain();
    assert_eq!(drained.len(), 2);
    assert_eq!(drained[0].id, "m1");
    assert_eq!(drained[1].id, "m2");

    assert!(buf.is_empty());
    assert_eq!(buf.len(), 0);
}

#[test]
fn test_working_buffer_entry_fields() {
    let mut buf = WorkingBuffer::with_capacity(5);
    buf.push("m1", "hello");

    let entries = buf.entries();
    assert_eq!(entries[0].id, "m1");
    assert_eq!(entries[0].content, "hello");
    assert!(entries[0].timestamp > 0);
}
