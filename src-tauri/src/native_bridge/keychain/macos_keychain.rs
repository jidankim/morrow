use std::string::FromUtf8Error;

use core_foundation::base::TCFType;
use security_framework::{
    base::Error as SecurityFrameworkError,
    os::macos::{
        keychain::{SecKeychain, SecPreferencesDomain},
        keychain_item::SecKeychainItem,
        passwords::find_generic_password,
    },
};
use security_framework_sys::keychain_item::SecKeychainItemDelete;

use super::{KeychainBridgeError, TokenLookupRequest};

const ERR_SEC_SUCCESS: i32 = 0;
const ERR_SEC_ITEM_NOT_FOUND: i32 = -25300;

pub fn store_password(lookup: &TokenLookupRequest, token: &str) -> Result<(), KeychainBridgeError> {
    user_keychain()?
        .set_generic_password(&lookup.service, &lookup.token_kind, token.as_bytes())
        .map_err(|error| security_storage_error("store", error))
}

pub fn read_password(lookup: &TokenLookupRequest) -> Result<Option<String>, KeychainBridgeError> {
    let keychain = user_keychain()?;
    match find_generic_password(Some(&[keychain]), &lookup.service, &lookup.token_kind) {
        Ok((password, _item)) => String::from_utf8(password.to_vec())
            .map(Some)
            .map_err(invalid_utf8_error),
        Err(error) if error.code() == ERR_SEC_ITEM_NOT_FOUND => Ok(None),
        Err(error) => Err(security_storage_error("read", error)),
    }
}

pub fn delete_password(lookup: &TokenLookupRequest) -> Result<bool, KeychainBridgeError> {
    let keychain = user_keychain()?;
    match find_generic_password(Some(&[keychain]), &lookup.service, &lookup.token_kind) {
        Ok((_password, item)) => delete_keychain_item(item),
        Err(error) if error.code() == ERR_SEC_ITEM_NOT_FOUND => Ok(false),
        Err(error) => Err(security_storage_error("delete", error)),
    }
}

fn delete_keychain_item(item: SecKeychainItem) -> Result<bool, KeychainBridgeError> {
    // SAFETY: [Category 8 - FFI Boundary UB]
    // `item` is a live Security.framework object returned by
    // `SecKeychainFindGenericPassword` under the create rule. Its CFTypeRef is a
    // valid SecKeychainItemRef for the duration of this call, and the OSStatus is
    // checked before reporting a successful delete.
    let status = unsafe { SecKeychainItemDelete(item.as_CFTypeRef() as *mut _) };
    checked_delete_result(status)
}

fn checked_delete_result(status: i32) -> Result<bool, KeychainBridgeError> {
    if status == ERR_SEC_SUCCESS {
        return Ok(true);
    }

    Err(security_storage_status_error("delete", status))
}

fn user_keychain() -> Result<SecKeychain, KeychainBridgeError> {
    SecKeychain::default_for_domain(SecPreferencesDomain::User)
        .map_err(|error| security_storage_error("open user keychain", error))
}

fn security_storage_error(action: &str, error: SecurityFrameworkError) -> KeychainBridgeError {
    KeychainBridgeError::storage_unavailable(format!(
        "Morrow Keychain {action} failed with OSStatus {}: {error}",
        error.code()
    ))
}

fn security_storage_status_error(action: &str, status: i32) -> KeychainBridgeError {
    let error = SecurityFrameworkError::from_code(status);
    security_storage_error(action, error)
}

fn invalid_utf8_error(_error: FromUtf8Error) -> KeychainBridgeError {
    KeychainBridgeError::storage_unavailable("Morrow Keychain token was not valid UTF-8")
}

#[cfg(test)]
mod tests {
    use crate::native_bridge::KeychainErrorCode;

    use super::*;

    #[test]
    fn checked_delete_result_reports_deleted_only_after_success_status() {
        let success = checked_delete_result(0);
        let failure = checked_delete_result(-60008);

        assert_eq!(success, Ok(true));
        let error = failure.expect_err("nonzero OSStatus must not report deleted=true");
        assert_eq!(error.code(), KeychainErrorCode::StorageUnavailable);
        assert!(error.to_string().contains("-60008"));
    }
}
