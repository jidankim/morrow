mod public_chat_id {
    include!("../../src/native_bridge/public_chat_id.rs");
}

mod scan_privacy {
    include!("../../src/native_bridge/scan_privacy.rs");
}

#[path = "outcome_plan_support.rs"]
mod outcome_plan_support;

mod scan {
    pub enum ScanSelectedChatsError {
        Detection(String),
        Messages(String),
    }

    impl std::fmt::Debug for ScanSelectedChatsError {
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::Detection(message) => {
                    formatter.debug_tuple("Detection").field(message).finish()
                }
                Self::Messages(message) => {
                    formatter.debug_tuple("Messages").field(message).finish()
                }
            }
        }
    }

    mod outcome_plan {
        include!("../../src/native_bridge/scan/outcome_plan.rs");
    }

    mod title_privacy {
        include!("outcome_plan/title_privacy.rs");
    }
}
