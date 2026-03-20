pub mod service;
pub mod traits;

pub use service::StackService;
pub use traits::{GitHubClientTrait, StackRepository, VcsClientTrait};
