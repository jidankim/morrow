use std::future::Future;

use morrow_lib::native_bridge::{
    reconcile_now, scan_selected_chats, NativeBridgeState, ScanSelectedChatsRequest,
    ScanSelectedChatsResult,
};
use tauri::{AppHandle, State};

#[test]
fn sync_now_command_wrappers_return_futures() {
    fn assert_future<T, F: Future<Output = T>>(_future: F) {}

    fn scan_contract(
        app: AppHandle,
        state: State<'_, NativeBridgeState>,
        request: ScanSelectedChatsRequest,
    ) {
        assert_future::<Result<ScanSelectedChatsResult, String>, _>(scan_selected_chats(
            app, state, request,
        ));
    }

    fn reconcile_contract(app: AppHandle) {
        assert_future::<Result<(), String>, _>(reconcile_now(app));
    }

    let _scan_contract = scan_contract;
    let _reconcile_contract = reconcile_contract;
}
