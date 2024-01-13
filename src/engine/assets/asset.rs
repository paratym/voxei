use downcast::{downcast, Any};

pub trait Asset: Any + Send + Sync {}
downcast!(dyn Asset);
