use crate::db::models::{ConnectionInfo, DbType};

pub fn build_connection_url(info: &ConnectionInfo) -> String {
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
