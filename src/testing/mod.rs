//! Utilities for testing a CouchDB application.

mod fake_server;

pub use attachment::{AttachmentBuilder, AttachmentIsSaved, AttachmentIsUnsaved};
pub use self::fake_server::FakeServer;
pub use view::{ViewIsReduced, ViewIsUnreduced, ViewResponseBuilder};
