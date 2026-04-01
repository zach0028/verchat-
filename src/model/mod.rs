pub mod conversation;
pub mod message;
pub mod source;

// Ré-exports pour un accès direct : `use crate::model::Conversation;`
pub use conversation::Conversation;
pub use message::{Message, Role};
pub use source::Source;
