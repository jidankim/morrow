use std::path::Path;

use super::{
    codex_auth, eventkit_proposal, scan, CodexExecRunner, CodexProvider,
    CodexProviderAuthReadiness, ProcessCodexExecRunner, ProposalReplayAdapter,
    ScanSelectedChatsError, ScanSelectedChatsRequest, ScanSelectedChatsResult,
};

pub struct ProductionScanCodexDependencies<'a, R, A> {
    pub auth_readiness: CodexProviderAuthReadiness,
    pub codex_runner: &'a R,
    pub proposal_adapter: &'a A,
}

pub fn scan_selected_chats_at(
    request: ScanSelectedChatsRequest,
    store_path: &Path,
    messages_db_path: &Path,
) -> Result<ScanSelectedChatsResult, ScanSelectedChatsError> {
    request.validate_local_diagnostics_retention()?;
    let proposal_adapter = eventkit_proposal::EventKitProposalBridge;
    let codex_runner = ProcessCodexExecRunner;
    scan_selected_chats_at_with_codex_dependencies(
        request,
        store_path,
        messages_db_path,
        ProductionScanCodexDependencies {
            auth_readiness: codex_auth::probe_codex_provider_auth(),
            codex_runner: &codex_runner,
            proposal_adapter: &proposal_adapter,
        },
    )
}

pub fn scan_selected_chats_at_with_app_data_dir(
    request: ScanSelectedChatsRequest,
    store_path: &Path,
    messages_db_path: &Path,
    app_data_dir: &Path,
) -> Result<ScanSelectedChatsResult, ScanSelectedChatsError> {
    request.validate_local_diagnostics_retention()?;
    let proposal_adapter = eventkit_proposal::EventKitProposalBridge;
    let codex_runner = ProcessCodexExecRunner;
    scan_selected_chats_at_with_codex_dependencies_and_app_data_dir(
        request,
        store_path,
        messages_db_path,
        app_data_dir,
        ProductionScanCodexDependencies {
            auth_readiness: codex_auth::probe_codex_provider_auth(),
            codex_runner: &codex_runner,
            proposal_adapter: &proposal_adapter,
        },
    )
}

pub fn scan_selected_chats_at_with_codex_dependencies<R, A>(
    request: ScanSelectedChatsRequest,
    store_path: &Path,
    messages_db_path: &Path,
    dependencies: ProductionScanCodexDependencies<'_, R, A>,
) -> Result<ScanSelectedChatsResult, ScanSelectedChatsError>
where
    R: CodexExecRunner,
    A: ProposalReplayAdapter,
{
    request.validate_local_diagnostics_retention()?;
    scan_selected_chats_at_with_provider_mode(
        request,
        store_path,
        messages_db_path,
        None,
        dependencies,
    )
}

pub fn scan_selected_chats_at_with_codex_dependencies_and_app_data_dir<R, A>(
    request: ScanSelectedChatsRequest,
    store_path: &Path,
    messages_db_path: &Path,
    app_data_dir: &Path,
    dependencies: ProductionScanCodexDependencies<'_, R, A>,
) -> Result<ScanSelectedChatsResult, ScanSelectedChatsError>
where
    R: CodexExecRunner,
    A: ProposalReplayAdapter,
{
    request.validate_local_diagnostics_retention()?;
    scan_selected_chats_at_with_provider_mode(
        request,
        store_path,
        messages_db_path,
        Some(app_data_dir),
        dependencies,
    )
}

fn scan_selected_chats_at_with_provider_mode<R, A>(
    request: ScanSelectedChatsRequest,
    store_path: &Path,
    messages_db_path: &Path,
    app_data_dir: Option<&Path>,
    dependencies: ProductionScanCodexDependencies<'_, R, A>,
) -> Result<ScanSelectedChatsResult, ScanSelectedChatsError>
where
    R: CodexExecRunner,
    A: ProposalReplayAdapter,
{
    if dependencies.auth_readiness.ready {
        let provider = CodexProvider::new(dependencies.codex_runner);
        scan_selected_chats_with_provider(
            request,
            store_path,
            messages_db_path,
            app_data_dir,
            &provider,
            dependencies.proposal_adapter,
        )
    } else {
        scan_selected_chats_with_unavailable_provider(
            request,
            store_path,
            messages_db_path,
            app_data_dir,
            dependencies.proposal_adapter,
        )
    }
}

fn scan_selected_chats_with_provider<P, A>(
    request: ScanSelectedChatsRequest,
    store_path: &Path,
    messages_db_path: &Path,
    app_data_dir: Option<&Path>,
    provider: &P,
    proposal_adapter: &A,
) -> Result<ScanSelectedChatsResult, ScanSelectedChatsError>
where
    P: morrow_detection::AiProvider,
    A: ProposalReplayAdapter,
{
    match app_data_dir {
        Some(app_data_dir) => scan::scan_selected_chats_at_with_dependencies_and_app_data_dir(
            request,
            store_path,
            messages_db_path,
            app_data_dir,
            provider,
            proposal_adapter,
        ),
        None => scan::scan_selected_chats_at_with_dependencies(
            request,
            store_path,
            messages_db_path,
            provider,
            proposal_adapter,
        ),
    }
}

fn scan_selected_chats_with_unavailable_provider<A>(
    request: ScanSelectedChatsRequest,
    store_path: &Path,
    messages_db_path: &Path,
    app_data_dir: Option<&Path>,
    proposal_adapter: &A,
) -> Result<ScanSelectedChatsResult, ScanSelectedChatsError>
where
    A: ProposalReplayAdapter,
{
    match app_data_dir {
        Some(app_data_dir) => {
            scan::scan_selected_chats_at_with_unavailable_provider_and_app_data_dir(
                request,
                store_path,
                messages_db_path,
                app_data_dir,
                proposal_adapter,
            )
        }
        None => scan::scan_selected_chats_at_with_unavailable_provider(
            request,
            store_path,
            messages_db_path,
            proposal_adapter,
        ),
    }
}
