//! Styled HTML error pages for the Hive reverse proxy.
//!
//! Replaces plain-text error responses with branded HTML pages.
//! Templates are embedded at compile time via `include_str!`.
//! Supports `?format=plain` query parameter for LLM-friendly plain text output.

use crate::observability::{LogLevel, LogLine};
use axum::response::{Html, IntoResponse, Response};
use http::StatusCode;
use std::collections::HashMap;

const TEMPLATE_400: &str = include_str!("../templates/error_400.html");
const TEMPLATE_404: &str = include_str!("../templates/error_404.html");
const TEMPLATE_502: &str = include_str!("../templates/error_502.html");

/// Base URL for all shortcut links. Change this to update every link at once.
const SHORTCUT_BASE: &str = "https://adi.the-ihor.com/sc";

/// Build a full shortcut URL from a shortcut name.
fn shortcut_url(name: &str) -> String {
    format!("{}/{}", SHORTCUT_BASE, name)
}

/// Check if query string requests plain text format.
fn is_plain_format(query: Option<&str>) -> bool {
    query.map_or(false, |q| {
        q.split('&').any(|param| param == "format=plain")
    })
}


/// Replace `{{key}}` placeholders in a template with HTML-escaped values.
///
/// The `raw_keys` set contains keys whose values are inserted without escaping
/// (used for pre-rendered HTML like log output).
fn render(template: &str, vars: &HashMap<&str, String>, raw_keys: &[&str]) -> String {
    let mut result = template.to_string();
    for (key, value) in vars {
        let placeholder = format!("{{{{{}}}}}", key);
        let safe_value = if raw_keys.contains(key) {
            value.clone()
        } else {
            html_escape(value)
        };
        result = result.replace(&placeholder, &safe_value);
    }
    result
}

/// Escape HTML special characters to prevent XSS.
fn html_escape(s: &str) -> String {
    s.chars()
        .flat_map(|c| match c {
            '&' => vec!['&', 'a', 'm', 'p', ';'],
            '<' => vec!['&', 'l', 't', ';'],
            '>' => vec!['&', 'g', 't', ';'],
            '"' => vec!['&', 'q', 'u', 'o', 't', ';'],
            '\'' => vec!['&', '#', '3', '9', ';'],
            other => vec![other],
        })
        .collect()
}

/// Render log lines as HTML divs with level-based coloring.
fn format_logs(logs: &[LogLine]) -> String {
    if logs.is_empty() {
        return String::new();
    }

    let mut html = String::from(
        "<div class=\"spacer\"></div><div class=\"logs-section\"><div class=\"logs-title\">--- recent logs ---</div><div class=\"logs\">",
    );

    for line in logs {
        let class = match line.level {
            LogLevel::Error | LogLevel::Fatal => "log-error",
            LogLevel::Warn | LogLevel::Notice => "log-warn",
            LogLevel::Info => "log-info",
            _ => "log-debug",
        };
        let ts = line.timestamp.format("%H:%M:%S");
        let level = format!("{}", line.level).to_uppercase();
        let msg = html_escape(&line.message);
        html.push_str(&format!(
            "<div class=\"log-line {class}\">{ts} [{level}] {msg}</div>",
        ));
    }

    html.push_str("</div></div>");
    html
}

/// Render log lines as plain text.
fn format_logs_plain(logs: &[LogLine]) -> String {
    if logs.is_empty() {
        return String::new();
    }

    let mut text = String::from("\n--- recent logs ---\n");
    for line in logs {
        let ts = line.timestamp.format("%H:%M:%S");
        let level = format!("{}", line.level).to_uppercase();
        text.push_str(&format!("{ts} [{level}] {}\n", line.message));
    }
    text
}

fn base_vars(message: &str, path: &str, host: &str, code: u16) -> HashMap<&'static str, String> {
    let mut vars = HashMap::new();
    vars.insert("message", message.to_string());
    vars.insert("request_path", path.to_string());
    vars.insert("host", host.to_string());
    vars.insert("version", env!("CARGO_PKG_VERSION").to_string());
    vars.insert("timestamp", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string());
    vars.insert("link_llm", shortcut_url(&format!("hive-error-{}-llm", code)));
    vars.insert("link_user", shortcut_url(&format!("hive-error-{}", code)));
    vars
}

/// 404 Not Found response.
pub fn not_found(message: &str, path: &str, host: &str, query: Option<&str>) -> Response {
    if is_plain_format(query) {
        let body = format!(
            "WARN 404 Not Found\n\n{message}\n\npath      {path}\nhost      {host}\n\nhive {} | {}\n",
            env!("CARGO_PKG_VERSION"),
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
        );
        return (StatusCode::NOT_FOUND, [("content-type", "text/plain; charset=utf-8")], body).into_response();
    }
    let vars = base_vars(message, path, host, 404);
    let body = render(TEMPLATE_404, &vars, &[]);
    (StatusCode::NOT_FOUND, Html(body)).into_response()
}

/// 400 Bad Request response.
pub fn bad_request(message: &str, path: &str, host: &str, query: Option<&str>) -> Response {
    if is_plain_format(query) {
        let body = format!(
            "ERROR 400 Bad Request\n\n{message}\n\npath      {path}\nhost      {host}\n\nhive {} | {}\n",
            env!("CARGO_PKG_VERSION"),
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
        );
        return (StatusCode::BAD_REQUEST, [("content-type", "text/plain; charset=utf-8")], body).into_response();
    }
    let vars = base_vars(message, path, host, 400);
    let body = render(TEMPLATE_400, &vars, &[]);
    (StatusCode::BAD_REQUEST, Html(body)).into_response()
}

/// 502 Bad Gateway response, optionally including recent service logs.
pub fn bad_gateway(
    message: &str,
    path: &str,
    host: &str,
    service_name: &str,
    logs: Option<&[LogLine]>,
    query: Option<&str>,
) -> Response {
    if is_plain_format(query) {
        let logs_text = logs.map(format_logs_plain).unwrap_or_default();
        let body = format!(
            "ERROR 502 Bad Gateway\n\n{message}\n\nservice   {service_name}\npath      {path}\nhost      {host}\n{logs_text}\nhive {} | {}\n",
            env!("CARGO_PKG_VERSION"),
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
        );
        return (StatusCode::BAD_GATEWAY, [("content-type", "text/plain; charset=utf-8")], body).into_response();
    }
    let mut vars = base_vars(message, path, host, 502);
    vars.insert("service_name", service_name.to_string());
    vars.insert("logs", logs.map(format_logs).unwrap_or_default());
    let body = render(TEMPLATE_502, &vars, &["logs"]);
    (StatusCode::BAD_GATEWAY, Html(body)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use crate::observability::LogStream;

    #[test]
    fn test_html_escape() {
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
        assert_eq!(html_escape("a&b"), "a&amp;b");
        assert_eq!(html_escape("\"hi\""), "&quot;hi&quot;");
        assert_eq!(html_escape("it's"), "it&#39;s");
        assert_eq!(html_escape("plain"), "plain");
    }

    #[test]
    fn test_render_replaces_placeholders() {
        let template = "<p>{{greeting}}, {{name}}!</p>";
        let mut vars = HashMap::new();
        vars.insert("greeting", "Hello".to_string());
        vars.insert("name", "<b>World</b>".to_string());

        let result = render(template, &vars, &[]);
        assert_eq!(result, "<p>Hello, &lt;b&gt;World&lt;/b&gt;!</p>");
    }

    #[test]
    fn test_render_raw_keys_skip_escaping() {
        let template = "<div>{{content}}</div>";
        let mut vars = HashMap::new();
        vars.insert("content", "<b>bold</b>".to_string());

        let result = render(template, &vars, &["content"]);
        assert_eq!(result, "<div><b>bold</b></div>");
    }

    #[test]
    fn test_format_logs_empty() {
        assert_eq!(format_logs(&[]), "");
    }

    #[test]
    fn test_format_logs_renders_lines() {
        let logs = vec![
            LogLine {
                timestamp: Utc::now(),
                service_fqn: "default:web".to_string(),
                level: LogLevel::Error,
                message: "connection refused".to_string(),
                stream: LogStream::Stderr,
            },
            LogLine {
                timestamp: Utc::now(),
                service_fqn: "default:web".to_string(),
                level: LogLevel::Info,
                message: "starting up".to_string(),
                stream: LogStream::Stdout,
            },
        ];

        let html = format_logs(&logs);
        assert!(html.contains("log-error"));
        assert!(html.contains("log-info"));
        assert!(html.contains("connection refused"));
        assert!(html.contains("starting up"));
        assert!(html.contains("recent logs"));
    }

    #[test]
    fn test_format_logs_escapes_message() {
        let logs = vec![LogLine {
            timestamp: Utc::now(),
            service_fqn: "default:web".to_string(),
            level: LogLevel::Warn,
            message: "<script>alert('xss')</script>".to_string(),
            stream: LogStream::Stderr,
        }];

        let html = format_logs(&logs);
        assert!(!html.contains("<script>"));
        assert!(html.contains("&lt;script&gt;"));
    }

    #[test]
    fn test_is_plain_format() {
        assert!(is_plain_format(Some("format=plain")));
        assert!(is_plain_format(Some("foo=bar&format=plain")));
        assert!(is_plain_format(Some("format=plain&other=1")));
        assert!(!is_plain_format(Some("format=html")));
        assert!(!is_plain_format(Some("")));
        assert!(!is_plain_format(None));
    }

    #[test]
    fn test_format_logs_plain_empty() {
        assert_eq!(format_logs_plain(&[]), "");
    }

    #[test]
    fn test_format_logs_plain_renders() {
        let logs = vec![LogLine {
            timestamp: Utc::now(),
            service_fqn: "default:web".to_string(),
            level: LogLevel::Error,
            message: "connection refused".to_string(),
            stream: LogStream::Stderr,
        }];

        let text = format_logs_plain(&logs);
        assert!(text.contains("recent logs"));
        assert!(text.contains("[ERROR]"));
        assert!(text.contains("connection refused"));
    }
}
