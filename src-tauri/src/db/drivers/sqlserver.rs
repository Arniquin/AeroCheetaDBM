use futures::StreamExt;
use std::time::Instant;
use tokio::net::TcpStream;
use tiberius::{Client as TiberiusClient, Config, AuthMethod};
use tokio_util::compat::TokioAsyncWriteCompatExt;
use azure_identity::DefaultAzureCredential;
use azure_core::auth::TokenCredential;

#[cfg(windows)]
use std::sync::OnceLock;
#[cfg(windows)]
use odbc_api::{Environment, ConnectionOptions, Cursor, buffers::TextRowSet, ResultSetMetadata};

use crate::db::models::{ConnectionInfo, QueryResult, TableInfo, ColumnInfo};

// --- ODBC LOGIC (Windows only) ---

#[cfg(windows)]
static ODBC_ENV: OnceLock<Environment> = OnceLock::new();

#[cfg(windows)]
pub fn get_odbc_env() -> Result<&'static Environment, String> {
    if let Some(env) = ODBC_ENV.get() {
        return Ok(env);
    }
    let env = Environment::new().map_err(|e| e.to_string())?;
    let _ = ODBC_ENV.set(env);
    Ok(ODBC_ENV.get().expect("Environment should be set"))
}

pub fn get_odbc_drivers() -> Result<Vec<String>, String> {
    #[cfg(windows)]
    {
        let env = get_odbc_env()?;
        let drivers = env.drivers().map_err(|e| e.to_string())?;
        Ok(drivers.into_iter().map(|d| d.description).collect())
    }
    #[cfg(not(windows))]
    {
        Ok(vec![])
    }
}

#[cfg(windows)]
fn get_best_sql_server_driver() -> Option<String> {
    let env = get_odbc_env().ok()?;
    let modern_drivers = [
        "ODBC Driver 18 for SQL Server",
        "ODBC Driver 17 for SQL Server",
        "ODBC Driver 13 for SQL Server",
        "ODBC Driver 11 for SQL Server",
    ];

    if let Ok(drivers_list) = env.drivers() {
        for driver_info in drivers_list {
            for &modern in &modern_drivers {
                if driver_info.description.contains(modern) {
                    return Some(modern.to_string());
                }
            }
        }
    }

    if let Ok(drivers_list) = env.drivers() {
        for driver_info in drivers_list {
            if driver_info.description.contains("SQL Server") {
                return Some("SQL Server".to_string());
            }
        }
    }
    None
}

#[cfg(windows)]
fn build_odbc_connection_string(info: &ConnectionInfo) -> Result<String, String> {
    let driver = get_best_sql_server_driver().ok_or_else(|| "No SQL Server ODBC driver found on this system. Please install the Microsoft ODBC Driver for SQL Server.".to_string())?;
    let mut parts = Vec::new();
    
    if info.host.is_empty() {
        let mut conn_str = info.database.clone().unwrap_or_default();
        if !conn_str.to_lowercase().contains("driver=") {
            conn_str = format!("Driver={{{}}};{}", driver, conn_str);
        }
        if info.use_entra_id.unwrap_or(false) && !conn_str.to_lowercase().contains("encrypt=") {
            if !conn_str.ends_with(';') && !conn_str.is_empty() { conn_str.push(';'); }
            conn_str.push_str("Encrypt=yes");
        }
        if info.use_entra_id.unwrap_or(false) && !conn_str.to_lowercase().contains("authentication=") {
            if !conn_str.ends_with(';') && !conn_str.is_empty() { conn_str.push(';'); }
            conn_str.push_str("Authentication=ActiveDirectoryIntegrated");
        }
        return Ok(conn_str);
    }

    parts.push(format!("Driver={{{}}}", driver));
    parts.push(format!("Server={},{}", info.host, info.port));
    if let Some(db) = &info.database {
        if !db.trim().is_empty() { parts.push(format!("Database={}", db)); }
    }

    if info.use_entra_id.unwrap_or(false) {
        if driver == "SQL Server" {
            return Err("Entra ID authentication requires a modern ODBC driver (v13.1, v17, or v18). Only the legacy 'SQL Server' driver was found.".to_string());
        }
        parts.push("Authentication=ActiveDirectoryIntegrated".to_string());
        parts.push("Encrypt=yes".to_string());
    } else if let Some(user) = &info.user {
        if !user.trim().is_empty() {
            parts.push(format!("UID={}", user));
            parts.push(format!("PWD={}", info.password.as_deref().unwrap_or_default()));
        } else {
            parts.push("Trusted_Connection=yes".to_string());
        }
    } else {
        parts.push("Trusted_Connection=yes".to_string());
    }

    if info.trust_server_certificate.unwrap_or(false) {
        parts.push("TrustServerCertificate=yes".to_string());
    }

    if let Some(timeout) = info.connect_timeout {
        parts.push(format!("ConnectRetryCount=0;Connection Timeout={}", timeout));
    }

    Ok(parts.join(";"))
}

#[cfg(windows)]
async fn test_connection_odbc(info: ConnectionInfo) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        let env = get_odbc_env()?;
        let conn_str = build_odbc_connection_string(&info)?;
        env.connect_with_connection_string(&conn_str, ConnectionOptions::default())
            .map_err(|e| format!("ODBC Connection failed: {}", e))?;
        Ok("Connection successful (ODBC)".to_string())
    }).await.map_err(|e| e.to_string())?
}

#[cfg(windows)]
async fn execute_query_odbc(info: ConnectionInfo, query: String) -> Result<QueryResult, String> {
    let start = Instant::now();
    tokio::task::spawn_blocking(move || {
        let env = get_odbc_env()?;
        let conn_str = build_odbc_connection_string(&info)?;
        let conn = env.connect_with_connection_string(&conn_str, ConnectionOptions::default())
            .map_err(|e| format!("ODBC Connection failed: {}", e))?;
        
        let cursor = conn.execute(&query, (), None).map_err(|e| e.to_string())?;
        let mut columns = Vec::new();
        let mut rows = Vec::new();
        
        if let Some(mut cursor) = cursor {
            let num_cols = cursor.num_result_cols().map_err(|e| e.to_string())?;
            for i in 1..=num_cols {
                let name = cursor.col_name(i as u16).map_err(|e| e.to_string())?;
                columns.push(name);
            }

            let mut buffers = TextRowSet::for_cursor(100, &mut cursor, Some(4096)).map_err(|e| e.to_string())?;
            let mut row_set_cursor = cursor.bind_buffer(&mut buffers).map_err(|e| e.to_string())?;
            
            while let Some(batch) = row_set_cursor.fetch().map_err(|e| e.to_string())? {
                for row_index in 0..batch.num_rows() {
                    let mut map = serde_json::Map::new();
                    for col_index in 0..num_cols {
                        let val = batch.at(col_index as usize, row_index)
                            .map(|bytes| String::from_utf8_lossy(bytes).trim_end_matches('\0').to_string())
                            .map(serde_json::Value::String)
                            .unwrap_or(serde_json::Value::Null);
                        map.insert(columns[col_index as usize].clone(), val);
                    }
                    rows.push(serde_json::Value::Object(map));
                }
            }
        }

        Ok(QueryResult {
            columns,
            rows,
            execution_time_ms: start.elapsed().as_millis(),
        })
    }).await.map_err(|e| e.to_string())?
}

// --- TIBERIUS LOGIC ---

pub async fn get_sqlserver_token() -> Result<String, String> {
    let credential = DefaultAzureCredential::create(Default::default())
        .map_err(|e| format!("Failed to initialize Azure credentials: {}", e))?;
    
    let token_response = credential
        .get_token(&["https://database.windows.net/.default"])
        .await
        .map_err(|e| format!("Failed to fetch Entra ID token: {}", e))?;
    
    Ok(token_response.token.secret().to_string())
}

pub async fn get_sqlserver_client(info: &ConnectionInfo) -> Result<TiberiusClient<tokio_util::compat::Compat<TcpStream>>, String> {
    let mut config = if !info.host.is_empty() {
        let mut c = Config::new();
        c.host(&info.host);
        c.port(info.port);
        if let Some(user) = &info.user {
            if !user.trim().is_empty() {
                c.authentication(AuthMethod::sql_server(user, info.password.as_deref().unwrap_or_default()));
            }
        }
        if let Some(db) = &info.database {
            if !db.trim().is_empty() {
                c.database(db);
            }
        }
        c
    } else if let Some(url) = &info.database {
        Config::from_ado_string(url).map_err(|e| e.to_string())?
    } else {
        return Err("Invalid SQL Server configuration: No host or connection string provided".to_string());
    };

    if info.use_entra_id.unwrap_or(false) {
        let token = get_sqlserver_token().await?;
        config.authentication(AuthMethod::AADToken(token));
        config.encryption(tiberius::EncryptionLevel::Required);
    }

    if info.trust_server_certificate.unwrap_or(false) {
        config.trust_cert();
    }

    let timeout_duration = std::time::Duration::from_secs(info.connect_timeout.unwrap_or(30) as u64);

    let connect_future = async {
        let tcp = TcpStream::connect(config.get_addr()).await.map_err(|e| e.to_string())?;
        tcp.set_nodelay(true).map_err(|e| e.to_string())?;
        TiberiusClient::connect(config, tcp.compat_write()).await.map_err(|e| e.to_string())
    };

    match tokio::time::timeout(timeout_duration, connect_future).await {
        Ok(res) => res,
        Err(_) => Err("SQL Server connection timed out".to_string()),
    }
}

// --- DRIVER INTERFACE ---

pub async fn test_connection(info: &ConnectionInfo) -> Result<String, String> {
    #[cfg(windows)]
    if info.use_entra_id.unwrap_or(false) || (info.user.as_deref().unwrap_or("").is_empty() && !info.host.is_empty()) {
        return test_connection_odbc(info.clone()).await;
    }

    let _client = get_sqlserver_client(info).await?;
    Ok("Connection successful".to_string())
}

pub async fn execute_query(info: &ConnectionInfo, query: &str) -> Result<QueryResult, String> {
    #[cfg(windows)]
    if info.use_entra_id.unwrap_or(false) || (info.user.as_deref().unwrap_or("").is_empty() && !info.host.is_empty()) {
        return execute_query_odbc(info.clone(), query.to_string()).await;
    }

    let start = Instant::now();
    let mut client = get_sqlserver_client(info).await?;
    let mut stream = client.query(query, &[]).await.map_err(|e| e.to_string())?;
    
    let mut columns = Vec::new();
    let mut results = Vec::new();

    while let Some(item) = stream.next().await {
        match item {
            Ok(tiberius::QueryItem::Metadata(meta)) => {
                for col in meta.columns() {
                    columns.push(col.name().to_string());
                }
            }
            Ok(tiberius::QueryItem::Row(row)) => {
                let mut map = serde_json::Map::new();
                for (i, col_name) in columns.iter().enumerate() {
                    let val: serde_json::Value = if let Ok(Some(s)) = row.try_get::<&str, _>(i) {
                        serde_json::Value::String(s.to_string())
                    } else if let Ok(Some(n)) = row.try_get::<i32, _>(i) {
                        serde_json::Value::Number(n.into())
                    } else if let Ok(Some(n)) = row.try_get::<i64, _>(i) {
                        serde_json::Value::Number(n.into())
                    } else if let Ok(Some(f)) = row.try_get::<f64, _>(i) {
                        serde_json::Number::from_f64(f).map(serde_json::Value::Number).unwrap_or(serde_json::Value::Null)
                    } else if let Ok(Some(b)) = row.try_get::<bool, _>(i) {
                        serde_json::Value::Bool(b)
                    } else {
                        serde_json::Value::Null
                    };
                    map.insert(col_name.clone(), val);
                }
                results.push(serde_json::Value::Object(map));
            }
            Err(e) => return Err(e.to_string()),
        }
    }

    Ok(QueryResult {
        columns,
        rows: results,
        execution_time_ms: start.elapsed().as_millis(),
    })
}

pub async fn get_databases(info: &ConnectionInfo) -> Result<Vec<String>, String> {
    #[cfg(windows)]
    if info.use_entra_id.unwrap_or(false) || (info.user.as_deref().unwrap_or("").is_empty() && !info.host.is_empty()) {
        let res = execute_query_odbc(info.clone(), "SELECT name FROM sys.databases WHERE state = 0".to_string()).await?;
        return Ok(res.rows.into_iter().filter_map(|v| {
            v.as_object()?.get("name")?.as_str().map(|s| s.to_string())
        }).collect());
    }

    let mut client = get_sqlserver_client(info).await?;
    let mut stream = client.query("SELECT name FROM sys.databases WHERE state = 0", &[]).await.map_err(|e| e.to_string())?;
    let mut results = Vec::new();
    while let Some(item) = stream.next().await {
        if let Ok(tiberius::QueryItem::Row(row)) = item {
            if let Some(s) = row.try_get::<&str, _>(0).unwrap_or(None) {
                results.push(s.to_string());
            }
        }
    }
    Ok(results)
}

pub async fn get_tables(info: &ConnectionInfo) -> Result<Vec<TableInfo>, String> {
    #[cfg(windows)]
    if info.use_entra_id.unwrap_or(false) || (info.user.as_deref().unwrap_or("").is_empty() && !info.host.is_empty()) {
        let res = execute_query_odbc(info.clone(), "SELECT TABLE_NAME as [name], TABLE_SCHEMA as [schema], DB_NAME() as [database] FROM INFORMATION_SCHEMA.TABLES WHERE TABLE_TYPE = 'BASE TABLE'".to_string()).await?;
        return Ok(res.rows.into_iter().filter_map(|v| {
            let obj = v.as_object()?;
            Some(TableInfo {
                name: obj.get("name")?.as_str()?.to_string(),
                schema: obj.get("schema")?.as_str().map(|s| s.to_string()),
                database: obj.get("database")?.as_str().map(|s| s.to_string()),
            })
        }).collect());
    }

    let mut client = get_sqlserver_client(info).await?;
    let query = "SELECT TABLE_NAME as [name], TABLE_SCHEMA as [schema], DB_NAME() as [database] FROM INFORMATION_SCHEMA.TABLES WHERE TABLE_TYPE = 'BASE TABLE'";
    let mut stream = client.query(query, &[]).await.map_err(|e| e.to_string())?;
    
    let mut results = Vec::new();
    while let Some(item) = stream.next().await {
        match item {
            Ok(tiberius::QueryItem::Row(row)) => {
                results.push(TableInfo {
                    name: row.try_get::<&str, _>(0).unwrap_or(Some("")).unwrap_or("").to_string(),
                    schema: row.try_get::<&str, _>(1).unwrap_or(None).map(|s| s.to_string()),
                    database: row.try_get::<&str, _>(2).unwrap_or(None).map(|s| s.to_string()),
                });
            }
            _ => {}
        }
    }
    Ok(results)
}

pub async fn get_columns(info: &ConnectionInfo, table: &str, schema: Option<&str>) -> Result<Vec<ColumnInfo>, String> {
    let query = format!(
        "SELECT COLUMN_NAME as [name], DATA_TYPE as [data_type], IS_NULLABLE as [is_nullable], COLUMN_DEFAULT as [default_value], CHARACTER_MAXIMUM_LENGTH as [max_length] 
         FROM INFORMATION_SCHEMA.COLUMNS 
         WHERE TABLE_NAME = '{}' AND TABLE_SCHEMA = '{}'",
        table,
        schema.unwrap_or("dbo")
    );
    
    #[cfg(windows)]
    if info.use_entra_id.unwrap_or(false) || (info.user.as_deref().unwrap_or("").is_empty() && !info.host.is_empty()) {
        let res = execute_query_odbc(info.clone(), query).await?;
        return Ok(res.rows.into_iter().filter_map(|v| {
            let obj = v.as_object()?;
            Some(ColumnInfo {
                name: obj.get("name")?.as_str()?.to_string(),
                data_type: obj.get("data_type")?.as_str()?.to_string(),
                is_nullable: obj.get("is_nullable").and_then(|v| v.as_str()).map(|s| s == "YES"),
                default_value: obj.get("default_value").and_then(|v| v.as_str()).map(|s| s.to_string()),
                max_length: obj.get("max_length").and_then(|v| v.as_i64()).or_else(|| obj.get("max_length").and_then(|v| v.as_str().and_then(|s| s.parse::<i64>().ok()))).map(|n| n as i32),
            })
        }).collect());
    }

    let mut client = get_sqlserver_client(info).await?;
    let mut stream = client.query(query, &[]).await.map_err(|e| e.to_string())?;
    
    let mut results = Vec::new();
    while let Some(item) = stream.next().await {
        match item {
            Ok(tiberius::QueryItem::Row(row)) => {
                results.push(ColumnInfo {
                    name: row.try_get::<&str, _>(0).unwrap_or(Some("")).unwrap_or("").to_string(),
                    data_type: row.try_get::<&str, _>(1).unwrap_or(Some("")).unwrap_or("").to_string(),
                    is_nullable: row.try_get::<&str, _>(2).unwrap_or(Some("")).map(|s| s == "YES"),
                    default_value: row.try_get::<&str, _>(3).unwrap_or(None).map(|s| s.to_string()),
                    max_length: row.try_get::<i32, _>(4).unwrap_or(None),
                });
            }
            _ => {}
        }
    }
    Ok(results)
}
