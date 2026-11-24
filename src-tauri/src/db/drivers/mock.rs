use std::time::Instant;
use crate::db::models::{QueryResult, TableInfo, ColumnInfo};

pub async fn test_connection() -> Result<String, String> {
    Ok("Connection successful (Mock)".to_string())
}

pub async fn execute_query(query: &str) -> Result<QueryResult, String> {
    let start = Instant::now();
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

pub async fn get_databases() -> Result<Vec<String>, String> {
    Ok(vec!["demo_db".to_string(), "test_db".to_string()])
}

pub async fn get_tables() -> Result<Vec<TableInfo>, String> {
    Ok(vec![
        TableInfo { name: "users".to_string(), schema: Some("public".to_string()), database: Some("demo_db".to_string()) },
        TableInfo { name: "products".to_string(), schema: Some("public".to_string()), database: Some("demo_db".to_string()) },
    ])
}

pub async fn get_columns(table: &str) -> Result<Vec<ColumnInfo>, String> {
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
