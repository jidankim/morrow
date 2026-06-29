#[path = "support/openai_provider.rs"]
pub mod openai_provider;
#[path = "support/provider.rs"]
pub mod provider;

mod support {
    pub use crate::openai_provider;
    pub use crate::provider;
}

mod native_provider {
    mod openai_provider_cases;
}
