use mongodb::{Client, bson::{Document, doc}};
use futures::StreamExt;
use std::time::Instant;
use crate::db::models::{ConnectionInfo, QueryResult, TableInfo, ColumnInfo};
use crate::db::utils::build_connection_url;

pub async fn test_connection(info: &ConnectionInfo) -> Result<String, String> {
    let url = build_connection_url(info);
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

pub async fn execute_query(info: &ConnectionInfo, query: &str) -> Result<QueryResult, String> {
    let start = Instant::now();
    let url = build_connection_url(info);
    let client = Client::with_uri_str(&url).await.map_err(|e| e.to_string())?;
    let db_name = match &url.split('/').last() {
        Some(name) if !name.is_empty() => name.split('?').next().unwrap_or("test"),
        _ => "test",
    };
    let db = client.database(db_name);
    
    let mut results = Vec::new();
    if query.trim().starts_with('{') {
        let doc: Document = serde_json::from_str(query).map_err(|e| e.to_string())?;
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

pub async fn get_databases(info: &ConnectionInfo) -> Result<Vec<String>, String> {
    let url = build_connection_url(info);
    let client = Client::with_uri_str(&url).await.map_err(|e| e.to_string())?;
    let db_names = client.list_database_names().await.map_err(|e| e.to_string())?;
    Ok(db_names)
}

pub async fn get_tables(info: &ConnectionInfo) -> Result<Vec<TableInfo>, String> {
    let url = build_connection_url(info);
    let client = Client::with_uri_str(&url).await.map_err(|e| e.to_string())?;
    let db_name = match &url.split('/').last() {
        Some(name) if !name.is_empty() => name.split('?').next().unwrap_or("test"),
        _ => "test",
    };
    let db = client.database(db_name);
    let collections = db.list_collection_names().await.map_err(|e| e.to_string())?;
    Ok(collections.into_iter().map(|name| TableInfo { name, schema: None, database: Some(db_name.to_string()) }).collect())
}

pub async fn get_columns(info: &ConnectionInfo, table: &str) -> Result<Vec<ColumnInfo>, String> {
    let url = build_connection_url(info);
    let client = Client::with_uri_str(&url).await.map_err(|e| e.to_string())?;
    let db_name = match &url.split('/').last() {
        Some(name) if !name.is_empty() => name.split('?').next().unwrap_or("test"),
        _ => "test",
    };
    let db = client.database(db_name);
    let collection = db.collection::<Document>(table);
    if let Some(doc) = collection.find_one(doc! {}).await.map_err(|e| e.to_string())? {
        Ok(doc.keys().map(|k| ColumnInfo { name: k.clone(), data_type: "dynamic".to_string(), is_nullable: Some(true), default_value: None, max_length: None }).collect())
    } else {
        Ok(vec![])
    }
}
