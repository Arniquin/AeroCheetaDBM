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
use odbc_api::{Environment, ConnectionOptions, ResultSetMetadata, buffers::TextRowSet};

#[cfg(windows)]
static ODBC_ENV: OnceLock<Environment> = OnceLock::new();

#[cfg(windows)]
fn get_odbc_env() -> Result<&'static Environment, String> {
    ODBC_ENV.get_or_try_init(|| {
        Environment::new().map_err(|e| e.to_string())
    })
}

#[derive(Debug, Deserialize, Serialize, Clone)]
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
}

#[derive(Debug, Serialize)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
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
                } else if s.starts_with("sqlserver://") || s_lower.contains("initial catalog=") {
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
fn build_odbc_connection_string(info: &ConnectionInfo) -> String {
    let mut parts = Vec::new();
    
    // Default to Driver 18 which is common for modern Entra ID support
    parts.push("Driver={ODBC Driver 18 for SQL Server}".to_string());
    
    if !info.host.is_empty() {
        parts.push(format!("Server={},{}", info.host, info.port));
    }

    if let Some(db) = &info.database {
        if !db.trim().is_empty() && !db.contains('=') {
            parts.push(format!("Database={}", db));
        }
    }

    if info.use_entra_id.unwrap_or(false) {
        parts.push("Authentication=ActiveDirectoryIntegrated".to_string());
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

    parts.join(";")
}

#[cfg(windows)]
async fn test_connection_odbc(info: ConnectionInfo) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        let env = get_odbc_env()?;
        let conn_str = build_odbc_connection_string(&info);
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
        let conn_str = build_odbc_connection_string(&info);
        let conn = env.connect_with_connection_string(&conn_str, ConnectionOptions::default())
            .map_err(|e| format!("ODBC Connection failed: {}", e))?;
        
        let cursor = conn.execute(&query, ()).map_err(|e| e.to_string())?;
        
        let mut columns = Vec::new();
        let mut rows = Vec::new();
        
        if let Some(mut cursor) = cursor {
            let num_cols = cursor.num_result_cols().map_err(|e| e.to_string())?;
            for i in 1..=num_cols {
                let mut name = String::new();
                cursor.col_name(i as u16, &mut name).map_err(|e| e.to_string())?;
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
                TableInfo { name: "users".to_string(), schema: Some("public".to_string()) },
                TableInfo { name: "products".to_string(), schema: Some("public".to_string()) },
            ])
        }
        DbType::Postgres => {
            let query = "SELECT table_name as name, table_schema as schema FROM information_schema.tables WHERE table_schema NOT IN ('information_schema', 'pg_catalog')";
            let res = execute_query(wrapper, query.to_string()).await?;
            Ok(res.rows.into_iter().filter_map(|v| {
                let obj = v.as_object()?;
                Some(TableInfo {
                    name: obj.get("name")?.as_str()?.to_string(),
                    schema: obj.get("schema")?.as_str().map(|s| s.to_string()),
                })
            }).collect())
        }
        DbType::Mysql => {
            let query = "SELECT table_name as name FROM information_schema.tables WHERE table_schema = DATABASE()";
            let res = execute_query(wrapper, query.to_string()).await?;
            Ok(res.rows.into_iter().filter_map(|v| {
                let obj = v.as_object()?;
                Some(TableInfo {
                    name: obj.get("name")?.as_str()?.to_string(),
                    schema: None,
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
            Ok(collections.into_iter().map(|name| TableInfo { name, schema: None }).collect())
        }
        DbType::Sqlserver => {
            #[cfg(windows)]
            if info.use_entra_id.unwrap_or(false) || (info.user.as_deref().unwrap_or("").is_empty() && !info.host.is_empty()) {
                let res = execute_query_odbc(info, "SELECT TABLE_NAME as [name], TABLE_SCHEMA as [schema] FROM INFORMATION_SCHEMA.TABLES WHERE TABLE_TYPE = 'BASE TABLE'".to_string()).await?;
                return Ok(res.rows.into_iter().filter_map(|v| {
                    let obj = v.as_object()?;
                    Some(TableInfo {
                        name: obj.get("name")?.as_str()?.to_string(),
                        schema: obj.get("schema")?.as_str().map(|s| s.to_string()),
                    })
                }).collect());
            }

            let mut client = get_sqlserver_client(&info).await?;
            let query = "SELECT TABLE_NAME as [name], TABLE_SCHEMA as [schema] FROM INFORMATION_SCHEMA.TABLES WHERE TABLE_TYPE = 'BASE TABLE'";
            let mut stream = client.query(query, &[]).await.map_err(|e| e.to_string())?;
            
            let mut results = Vec::new();
            while let Some(item) = stream.next().await {
                match item {
                    Ok(tiberius::QueryItem::Row(row)) => {
                        results.push(TableInfo {
                            name: row.try_get::<&str, _>(0).unwrap_or(Some("")).unwrap_or("").to_string(),
                            schema: row.try_get::<&str, _>(1).unwrap_or(None).map(|s| s.to_string()),
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
                    ColumnInfo { name: "id".to_string(), data_type: "integer".to_string() },
                    ColumnInfo { name: "name".to_string(), data_type: "varchar".to_string() },
                    ColumnInfo { name: "email".to_string(), data_type: "varchar".to_string() },
                ])
            } else {
                Ok(vec![
                    ColumnInfo { name: "id".to_string(), data_type: "integer".to_string() },
                    ColumnInfo { name: "title".to_string(), data_type: "varchar".to_string() },
                    ColumnInfo { name: "price".to_string(), data_type: "numeric".to_string() },
                ])
            }
        }
        DbType::Postgres => {
            let query = format!(
                "SELECT column_name as name, data_type FROM information_schema.columns WHERE table_name = '{}' AND table_schema = '{}'",
                table,
                schema.unwrap_or_else(|| "public".to_string())
            );
            let res = execute_query(wrapper, query).await?;
            Ok(res.rows.into_iter().filter_map(|v| {
                let obj = v.as_object()?;
                Some(ColumnInfo {
                    name: obj.get("name")?.as_str()?.to_string(),
                    data_type: obj.get("data_type")?.as_str()?.to_string(),
                })
            }).collect())
        }
        DbType::Mysql => {
            let query = format!(
                "SELECT column_name as name, data_type FROM information_schema.columns WHERE table_name = '{}' AND table_schema = DATABASE()",
                table
            );
            let res = execute_query(wrapper, query).await?;
            Ok(res.rows.into_iter().filter_map(|v| {
                let obj = v.as_object()?;
                Some(ColumnInfo {
                    name: obj.get("name")?.as_str()?.to_string(),
                    data_type: obj.get("data_type")?.as_str()?.to_string(),
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
                Ok(doc.keys().map(|k| ColumnInfo { name: k.clone(), data_type: "dynamic".to_string() }).collect())
            } else {
                Ok(vec![])
            }
        }
        DbType::Sqlserver => {
            #[cfg(windows)]
            if info.use_entra_id.unwrap_or(false) || (info.user.as_deref().unwrap_or("").is_empty() && !info.host.is_empty()) {
                let query = format!(
                    "SELECT COLUMN_NAME as [name], DATA_TYPE as [data_type] FROM INFORMATION_SCHEMA.COLUMNS WHERE TABLE_NAME = '{}' AND TABLE_SCHEMA = '{}'",
                    table,
                    schema.unwrap_or_else(|| "dbo".to_string())
                );
                let res = execute_query_odbc(info, query).await?;
                return Ok(res.rows.into_iter().filter_map(|v| {
                    let obj = v.as_object()?;
                    Some(ColumnInfo {
                        name: obj.get("name")?.as_str()?.to_string(),
                        data_type: obj.get("data_type")?.as_str()?.to_string(),
                    })
                }).collect());
            }

            let mut client = get_sqlserver_client(&info).await?;
            let query = format!(
                "SELECT COLUMN_NAME as [name], DATA_TYPE as [data_type] FROM INFORMATION_SCHEMA.COLUMNS WHERE TABLE_NAME = '{}' AND TABLE_SCHEMA = '{}'",
                table,
                schema.unwrap_or_else(|| "dbo".to_string())
            );
            let mut stream = client.query(query, &[]).await.map_err(|e| e.to_string())?;
            
            let mut results = Vec::new();
            while let Some(item) = stream.next().await {
                match item {
                    Ok(tiberius::QueryItem::Row(row)) => {
                        results.push(ColumnInfo {
                            name: row.try_get::<&str, _>(0).unwrap_or(Some("")).unwrap_or("").to_string(),
                            data_type: row.try_get::<&str, _>(1).unwrap_or(Some("")).unwrap_or("").to_string(),
                        });
                    }
                    _ => {}
                }
            }
            Ok(results)
        }
    }
}
