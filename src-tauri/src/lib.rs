mod db;
use db::{QueryResult, TableInfo, ColumnInfo, ConnectionWrapper};

#[tauri::command]
async fn test_connection(info: ConnectionWrapper) -> Result<String, String> {
    db::test_connection(info).await
}

#[tauri::command]
async fn test_connection_string(connection_string: String) -> Result<String, String> {
    db::test_connection(ConnectionWrapper::String(connection_string)).await
}

#[tauri::command]
async fn execute_query(info: ConnectionWrapper, query: String) -> Result<QueryResult, String> {
    db::execute_query(info, query).await
}

#[tauri::command]
async fn get_tables(info: ConnectionWrapper) -> Result<Vec<TableInfo>, String> {
    db::get_tables(info).await
}

#[tauri::command]
async fn get_columns(info: ConnectionWrapper, table: String, schema: Option<String>) -> Result<Vec<ColumnInfo>, String> {
    db::get_columns(info, table, schema).await
}

#[tauri::command]
async fn get_odbc_drivers() -> Result<Vec<String>, String> {
    db::get_odbc_drivers()
}

#[tauri::command]
async fn get_databases(info: ConnectionWrapper) -> Result<Vec<String>, String> {
    db::get_databases(info).await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            test_connection,
            test_connection_string,
            execute_query,
            get_tables,
            get_columns,
            get_odbc_drivers,
            get_databases
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
