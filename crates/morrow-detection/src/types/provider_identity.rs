use super::{
    bounded, DetectionError, ListReminderProfile, ListReminderProfileId, ListReminderProfileVersion,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderIdentity {
    pub provider_id: String,
    pub model_id: String,
    pub prompt_version: String,
    pub profile_id: ListReminderProfileId,
    pub profile_version: ListReminderProfileVersion,
}

impl ProviderIdentity {
    pub fn new(
        provider_id: &str,
        model_id: &str,
        prompt_version: &str,
    ) -> Result<Self, DetectionError> {
        Self::new_with_profile(
            provider_id,
            model_id,
            prompt_version,
            &ListReminderProfile::disabled(),
        )
    }

    pub fn new_with_profile(
        provider_id: &str,
        model_id: &str,
        prompt_version: &str,
        profile: &ListReminderProfile,
    ) -> Result<Self, DetectionError> {
        Ok(Self {
            provider_id: bounded("provider_id", provider_id, 80)?,
            model_id: bounded("model_id", model_id, 120)?,
            prompt_version: bounded("prompt_version", prompt_version, 80)?,
            profile_id: profile.profile_id,
            profile_version: profile.profile_version,
        })
    }
}
