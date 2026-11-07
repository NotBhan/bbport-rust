//! A little-endian-agnostic serialization archive, ported from
//! `legacy/gpu/shadps4/common/serdes.h`.
//!
//! The C++ copies raw objects with `memcpy`; the port exposes the primitive writers/readers
//! plus byte-slice forms for structs that are serialized field by field.

/// `Serialization::Archive`.
#[derive(Debug, Clone, Default)]
pub struct Archive {
    offset: usize,
    container: Vec<u8>,
}

impl Archive {
    /// `Archive()`.
    pub fn new() -> Self {
        Self {
            offset: 0,
            container: Vec::new(),
        }
    }

    /// `Archive(std::vector<u8>&&)`.
    pub fn from_bytes(container: Vec<u8>) -> Self {
        Self {
            offset: 0,
            container,
        }
    }

    /// `Alloc`.
    pub fn alloc(&mut self, size: usize) {
        self.container.resize(size, 0);
    }

    /// `Grow`.
    pub fn grow(&mut self, size: usize) {
        self.container.resize(self.container.len() + size, 0);
    }

    /// `Merge`.
    pub fn merge(&mut self, other: &Archive) {
        self.container.extend_from_slice(&other.container);
        self.offset = self.container.len();
    }

    /// `SizeBytes`.
    pub fn size_bytes(&self) -> usize {
        self.container.len()
    }

    /// `CurrPtr`.
    pub fn curr_ptr(&self) -> &[u8] {
        &self.container[self.offset..]
    }

    /// The mutable `CurrPtr`.
    pub fn curr_ptr_mut(&mut self) -> &mut [u8] {
        &mut self.container[self.offset..]
    }

    /// `Advance`.
    pub fn advance(&mut self, size: usize) {
        assert!(
            self.offset + size <= self.container.len(),
            "Invalid or corrupted deserialization container/shader cache"
        );
        self.offset += size;
    }

    /// `TakeOff`.
    pub fn take_off(&mut self) -> Vec<u8> {
        self.offset = 0;
        std::mem::take(&mut self.container)
    }

    /// `IsEoS`.
    pub fn is_eos(&self) -> bool {
        self.offset >= self.container.len()
    }

    /// The current read/write position.
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// The bytes written so far.
    pub fn bytes(&self) -> &[u8] {
        &self.container
    }
}

/// `Serialization::Writer`.
pub struct Writer<'a> {
    archive: &'a mut Archive,
}

impl<'a> Writer<'a> {
    /// `Writer(Archive&)`.
    pub fn new(archive: &'a mut Archive) -> Self {
        Self { archive }
    }

    /// `Write(ptr, size)`.
    pub fn write_bytes(&mut self, bytes: &[u8]) {
        let size = bytes.len();
        if self.archive.offset + size >= self.archive.container.len() {
            self.archive.grow(size);
        }
        self.archive.curr_ptr_mut()[..size].copy_from_slice(bytes);
        self.archive.advance(size);
    }

    /// `Write(const T&)` for a primitive.
    pub fn write_u8(&mut self, value: u8) {
        self.write_bytes(&[value]);
    }

    pub fn write_u16(&mut self, value: u16) {
        self.write_bytes(&value.to_ne_bytes());
    }

    pub fn write_u32(&mut self, value: u32) {
        self.write_bytes(&value.to_ne_bytes());
    }

    pub fn write_u64(&mut self, value: u64) {
        self.write_bytes(&value.to_ne_bytes());
    }

    pub fn write_i32(&mut self, value: i32) {
        self.write_bytes(&value.to_ne_bytes());
    }

    pub fn write_f32(&mut self, value: f32) {
        self.write_bytes(&value.to_ne_bytes());
    }

    pub fn write_f64(&mut self, value: f64) {
        self.write_bytes(&value.to_ne_bytes());
    }

    /// `Write(const std::string&)`.
    pub fn write_str(&mut self, value: &str) {
        self.write_u64(value.len() as u64);
        self.write_bytes(value.as_bytes());
    }

    /// `Write` for a container: the length then each element.
    pub fn write_slice<T>(&mut self, values: &[T], write: impl Fn(&mut Self, &T))
    where
        Self: Sized,
    {
        self.write_u64(values.len() as u64);
        for value in values {
            write(self, value);
        }
    }

    /// The archive being written.
    pub fn archive(&mut self) -> &mut Archive {
        self.archive
    }
}

/// `Serialization::Reader`.
pub struct Reader<'a> {
    archive: &'a mut Archive,
}

impl<'a> Reader<'a> {
    /// `Reader(Archive&)`.
    pub fn new(archive: &'a mut Archive) -> Self {
        Self { archive }
    }

    /// `Read(ptr, size)`.
    pub fn read_bytes(&mut self, bytes: &mut [u8]) {
        let size = bytes.len();
        assert!(
            self.archive.offset + size <= self.archive.container.len(),
            "Invalid or corrupted deserialization container/shader cache"
        );
        bytes.copy_from_slice(&self.archive.curr_ptr()[..size]);
        self.archive.advance(size);
    }

    pub fn read_u8(&mut self) -> u8 {
        let mut bytes = [0u8; 1];
        self.read_bytes(&mut bytes);
        bytes[0]
    }

    pub fn read_u16(&mut self) -> u16 {
        let mut bytes = [0u8; 2];
        self.read_bytes(&mut bytes);
        u16::from_ne_bytes(bytes)
    }

    pub fn read_u32(&mut self) -> u32 {
        let mut bytes = [0u8; 4];
        self.read_bytes(&mut bytes);
        u32::from_ne_bytes(bytes)
    }

    pub fn read_u64(&mut self) -> u64 {
        let mut bytes = [0u8; 8];
        self.read_bytes(&mut bytes);
        u64::from_ne_bytes(bytes)
    }

    pub fn read_i32(&mut self) -> i32 {
        let mut bytes = [0u8; 4];
        self.read_bytes(&mut bytes);
        i32::from_ne_bytes(bytes)
    }

    pub fn read_f32(&mut self) -> f32 {
        let mut bytes = [0u8; 4];
        self.read_bytes(&mut bytes);
        f32::from_ne_bytes(bytes)
    }

    pub fn read_f64(&mut self) -> f64 {
        let mut bytes = [0u8; 8];
        self.read_bytes(&mut bytes);
        f64::from_ne_bytes(bytes)
    }

    /// `Read(std::string&)`.
    pub fn read_str(&mut self) -> String {
        let length = self.read_u64() as usize;
        let mut bytes = vec![0u8; length];
        self.read_bytes(&mut bytes);
        String::from_utf8_lossy(&bytes).into_owned()
    }

    /// `Read` for a container: the length then each element.
    pub fn read_slice<T>(&mut self, read: impl Fn(&mut Self) -> T) -> Vec<T> {
        let length = self.read_u64() as usize;
        let mut values = Vec::with_capacity(length);
        for _ in 0..length {
            values.push(read(self));
        }
        values
    }

    /// The archive being read.
    pub fn archive(&mut self) -> &mut Archive {
        self.archive
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primitives_and_strings_round_trip() {
        let mut archive = Archive::new();
        {
            let mut writer = Writer::new(&mut archive);
            writer.write_u32(0x1122_3344);
            writer.write_u64(0x5566_7788_99aa_bbcc);
            writer.write_f32(1.5);
            writer.write_str("shader");
        }
        let bytes = archive.take_off();
        let mut archive = Archive::from_bytes(bytes);
        let mut reader = Reader::new(&mut archive);
        assert_eq!(reader.read_u32(), 0x1122_3344);
        assert_eq!(reader.read_u64(), 0x5566_7788_99aa_bbcc);
        assert_eq!(reader.read_f32(), 1.5);
        assert_eq!(reader.read_str(), "shader");
        assert!(archive.is_eos());
    }

    #[test]
    fn a_slice_writes_its_length_and_elements() {
        let mut archive = Archive::new();
        {
            let mut writer = Writer::new(&mut archive);
            let values = [1u32, 2, 3];
            writer.write_slice(&values, |writer, value| writer.write_u32(*value));
        }
        let bytes = archive.take_off();
        let mut archive = Archive::from_bytes(bytes);
        let mut reader = Reader::new(&mut archive);
        let values = reader.read_slice(|reader| reader.read_u32());
        assert_eq!(values, vec![1, 2, 3]);
    }

    #[test]
    fn merging_and_taking_the_buffer_move_the_offset() {
        let mut first = Archive::new();
        Writer::new(&mut first).write_u32(7);
        let mut second = Archive::new();
        Writer::new(&mut second).write_u32(8);
        first.merge(&second);
        assert_eq!(first.offset(), first.size_bytes());
        let bytes = first.take_off();
        assert_eq!(bytes.len(), 8);
        assert!(first.is_eos());
        let mut restored = Archive::from_bytes(bytes);
        assert_eq!(Reader::new(&mut restored).read_u32(), 7);
    }
}
