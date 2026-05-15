use serde::{Deserialize, Serialize};
use sqlx::{Row, Column, Connection};
use sqlx::postgres::PgConnection;
use sqlx::mysql::MySqlConnection;
use mongodb::{Client, bson::{Document, doc}};
use tiberius::{Client as TiberiusClient, Config, AuthMethod};
use tokio::net::TcpStream;
use tokio_util::compat::TokioAsyncWriteCompatExt;
use std::time::Instant;
use futures::StreamExt;
use azure_identity::DefaultAzureCredential;
use azure_core::auth::TokenCredential;
#[cfg(windows)]
use std::sync::OnceLock;

#[cfg(windows)]
use odbc_api::{Environment, ConnectionOptions, ResultSetMetadata, Cursor, buffers::TextRowSet};

#[cfg(windows)]
static ODBC_ENV: OnceLock<Environment> = OnceLock::new();

#[cfg(windows)]
fn get_odbc_env() -> Result<&'static Environment, String> {
    if let Some(env) = ODBC_ENV.get() {
        return Ok(env);
    }
    let env = Environment::new().map_err(|e| e.to_string())?;
    let _ = ODBC_ENV.set(env);
    Ok(ODBC_ENV.get().expect("Environment should be set"))
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub enum DbType {
    #[serde(rename = "postgres")]
    Postgres,
    #[serde(rename = "mysql")]
    Mysql,
    #[serde(rename = "sqlserver")]
    Sqlserver,
    #[serde(rename = "mongodb")]
    Mongodb,
    #[serde(rename = "mock")]
    Mock,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ConnectionInfo {
    pub db_type: DbType,
    pub host: String,
    pub port: u16,
    pub user: Option<String>,
    pub password: Option<String>,
    pub database: Option<String>,
    pub use_entra_id: Option<bool>,
    pub trust_server_certificate: Option<bool>,
    pub connect_timeout: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<serde_json::Value>,
    pub execution_time_ms: u128,
}

#[derive(Debug, Serialize)]
pub struct TableInfo {
    pub name: String,
    pub schema: Option<String>,
    pub database: Option<String>,
}

pub async fn get_databases(wrapper: ConnectionWrapper) -> Result<Vec<String>, String> {
    let info = wrapper.get_info()?;
    match info.db_type {
        DbType::Mock => Ok(vec!["demo_db".to_string(), "test_db".to_string()]),
        DbType::Postgres => {
            let query = "SELECT datname FROM pg_database WHERE datistemplate = false";
            let res = execute_query(wrapper, query.to_string()).await?;
            Ok(res.rows.into_iter().filter_map(|v| {
                v.as_object()?.get("datname")?.as_str().map(|s| s.to_string())
            }).collect())
        }
        DbType::Mysql => {
            let query = "SHOW DATABASES";
            let res = execute_query(wrapper, query.to_string()).await?;
            Ok(res.rows.into_iter().filter_map(|v| {
                v.as_object()?.get("Database")?.as_str().map(|s| s.to_string())
            }).collect())
        }
        DbType::Mongodb => {
            let info = wrapper.get_info()?;
            let url = build_connection_url(&info);
            let client = Client::with_uri_str(&url).await.map_err(|e| e.to_string())?;
            let db_names = client.list_database_names().await.map_err(|e| e.to_string())?;
            Ok(db_names)
        }
        DbType::Sqlserver => {
            #[cfg(windows)]
            if info.use_entra_id.unwrap_or(false) || (info.user.as_deref().unwrap_or("").is_empty() && !info.host.is_empty()) {
                let res = execute_query_odbc(info, "SELECT name FROM sys.databases WHERE state = 0".to_string()).await?;
                return Ok(res.rows.into_iter().filter_map(|v| {
                    v.as_object()?.get("name")?.as_str().map(|s| s.to_string())
                }).collect());
            }

            let mut client = get_sqlserver_client(&info).await?;
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
    }
}

#[derive(Debug, Serialize)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub is_nullable: Option<bool>,
    pub default_value: Option<String>,
    pub max_length: Option<i32>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum ConnectionWrapper {
    Info(ConnectionInfo),
    String(String),
}

impl ConnectionWrapper {
    pub fn get_info(&self) -> Result<ConnectionInfo, String> {
        match self {
            ConnectionWrapper::Info(info) => Ok(info.clone()),
            ConnectionWrapper::String(s) => {
                let s_lower = s.to_lowercase();
                let trust_server_certificate = s_lower.contains("trustservercertificate=true") 
                                            || s_lower.contains("sslmode=no-verify")
                                            || s_lower.contains("sslmode=disable");

                if s.starts_with("postgres://") {
                    Ok(ConnectionInfo { db_type: DbType::Postgres, host: "".to_string(), port: 0, user: None, password: None, database: Some(s.clone()), use_entra_id: None, trust_server_certificate: Some(trust_server_certificate), connect_timeout: None })
                } else if s.starts_with("mysql://") {
                    Ok(ConnectionInfo { db_type: DbType::Mysql, host: "".to_string(), port: 0, user: None, password: None, database: Some(s.clone()), use_entra_id: None, trust_server_certificate: Some(trust_server_certificate), connect_timeout: None })
                } else if s.starts_with("mongodb://") {
                    Ok(ConnectionInfo { db_type: DbType::Mongodb, host: "".to_string(), port: 0, user: None, password: None, database: Some(s.clone()), use_entra_id: None, trust_server_certificate: Some(trust_server_certificate), connect_timeout: None })
                } else if s.starts_with("sqlserver://") || s_lower.contains("initial catalog=") || s_lower.contains("database=") || s_lower.contains("server=") {
                    let use_entra_id = s_lower.contains("authentication=activedirectoryintegrated") 
                                    || s_lower.contains("authentication=activedirectorymsi")
                                    || s_lower.contains("authentication=activedirectorydefault");
                    
                    Ok(ConnectionInfo { 
                        db_type: DbType::Sqlserver, 
                        host: "".to_string(), 
                        port: 0, 
                        user: None, 
                        password: None, 
                        database: Some(s.clone()),
                        use_entra_id: Some(use_entra_id),
                        trust_server_certificate: Some(trust_server_certificate),
                        connect_timeout: None
                    })
                } else {
                    Err("Unsupported connection string format".to_string())
                }
            }
        }
    }
}

async fn get_sqlserver_token() -> Result<String, String> {
    let credential = DefaultAzureCredential::create(Default::default())
        .map_err(|e| format!("Failed to initialize Azure credentials: {}", e))?;
    
    let token_response = credential
        .get_token(&["https://database.windows.net/.default"])
        .await
        .map_err(|e| format!("Failed to fetch Entra ID token: {}", e))?;
    
    Ok(token_response.token.secret().to_string())
}

#[cfg(windows)]
pub fn get_odbc_drivers() -> Result<Vec<String>, String> {
    let env = get_odbc_env()?;
    let drivers = env.drivers().map_err(|e| e.to_string())?;
    Ok(drivers.into_iter().map(|d| d.description).collect())
}

#[cfg(not(windows))]
pub fn get_odbc_drivers() -> Result<Vec<String>, String> {
    Ok(vec![])
}

#[cfg(windows)]
fn get_best_sql_server_driver() -> Option<String> {
    let env = get_odbc_env().ok()?;

    // Check for modern drivers first
    let modern_drivers = [
        "ODBC Driver 18 for SQL Server",
        "ODBC Driver 17 for SQL Server",
        "ODBC Driver 13 for SQL Server",
        "ODBC Driver 11 for SQL Server",
    ];

    // Note: data_sources() returns DSNs, not drivers. 
    // To get drivers, we should use env.drivers()
    if let Ok(drivers_list) = env.drivers() {
        for driver_info in drivers_list {
            for &modern in &modern_drivers {
                if driver_info.description.contains(modern) {
                    return Some(modern.to_string());
                }
            }
        }
    }

    // Fallback to legacy if nothing else found
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
        // Connection string mode
        let mut conn_str = info.database.clone().unwrap_or_default();
        
        // Ensure Driver is present
        if !conn_str.to_lowercase().contains("driver=") {
            conn_str = format!("Driver={{{}}};{}", driver, conn_str);
        }
        
        // Ensure Encrypt=yes if Entra ID is used
        if info.use_entra_id.unwrap_or(false) && !conn_str.to_lowercase().contains("encrypt=") {
            if !conn_str.ends_with(';') && !conn_str.is_empty() {
                conn_str.push(';');
            }
            conn_str.push_str("Encrypt=yes");
        }

        // Ensure Authentication=ActiveDirectoryIntegrated if Entra ID is used and no authentication is specified
        if info.use_entra_id.unwrap_or(false) && !conn_str.to_lowercase().contains("authentication=") {
            if !conn_str.ends_with(';') && !conn_str.is_empty() {
                conn_str.push(';');
            }
            conn_str.push_str("Authentication=ActiveDirectoryIntegrated");
        }
        
        return Ok(conn_str);
    }

    // Structured mode
    parts.push(format!("Driver={{{}}}", driver));
    parts.push(format!("Server={},{}", info.host, info.port));

    if let Some(db) = &info.database {
        if !db.trim().is_empty() {
            parts.push(format!("Database={}", db));
        }
    }

    if info.use_entra_id.unwrap_or(false) {
        if driver == "SQL Server" {
            return Err("Entra ID authentication requires a modern ODBC driver (v13.1, v17, or v18). Only the legacy 'SQL Server' driver was found.".to_string());
        }
        parts.push("Authentication=ActiveDirectoryIntegrated".to_string());
        parts.push("Encrypt=yes".to_string()); // Mandatory for Azure/Entra ID
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

fn build_connection_url(info: &ConnectionInfo) -> String {
    if !info.host.is_empty() {
        // Structured config from form
        let user_part = match (&info.user, &info.password) {
            (Some(u), Some(p)) if !u.is_empty() => format!("{}:{}@", u, p),
            (Some(u), None) if !u.is_empty() => format!("{}@", u),
            _ => "".to_string(),
        };
        
        let db_path = match info.database.as_deref() {
            Some(d) if !d.trim().is_empty() => format!("/{}", d),
            _ => "".to_string(),
        };

        let timeout = info.connect_timeout.unwrap_or(30);

        match info.db_type {
            DbType::Postgres => {
                let mut url = format!(
                    "postgres://{}{}:{}{}",
                    user_part,
                    info.host,
                    info.port,
                    db_path
                );
                if info.trust_server_certificate.unwrap_or(false) && !url.contains("sslmode=") {
                    url.push_str(if url.contains('?') { "&" } else { "?" });
                    url.push_str("sslmode=no-verify");
                }
                if !url.contains("connect_timeout=") {
                    url.push_str(if url.contains('?') { "&" } else { "?" });
                    url.push_str(&format!("connect_timeout={}", timeout));
                }
                url
            }
            DbType::Mysql => {
                let mut url = format!(
                    "mysql://{}{}:{}{}",
                    user_part,
                    info.host,
                    info.port,
                    db_path
                );
                if info.trust_server_certificate.unwrap_or(false) && !url.contains("ssl-mode=") {
                    url.push_str(if url.contains('?') { "&" } else { "?" });
                    url.push_str("ssl-mode=Disabled");
                }
                if !url.contains("connect_timeout=") {
                    url.push_str(if url.contains('?') { "&" } else { "?" });
                    url.push_str(&format!("connect_timeout={}", timeout));
                }
                url
            }
            DbType::Mongodb => {
                let mut url = format!(
                    "mongodb://{}{}:{}{}",
                    user_part,
                    info.host,
                    info.port,
                    db_path
                );
                if info.trust_server_certificate.unwrap_or(false) && !url.contains("tlsInsecure=") {
                    url.push_str(if url.contains('?') { "&" } else { "?" });
                    url.push_str("tlsInsecure=true");
                }
                if !url.contains("connectTimeoutMS=") {
                    url.push_str(if url.contains('?') { "&" } else { "?" });
                    url.push_str(&format!("connectTimeoutMS={}", timeout * 1000));
                }
                url
            }
            _ => "".to_string(),
        }
    } else {
        // Connection string mode
        info.database.clone().unwrap_or_default()
    }
}

pub async fn test_connection(wrapper: ConnectionWrapper) -> Result<String, String> {
    let info = wrapper.get_info()?;
    match info.db_type {
        DbType::Mock => Ok("Connection successful (Mock)".to_string()),
        DbType::Postgres => {
            let url = build_connection_url(&info);
            println!("Connecting to Postgres with URL: {}", url);
            match PgConnection::connect(&url).await {
                Ok(_) => Ok("Connection successful".to_string()),
                Err(e) => Err(format!("Postgres connection failed: {}", e)),
            }
        }
        DbType::Mysql => {
            let url = build_connection_url(&info);
            println!("Connecting to MySQL with URL: {}", url);
            match MySqlConnection::connect(&url).await {
                Ok(_) => Ok("Connection successful".to_string()),
                Err(e) => Err(format!("MySQL connection failed: {}", e)),
            }
        }
        DbType::Mongodb => {
            let url = build_connection_url(&info);
            println!("Connecting to MongoDB with URL: {}", url);
            match Client::with_uri_str(&url).await {
                Ok(client) => {
                    match client.list_database_names().await {
                        Ok(_) => Ok("Connection successful".to_string()),
                        Err(e) => Err(format!("MongoDB connection failed: {}", e)),
                    }
                }
                Err(e) => Err(format!("MongoDB connection failed: {}", e)),
            }
        }
        DbType::Sqlserver => {
            #[cfg(windows)]
            if info.use_entra_id.unwrap_or(false) || (info.user.as_deref().unwrap_or("").is_empty() && !info.host.is_empty()) {
                return test_connection_odbc(info).await;
            }

            let mut config = if !info.host.is_empty() {
                // Structured config from form
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
            } else if let Some(url) = info.database {
                // Connection string
                Config::from_ado_string(&url).map_err(|e| e.to_string())?
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
                Ok(Ok(_)) => Ok("Connection successful".to_string()),
                Ok(Err(e)) => Err(format!("SQL Server connection failed: {}", e)),
                Err(_) => Err("SQL Server connection timed out".to_string()),
            }
        }
    }
}

async fn get_sqlserver_client(info: &ConnectionInfo) -> Result<TiberiusClient<tokio_util::compat::Compat<TcpStream>>, String> {
    let mut config = if !info.host.is_empty() {
        // Structured config from form
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
        // Connection string
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

pub async fn execute_query(wrapper: ConnectionWrapper, query: String) -> Result<QueryResult, String> {
    let info = wrapper.get_info()?;
    let start = Instant::now();
    match info.db_type {
        DbType::Mock => {
            let mut normalized_query = query.to_lowercase().replace("\n", " ").replace("\r", " ");
            while normalized_query.contains("  ") {
                normalized_query = normalized_query.replace("  ", " ");
            }
            let (columns, rows) = if normalized_query.contains("from users") {
                (
                    vec!["id".to_string(), "name".to_string(), "email".to_string()],
                    vec![
                        serde_json::json!({"id": 1, "name": "Alice", "email": "alice@example.com"}),
                        serde_json::json!({"id": 2, "name": "Bob", "email": "bob@example.com"}),
                        serde_json::json!({"id": 3, "name": "Charlie", "email": "charlie@example.com"}),
                    ]
                )
            } else if normalized_query.contains("from products") {
                (
                    vec!["id".to_string(), "title".to_string(), "price".to_string()],
                    vec![
                        serde_json::json!({"id": 101, "title": "Laptop", "price": 999.99}),
                        serde_json::json!({"id": 102, "title": "Mouse", "price": 25.50}),
                        serde_json::json!({"id": 103, "title": "Keyboard", "price": 49.99}),
                    ]
                )
            } else {
                (
                    vec!["message".to_string()],
                    vec![serde_json::json!({"message": "Mock query executed successfully. Try 'SELECT * FROM users' or 'SELECT * FROM products' for demo data."})]
                )
            };

            Ok(QueryResult {
                columns,
                rows,
                execution_time_ms: start.elapsed().as_millis(),
            })
        }
        DbType::Postgres => {
            let url = build_connection_url(&info);
            let pool = sqlx::PgPool::connect(&url).await.map_err(|e| e.to_string())?;
            let rows = sqlx::query(&query).fetch_all(&pool).await.map_err(|e| e.to_string())?;
            
            let mut columns = Vec::new();
            if !rows.is_empty() {
                for col in rows[0].columns() {
                    columns.push(col.name().to_string());
                }
            }

            let mut results = Vec::new();
            for row in rows {
                let mut map = serde_json::Map::new();
                for col in row.columns() {
                    let val: serde_json::Value = sqlx::postgres::PgRow::get_unchecked::<Option<serde_json::Value>, _>(&row, col.ordinal()).unwrap_or(serde_json::Value::Null);
                    map.insert(col.name().to_string(), val);
                }
                results.push(serde_json::Value::Object(map));
            }

            Ok(QueryResult {
                columns,
                rows: results,
                execution_time_ms: start.elapsed().as_millis(),
            })
        }
        DbType::Mysql => {
            let url = build_connection_url(&info);
            let pool = sqlx::MySqlPool::connect(&url).await.map_err(|e| e.to_string())?;
            let rows = sqlx::query(&query).fetch_all(&pool).await.map_err(|e| e.to_string())?;
            
            let mut columns = Vec::new();
            if !rows.is_empty() {
                for col in rows[0].columns() {
                    columns.push(col.name().to_string());
                }
            }

            let mut results = Vec::new();
            for row in rows {
                let mut map = serde_json::Map::new();
                for col in row.columns() {
                    let val: serde_json::Value = sqlx::mysql::MySqlRow::get_unchecked::<Option<serde_json::Value>, _>(&row, col.ordinal()).unwrap_or(serde_json::Value::Null);
                    map.insert(col.name().to_string(), val);
                }
                results.push(serde_json::Value::Object(map));
            }

            Ok(QueryResult {
                columns,
                rows: results,
                execution_time_ms: start.elapsed().as_millis(),
            })
        }
        DbType::Mongodb => {
            let url = build_connection_url(&info);
            let client = Client::with_uri_str(&url).await.map_err(|e| e.to_string())?;
            let db_name = match &url.split('/').last() {
                Some(name) if !name.is_empty() => name.split('?').next().unwrap_or("test"),
                _ => "test",
            };
            let db = client.database(db_name);
            
            let mut results = Vec::new();
            if query.trim().starts_with('{') {
                let doc: Document = serde_json::from_str(&query).map_err(|e| e.to_string())?;
                let res_doc = db.run_command(doc).await.map_err(|e| e.to_string())?;
                results.push(serde_json::to_value(res_doc).unwrap_or(serde_json::Value::Null));
            } else {
                let collection = db.collection::<Document>(query.trim());
                let mut cursor = collection.find(doc! {}).limit(100).await.map_err(|e| e.to_string())?;
                while let Some(result) = cursor.next().await {
                    match result {
                        Ok(doc) => {
                            let json = serde_json::to_value(doc).map_err(|e| e.to_string())?;
                            results.push(json);
                        }
                        Err(e) => return Err(e.to_string()),
                    }
                }
            }

            Ok(QueryResult {
                columns: vec!["document".to_string()],
                rows: results,
                execution_time_ms: start.elapsed().as_millis(),
            })
        }
        DbType::Sqlserver => {
            #[cfg(windows)]
            if info.use_entra_id.unwrap_or(false) || (info.user.as_deref().unwrap_or("").is_empty() && !info.host.is_empty()) {
                return execute_query_odbc(info, query).await;
            }

            let mut client = get_sqlserver_client(&info).await?;

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
    }
}

pub async fn get_tables(wrapper: ConnectionWrapper) -> Result<Vec<TableInfo>, String> {
    let info = wrapper.get_info()?;
    match info.db_type {
        DbType::Mock => {
            Ok(vec![
                TableInfo { name: "users".to_string(), schema: Some("public".to_string()), database: Some("demo_db".to_string()) },
                TableInfo { name: "products".to_string(), schema: Some("public".to_string()), database: Some("demo_db".to_string()) },
            ])
        }
        DbType::Postgres => {
            let query = "SELECT table_name as name, table_schema as schema, current_database() as database FROM information_schema.tables WHERE table_schema NOT IN ('information_schema', 'pg_catalog')";
            let res = execute_query(wrapper, query.to_string()).await?;
            Ok(res.rows.into_iter().filter_map(|v| {
                let obj = v.as_object()?;
                Some(TableInfo {
                    name: obj.get("name")?.as_str()?.to_string(),
                    schema: obj.get("schema")?.as_str().map(|s| s.to_string()),
                    database: obj.get("database")?.as_str().map(|s| s.to_string()),
                })
            }).collect())
        }
        DbType::Mysql => {
            let query = "SELECT table_name as name, DATABASE() as database FROM information_schema.tables WHERE table_schema = DATABASE()";
            let res = execute_query(wrapper, query.to_string()).await?;
            Ok(res.rows.into_iter().filter_map(|v| {
                let obj = v.as_object()?;
                Some(TableInfo {
                    name: obj.get("name")?.as_str()?.to_string(),
                    schema: None,
                    database: obj.get("database")?.as_str().map(|s| s.to_string()),
                })
            }).collect())
        }
        DbType::Mongodb => {
            let info = wrapper.get_info()?;
            let url = build_connection_url(&info);
            let client = Client::with_uri_str(&url).await.map_err(|e| e.to_string())?;
            let db_name = match &url.split('/').last() {
                Some(name) if !name.is_empty() => name.split('?').next().unwrap_or("test"),
                _ => "test",
            };
            let db = client.database(db_name);
            let collections = db.list_collection_names().await.map_err(|e| e.to_string())?;
            Ok(collections.into_iter().map(|name| TableInfo { name, schema: None, database: Some(db_name.to_string()) }).collect())
        }
        DbType::Sqlserver => {
            #[cfg(windows)]
            if info.use_entra_id.unwrap_or(false) || (info.user.as_deref().unwrap_or("").is_empty() && !info.host.is_empty()) {
                let res = execute_query_odbc(info, "SELECT TABLE_NAME as [name], TABLE_SCHEMA as [schema], DB_NAME() as [database] FROM INFORMATION_SCHEMA.TABLES WHERE TABLE_TYPE = 'BASE TABLE'".to_string()).await?;
                return Ok(res.rows.into_iter().filter_map(|v| {
                    let obj = v.as_object()?;
                    Some(TableInfo {
                        name: obj.get("name")?.as_str()?.to_string(),
                        schema: obj.get("schema")?.as_str().map(|s| s.to_string()),
                        database: obj.get("database")?.as_str().map(|s| s.to_string()),
                    })
                }).collect());
            }

            let mut client = get_sqlserver_client(&info).await?;
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
    }
}

pub async fn get_columns(wrapper: ConnectionWrapper, table: String, schema: Option<String>) -> Result<Vec<ColumnInfo>, String> {
    let info = wrapper.get_info()?;
    match info.db_type {
        DbType::Mock => {
            if table == "users" {
                Ok(vec![
                    ColumnInfo { name: "id".to_string(), data_type: "integer".to_string(), is_nullable: Some(false), default_value: None, max_length: None },
                    ColumnInfo { name: "name".to_string(), data_type: "varchar".to_string(), is_nullable: Some(false), default_value: None, max_length: Some(100) },
                    ColumnInfo { name: "email".to_string(), data_type: "varchar".to_string(), is_nullable: Some(true), default_value: None, max_length: Some(255) },
                ])
            } else {
                Ok(vec![
                    ColumnInfo { name: "id".to_string(), data_type: "integer".to_string(), is_nullable: Some(false), default_value: None, max_length: None },
                    ColumnInfo { name: "title".to_string(), data_type: "varchar".to_string(), is_nullable: Some(false), default_value: None, max_length: Some(200) },
                    ColumnInfo { name: "price".to_string(), data_type: "numeric".to_string(), is_nullable: Some(false), default_value: Some("0.0".to_string()), max_length: None },
                ])
            }
        }
        DbType::Postgres => {
            let query = format!(
                "SELECT column_name as name, data_type, is_nullable, column_default as default_value, character_maximum_length as max_length 
                 FROM information_schema.columns 
                 WHERE table_name = '{}' AND table_schema = '{}'",
                table,
                schema.unwrap_or_else(|| "public".to_string())
            );
            let res = execute_query(wrapper, query).await?;
            Ok(res.rows.into_iter().filter_map(|v| {
                let obj = v.as_object()?;
                Some(ColumnInfo {
                    name: obj.get("name")?.as_str()?.to_string(),
                    data_type: obj.get("data_type")?.as_str()?.to_string(),
                    is_nullable: obj.get("is_nullable").and_then(|v| v.as_str()).map(|s| s == "YES"),
                    default_value: obj.get("default_value").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    max_length: obj.get("max_length").and_then(|v| v.as_i64()).map(|n| n as i32),
                })
            }).collect())
        }
        DbType::Mysql => {
            let query = format!(
                "SELECT column_name as name, data_type, is_nullable, column_default as default_value, character_maximum_length as max_length 
                 FROM information_schema.columns 
                 WHERE table_name = '{}' AND table_schema = DATABASE()",
                table
            );
            let res = execute_query(wrapper, query).await?;
            Ok(res.rows.into_iter().filter_map(|v| {
                let obj = v.as_object()?;
                Some(ColumnInfo {
                    name: obj.get("name")?.as_str()?.to_string(),
                    data_type: obj.get("data_type")?.as_str()?.to_string(),
                    is_nullable: obj.get("is_nullable").and_then(|v| v.as_str()).map(|s| s == "YES"),
                    default_value: obj.get("default_value").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    max_length: obj.get("max_length").and_then(|v| v.as_i64()).map(|n| n as i32),
                })
            }).collect())
        }
        DbType::Mongodb => {
            let info = wrapper.get_info()?;
            let url = build_connection_url(&info);
            let client = Client::with_uri_str(&url).await.map_err(|e| e.to_string())?;
            let db_name = match &url.split('/').last() {
                Some(name) if !name.is_empty() => name.split('?').next().unwrap_or("test"),
                _ => "test",
            };
            let db = client.database(db_name);
            let collection = db.collection::<Document>(&table);
            if let Some(doc) = collection.find_one(doc! {}).await.map_err(|e| e.to_string())? {
                Ok(doc.keys().map(|k| ColumnInfo { name: k.clone(), data_type: "dynamic".to_string(), is_nullable: Some(true), default_value: None, max_length: None }).collect())
            } else {
                Ok(vec![])
            }
        }
        DbType::Sqlserver => {
            let query = format!(
                "SELECT COLUMN_NAME as [name], DATA_TYPE as [data_type], IS_NULLABLE as [is_nullable], COLUMN_DEFAULT as [default_value], CHARACTER_MAXIMUM_LENGTH as [max_length] 
                 FROM INFORMATION_SCHEMA.COLUMNS 
                 WHERE TABLE_NAME = '{}' AND TABLE_SCHEMA = '{}'",
                table,
                schema.unwrap_or_else(|| "dbo".to_string())
            );
            
            #[cfg(windows)]
            if info.use_entra_id.unwrap_or(false) || (info.user.as_deref().unwrap_or("").is_empty() && !info.host.is_empty()) {
                let res = execute_query_odbc(info, query).await?;
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

            let mut client = get_sqlserver_client(&info).await?;
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
    }
}
