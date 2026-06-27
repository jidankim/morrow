use morrow_lib::native_bridge::{
    NativeBridgeState, TokenLookupRequest, TokenWriteRequest, MORROW_KEYCHAIN_SERVICE,
    MORROW_TOKEN_KIND,
};

const TOKEN_ENV: &str = "MORROW_KEYCHAIN_SMOKE_TOKEN";

fn main() {
    if let Err(error) = run() {
        eprintln!("keychain_smoke_status=error");
        eprintln!("keychain_smoke_error={error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let lookup = TokenLookupRequest::new(MORROW_KEYCHAIN_SERVICE, MORROW_TOKEN_KIND);
    if std::env::args().nth(1).as_deref() == Some("cleanup") {
        let deleted = NativeBridgeState::default()
            .delete_morrow_token(lookup)
            .map_err(|error| error.to_string())?;
        println!("keychain_smoke_cleanup_deleted={}", deleted.deleted);
        return Ok(());
    }

    let token = std::env::var(TOKEN_ENV)
        .map_err(|_| format!("{TOKEN_ENV} must be set by the smoke runner"))?;
    let cleanup = Cleanup::new(lookup.clone());
    let writer = NativeBridgeState::default();
    if let Err(error) = writer.delete_morrow_token(lookup.clone()) {
        println!("keychain_smoke_predelete_error={error}");
    }

    writer
        .create_morrow_token(TokenWriteRequest::new(
            MORROW_KEYCHAIN_SERVICE,
            MORROW_TOKEN_KIND,
            &token,
        ))
        .map_err(|error| error.to_string())?;

    let read = NativeBridgeState::default()
        .read_morrow_token(lookup.clone())
        .map_err(|error| error.to_string())?;
    let read_token = read
        .token
        .as_deref()
        .ok_or_else(|| "fresh production state did not read a token".to_owned())?;
    if read_token != token {
        return Err("fresh production state read a different token".to_owned());
    }

    NativeBridgeState::default()
        .delete_morrow_token(lookup.clone())
        .map_err(|error| error.to_string())?;
    let after_delete = NativeBridgeState::default()
        .read_morrow_token(lookup)
        .map_err(|error| error.to_string())?;
    if after_delete.present {
        return Err("token remained present after delete".to_owned());
    }

    println!("keychain_smoke_status=ok");
    println!("keychain_smoke_storage_surface=keychainBridge");
    println!("keychain_smoke_read_present={}", read.present);
    println!("keychain_smoke_deleted_absent={}", !after_delete.present);
    println!("keychain_smoke_token_len={}", token.len());
    drop(cleanup);
    Ok(())
}

struct Cleanup {
    lookup: TokenLookupRequest,
}

impl Cleanup {
    fn new(lookup: TokenLookupRequest) -> Self {
        Self { lookup }
    }
}

impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = NativeBridgeState::default().delete_morrow_token(self.lookup.clone());
    }
}
