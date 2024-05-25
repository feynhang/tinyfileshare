#[derive(Debug, Clone)]
pub struct FileData {
    name: String,
    data: Option<Vec<u8>>,
}

impl FileData {
    pub(crate) fn new(name: String, data: Vec<u8>) -> Self {
        Self { name, data: Some(data) }
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn data(&self) -> Option<&[u8]> {
        match &self.data {
            Some(b) => Some(b),
            None => None,
        }
    }
}
