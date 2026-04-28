use crate::Plugin;

pub trait StandalonePlugin: Plugin {
    const EVENT_QUEUE_LEN: usize = 1024;
    const MAX_BLOCK_SIZE: usize = 4096;
}
