use serde::{Deserialize, Serialize};

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
