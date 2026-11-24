use sqlx::{Row, Column, Connection};
use std::time::Instant;
use crate::db::models::{ConnectionInfo, QueryResult, TableInfo, ColumnInfo};
use crate::db::utils::build_connection_url;

pub async fn test_connection(info: &ConnectionInfo) -> Result<String, String> {
    let url = build_connection_url(info);
    match sqlx::PgConnection::connect(&url).await {
        Ok(_) => Ok("Connection successful".to_string()),
        Err(e) => Err(format!("Postgres connection failed: {}", e)),
    }
}

fn decode_column(row: &sqlx::postgres::PgRow, i: usize) -> serde_json::Value {
    // Try JSON first
    if let Ok(Some(v)) = row.try_get::<Option<serde_json::Value>, _>(i) {
        return v;
    }
    // Fallback to common types to avoid panics on non-JSON compatible types
    if let Ok(Some(s)) = row.try_get::<Option<String>, _>(i) {
        return serde_json::Value::String(s);
    }
    if let Ok(Some(n)) = row.try_get::<Option<i64>, _>(i) {
        return serde_json::Value::Number(n.into());
    }
    if let Ok(Some(f)) = row.try_get::<Option<f64>, _>(i) {
        if let Some(num) = serde_json::Number::from_f64(f) {
            return serde_json::Value::Number(num);
        }
    }
    if let Ok(Some(b)) = row.try_get::<Option<bool>, _>(i) {
        return serde_json::Value::Bool(b);
    }
    
    // Last resort: try to get the raw value and use its Debug representation if possible,
    // or just return Null to avoid a panic.
    serde_json::Value::Null
}

pub async fn execute_query(info: &ConnectionInfo, query: &str) -> Result<QueryResult, String> {
    let start = Instant::now();
    let url = build_connection_url(info);
    let pool = sqlx::PgPool::connect(&url).await.map_err(|e| e.to_string())?;
    let rows = sqlx::query(query).fetch_all(&pool).await.map_err(|e| e.to_string())?;
    
    let mut columns = Vec::new();
    if !rows.is_empty() {
        for col in rows[0].columns() {
            columns.push(col.name().to_string());
        }
    }

    let mut results = Vec::new();
    for row in rows {
        let mut map = serde_json::Map::new();
        for (i, col) in row.columns().iter().enumerate() {
            let val = decode_column(&row, i);
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

pub async fn get_databases(info: &ConnectionInfo) -> Result<Vec<String>, String> {
    let query = "SELECT datname FROM pg_database WHERE datistemplate = false";
    let res = execute_query(info, query).await?;
    Ok(res.rows.into_iter().filter_map(|v| {
        v.as_object()?.get("datname")?.as_str().map(|s| s.to_string())
    }).collect())
}

pub async fn get_tables(info: &ConnectionInfo) -> Result<Vec<TableInfo>, String> {
    let query = "SELECT table_name as name, table_schema as schema, current_database() as database FROM information_schema.tables WHERE table_schema NOT IN ('information_schema', 'pg_catalog')";
    let res = execute_query(info, query).await?;
    Ok(res.rows.into_iter().filter_map(|v| {
        let obj = v.as_object()?;
        Some(TableInfo {
            name: obj.get("name")?.as_str()?.to_string(),
            schema: obj.get("schema")?.as_str().map(|s| s.to_string()),
            database: obj.get("database")?.as_str().map(|s| s.to_string()),
        })
    }).collect())
}

pub async fn get_columns(info: &ConnectionInfo, table: &str, schema: Option<&str>) -> Result<Vec<ColumnInfo>, String> {
    let query = format!(
        "SELECT column_name as name, data_type, is_nullable, column_default as default_value, character_maximum_length as max_length 
         FROM information_schema.columns 
         WHERE table_name = '{}' AND table_schema = '{}'",
        table,
        schema.unwrap_or("public")
    );
    let res = execute_query(info, &query).await?;
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
