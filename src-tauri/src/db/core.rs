use crate::db::models::{DbType, QueryResult, TableInfo, ColumnInfo, ConnectionWrapper};
use crate::db::drivers::{postgres, mysql, mongodb, mock, sqlserver};

pub async fn get_databases(wrapper: ConnectionWrapper) -> Result<Vec<String>, String> {
    let info = wrapper.get_info()?;
    match info.db_type {
        DbType::Mock => mock::get_databases().await,
        DbType::Postgres => postgres::get_databases(&info).await,
        DbType::Mysql => mysql::get_databases(&info).await,
        DbType::Mongodb => mongodb::get_databases(&info).await,
        DbType::Sqlserver => sqlserver::get_databases(&info).await,
    }
}

pub async fn test_connection(wrapper: ConnectionWrapper) -> Result<String, String> {
    let info = wrapper.get_info()?;
    match info.db_type {
        DbType::Mock => mock::test_connection().await,
        DbType::Postgres => postgres::test_connection(&info).await,
        DbType::Mysql => mysql::test_connection(&info).await,
        DbType::Mongodb => mongodb::test_connection(&info).await,
        DbType::Sqlserver => sqlserver::test_connection(&info).await,
    }
}

pub async fn execute_query(wrapper: ConnectionWrapper, query: String) -> Result<QueryResult, String> {
    let info = wrapper.get_info()?;
    match info.db_type {
        DbType::Mock => mock::execute_query(&query).await,
        DbType::Postgres => postgres::execute_query(&info, &query).await,
        DbType::Mysql => mysql::execute_query(&info, &query).await,
        DbType::Mongodb => mongodb::execute_query(&info, &query).await,
        DbType::Sqlserver => sqlserver::execute_query(&info, &query).await,
    }
}

pub async fn get_tables(wrapper: ConnectionWrapper) -> Result<Vec<TableInfo>, String> {
    let info = wrapper.get_info()?;
    match info.db_type {
        DbType::Mock => mock::get_tables().await,
        DbType::Postgres => postgres::get_tables(&info).await,
        DbType::Mysql => mysql::get_tables(&info).await,
        DbType::Mongodb => mongodb::get_tables(&info).await,
        DbType::Sqlserver => sqlserver::get_tables(&info).await,
    }
}

pub async fn get_columns(wrapper: ConnectionWrapper, table: String, schema: Option<String>) -> Result<Vec<ColumnInfo>, String> {
    let info = wrapper.get_info()?;
    match info.db_type {
        DbType::Mock => mock::get_columns(&table).await,
        DbType::Postgres => postgres::get_columns(&info, &table, schema.as_deref()).await,
        DbType::Mysql => mysql::get_columns(&info, &table).await,
        DbType::Mongodb => mongodb::get_columns(&info, &table).await,
        DbType::Sqlserver => sqlserver::get_columns(&info, &table, schema.as_deref()).await,
    }
}
