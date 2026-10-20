// 發布版本不顯示主控台視窗
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // OpenSSH 透過 SSH_ASKPASS 呼叫本程式時，只負責轉送提示並回傳答案，不啟動 GUI
    if conduit_lib::askpass::is_client_mode() {
        std::process::exit(conduit_lib::askpass::run_client());
    }
    conduit_lib::run();
}
