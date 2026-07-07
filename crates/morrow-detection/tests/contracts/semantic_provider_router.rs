#[path = "semantic_provider_router/context_boundary.rs"]
mod context_boundary;
#[path = "semantic_provider_router/lifecycle.rs"]
mod lifecycle;
#[path = "semantic_provider_router/support.rs"]
mod support;
#[path = "semantic_provider_router/wording_bank.rs"]
mod wording_bank;

type TestResult = Result<(), Box<dyn std::error::Error>>;
