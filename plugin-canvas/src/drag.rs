use std::path::PathBuf;

#[derive(Debug)]
pub enum DragOperation {
    None,
    Copy,
    Move,
}

#[derive(Debug)]
pub enum DragData {
    None,
    Files(Vec<PathBuf>),
}
