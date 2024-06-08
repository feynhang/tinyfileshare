

#[derive(Debug, Clone)]
pub struct FileData {
    name: String,
    data: Vec<u8>,
}

impl FileData {
    pub(crate) fn empty_file(name: String) -> Self {
        Self {
            name,
            data: Vec::with_capacity(0),
        }
    }
    pub(crate) fn new(name: String, data: Vec<u8>) -> Self {
        Self { name, data }
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn data(&self) -> &[u8] {
        &self.data
    }
}
