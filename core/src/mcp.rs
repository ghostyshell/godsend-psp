//! Loopback Model Context Protocol server.
//!
//! A small, dependency-free JSON-RPC 2.0 endpoint over HTTP/1.1 (POST with a
//! `Content-Length` body, one response per connection, no keep-alive). It
//! exposes read-only device, library, store, downloads, plugins and doctor
//! data to MCP clients. Downloads and every mutating operation stay out of the
//! tool surface on purpose.

use crate::config::Config;
use crate::{aria2, device, doctor, fm, library, plugins, sources};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::thread::JoinHandle;

/// MCP protocol revision this server speaks.
pub const PROTOCOL_VERSION: &str = "2024-11-05";

/// Largest accepted request body (8 MiB).
const MAX_BODY_BYTES: usize = 8 * 1024 * 1024;

/// Endpoint settings derived from [`Config`].
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub enabled: bool,
    pub port: u16,
}

impl ServerConfig {
    pub fn from_config(config: &Config) -> Self {
        Self {
            enabled: config.mcp_enabled,
            port: config.mcp_port,
        }
    }

    /// Endpoint URL shown in Settings and printed by the CLI.
    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }
}

/// Bind the loopback port and serve connections on a background thread.
///
/// Binding happens before the thread starts, so a busy port surfaces as an
/// error the caller can log instead of a panic. The thread runs until the
/// process exits.
pub fn serve(config: &Config) -> crate::Result<JoinHandle<()>> {
    let server = ServerConfig::from_config(config);
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, server.port))?;
    let config = config.clone();
    let handle = std::thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    if let Err(err) = handle_connection(&config, stream) {
                        tracing::warn!("mcp connection failed: {}", err);
                    }
                }
                Err(err) => tracing::warn!("mcp accept failed: {}", err),
            }
        }
    });
    Ok(handle)
}

/// Read one HTTP request, dispatch its JSON-RPC body and close the connection.
fn handle_connection(config: &Config, stream: TcpStream) -> crate::Result<()> {
    let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(10)));
    let _ = stream.set_write_timeout(Some(std::time::Duration::from_secs(10)));
    let mut reader = BufReader::new(stream);

    let mut request_line = String::new();
    if reader.read_line(&mut request_line)? == 0 {
        return Ok(());
    }
    let method = request_line
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_ascii_uppercase();

    let mut content_length: usize = 0;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            break;
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            if name.trim().eq_ignore_ascii_case("content-length") {
                content_length = value.trim().parse().unwrap_or(0);
            }
        }
    }

    if method != "POST" {
        return write_response(
            reader.get_mut(),
            405,
            "Method Not Allowed",
            Some("text/plain; charset=utf-8"),
            b"POST only\n",
        );
    }
    if content_length == 0 || content_length > MAX_BODY_BYTES {
        return write_response(
            reader.get_mut(),
            400,
            "Bad Request",
            Some("text/plain; charset=utf-8"),
            b"missing or oversized body\n",
        );
    }

    let mut body = vec![0u8; content_length];
    reader.read_exact(&mut body)?;

    let response = match serde_json::from_slice::<Value>(&body) {
        Ok(request) => handle_request(config, &request),
        Err(_) => Some(error_response(
            Value::Null,
            -32700,
            "parse error: body is not JSON",
        )),
    };

    match response {
        Some(value) => {
            let bytes = serde_json::to_vec(&value)?;
            write_response(
                reader.get_mut(),
                200,
                "OK",
                Some("application/json"),
                &bytes,
            )
        }
        None => write_response(reader.get_mut(), 204, "No Content", None, b""),
    }
}

fn write_response(
    stream: &mut TcpStream,
    status: u16,
    reason: &str,
    content_type: Option<&str>,
    body: &[u8],
) -> crate::Result<()> {
    let mut head = format!("HTTP/1.1 {} {}\r\n", status, reason);
    head.push_str("Connection: close\r\n");
    head.push_str("Cache-Control: no-store\r\n");
    if let Some(content_type) = content_type {
        head.push_str(&format!("Content-Type: {}\r\n", content_type));
    }
    head.push_str(&format!("Content-Length: {}\r\n\r\n", body.len()));
    stream.write_all(head.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()?;
    Ok(())
}

/// Pure JSON-RPC dispatch. `None` means the request needs no response (a
/// notification). Kept socket-free so it can be unit tested directly.
pub fn handle_request(config: &Config, req: &Value) -> Option<Value> {
    let id = req.get("id").cloned();
    let method = req.get("method").and_then(Value::as_str).unwrap_or("");

    match method {
        "initialize" => Some(result_response(
            id.unwrap_or(Value::Null),
            json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": { "tools": {} },
                "serverInfo": {
                    "name": "godsend-psp",
                    "version": env!("CARGO_PKG_VERSION"),
                },
            }),
        )),
        "notifications/initialized" | "notifications/cancelled" | "initialized" => None,
        "ping" => Some(result_response(id.unwrap_or(Value::Null), json!({}))),
        "tools/list" => Some(result_response(
            id.unwrap_or(Value::Null),
            json!({ "tools": tool_definitions() }),
        )),
        "tools/call" => Some(tools_call_response(config, id, req.get("params"))),
        _ => {
            // Unknown notifications must stay silent.
            let id = id?;
            Some(error_response(id, -32601, "method not found"))
        }
    }
}

fn result_response(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn error_response(id: Value, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message },
    })
}

fn tools_call_response(config: &Config, id: Option<Value>, params: Option<&Value>) -> Value {
    let id = id.unwrap_or(Value::Null);
    let params = match params {
        Some(params) => params,
        None => return error_response(id, -32602, "invalid params: missing tool name"),
    };
    let name = params.get("name").and_then(Value::as_str).unwrap_or("");
    if name.is_empty() {
        return error_response(id, -32602, "invalid params: missing tool name");
    }
    let arguments = params.get("arguments").cloned().unwrap_or_else(|| json!({}));

    match call_tool(config, name, &arguments) {
        Ok(value) => {
            let text = serde_json::to_string_pretty(&value)
                .unwrap_or_else(|_| String::from("null"));
            result_response(
                id,
                json!({
                    "content": [{ "type": "text", "text": text }],
                    "isError": false,
                }),
            )
        }
        Err(err) => {
            // Tool failures are reported in-band so the model can read them.
            result_response(
                id,
                json!({
                    "content": [{ "type": "text", "text": format!("error: {}", err) }],
                    "isError": true,
                }),
            )
        }
    }
}

/// Run one read-only tool. `store_download` and every mutation are excluded.
fn call_tool(config: &Config, name: &str, args: &Value) -> crate::Result<Value> {
    let value = match name {
        "devices" => to_json(device::scan_volumes())?,
        "library_scan" => {
            let (_, paths) = device::require_psp()?;
            to_json(library::scan(&paths)?)?
        }
        "local_library" => to_json(fm::local_library(&aria2::download_dir(config))?)?,
        "store_search" => {
            let query = args.get("query").and_then(Value::as_str).unwrap_or("");
            let source = args.get("source").and_then(Value::as_str).unwrap_or("");
            match source {
                "" | "gamesearch" => {
                    let hits = sources::gamesearch(&config.gamesearch_base, query, 30)?;
                    json!({ "kind": "gamesearch", "items": hits })
                }
                "ia" => {
                    let query = if query.is_empty() {
                        sources::ia_seed_queries()[0].to_string()
                    } else {
                        query.to_string()
                    };
                    let items = sources::ia_search(&query, 40)?;
                    json!({ "kind": "ia", "items": items })
                }
                "rroms" => {
                    let items = sources::rroms_items()?;
                    json!({ "kind": "rroms", "items": items })
                }
                "nps" => {
                    let entries = sources::nps_search(query, 40)?;
                    json!({ "kind": "nps", "items": entries })
                }
                other => {
                    return Err(crate::Error::Invalid(format!(
                        "unknown source {}: use gamesearch, ia, rroms or nps",
                        other
                    )))
                }
            }
        }
        "store_detail" => {
            let source = args.get("source").and_then(Value::as_str).unwrap_or("");
            let id = args.get("id").and_then(Value::as_str).unwrap_or("");
            match source {
                "" | "gamesearch" => {
                    if id.is_empty() {
                        return Err(crate::Error::Invalid("missing id".into()));
                    }
                    let hit = sources::gamesearch_detail(&config.gamesearch_base, id)?;
                    json!({ "kind": "gamesearch", "hit": hit })
                }
                "ia" => {
                    if id.is_empty() {
                        return Err(crate::Error::Invalid("missing id".into()));
                    }
                    let files = sources::ia_item_files(id)?;
                    json!({ "kind": "ia", "files": files })
                }
                "rroms" => {
                    if id.is_empty() {
                        return Err(crate::Error::Invalid("missing id".into()));
                    }
                    let files = sources::rroms_files(id)?;
                    json!({ "kind": "rroms", "files": files })
                }
                other => {
                    return Err(crate::Error::Invalid(format!(
                        "unknown source {}: use gamesearch, ia or rroms",
                        other
                    )))
                }
            }
        }
        "downloads_list" => {
            let dir = aria2::download_dir(config);
            let files = if dir.is_dir() {
                fm::list_dir(&dir)?
            } else {
                Vec::new()
            };
            json!({ "dir": dir.to_string_lossy(), "files": files })
        }
        "plugins_list" => {
            let (_, paths) = device::require_psp()?;
            let mut out = serde_json::Map::new();
            for (key, section) in [
                ("game", plugins::Section::Game),
                ("vsh", plugins::Section::Vsh),
                ("pops", plugins::Section::Pops),
            ] {
                out.insert(
                    key.to_string(),
                    to_json(plugins::list(&paths, section)?)?,
                );
            }
            Value::Object(out)
        }
        "doctor" => to_json(doctor::run(config))?,
        other => {
            return Err(crate::Error::Invalid(format!("unknown tool {}", other)));
        }
    };
    Ok(value)
}

fn to_json<T: serde::Serialize>(value: T) -> crate::Result<Value> {
    Ok(serde_json::to_value(value)?)
}

/// Tool metadata advertised by `tools/list`.
pub fn tool_definitions() -> Vec<Value> {
    vec![
        tool(
            "devices",
            "List mounted volumes and flag the PSP memory sticks.",
            json!({ "type": "object", "properties": {} }),
        ),
        tool(
            "library_scan",
            "Scan the connected memory stick for installed games, homebrew and disc images.",
            json!({ "type": "object", "properties": {} }),
        ),
        tool(
            "local_library",
            "List PSP content already downloaded to the local download folder.",
            json!({ "type": "object", "properties": {} }),
        ),
        tool(
            "store_search",
            "Search the game-search index, Internet Archive, r-roms or NoPayStation for PSP content.",
            json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Search text. Empty browses the source default." },
                    "source": {
                        "type": "string",
                        "enum": ["gamesearch", "ia", "rroms", "nps"],
                        "description": "gamesearch (default), ia, rroms or nps",
                    },
                },
            }),
        ),
        tool(
            "store_detail",
            "Fetch one store hit: game links for game-search, files for an Internet Archive item or r-roms entry.",
            json!({
                "type": "object",
                "properties": {
                    "source": {
                        "type": "string",
                        "enum": ["gamesearch", "ia", "rroms"],
                        "description": "gamesearch (default), ia or rroms",
                    },
                    "id": { "type": "string", "description": "game-search slug, IA identifier, or r-roms item id" },
                },
                "required": ["id"],
            }),
        ),
        tool(
            "downloads_list",
            "List the download folder with its files and sizes.",
            json!({ "type": "object", "properties": {} }),
        ),
        tool(
            "plugins_list",
            "List seplugins entries for game, vsh and pops with their enabled flags.",
            json!({ "type": "object", "properties": {} }),
        ),
        tool(
            "doctor",
            "Run environment checks: data dir, ffmpeg, ffprobe, device, catalog cache.",
            json!({ "type": "object", "properties": {} }),
        ),
    ]
}

fn tool(name: &str, description: &str, input_schema: Value) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": input_schema,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(config: &Config, req: Value) -> Value {
        handle_request(config, &req).expect("request must produce a response")
    }

    #[test]
    fn initialize_returns_protocol_and_server_info() {
        let response = call(
            &Config::default(),
            json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {} }),
        );
        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], 1);
        assert_eq!(response["result"]["protocolVersion"], PROTOCOL_VERSION);
        assert_eq!(response["result"]["serverInfo"]["name"], "godsend-psp");
        assert_eq!(
            response["result"]["serverInfo"]["version"],
            env!("CARGO_PKG_VERSION")
        );
        assert!(response["result"]["capabilities"]["tools"].is_object());
    }

    #[test]
    fn tools_list_advertises_read_only_tools() {
        let response = call(
            &Config::default(),
            json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }),
        );
        let tools = response["result"]["tools"]
            .as_array()
            .expect("tools must be an array");
        let names: Vec<&str> = tools
            .iter()
            .filter_map(|tool| tool["name"].as_str())
            .collect();
        for expected in [
            "devices",
            "library_scan",
            "local_library",
            "store_search",
            "store_detail",
            "downloads_list",
            "plugins_list",
            "doctor",
        ] {
            assert!(names.contains(&expected), "missing tool {}", expected);
        }
        assert!(!names.contains(&"store_download"), "store_download stays out");
        for tool in tools {
            assert_eq!(tool["inputSchema"]["type"], "object");
        }
    }

    #[test]
    fn tools_call_devices_returns_text_content() {
        let response = call(
            &Config::default(),
            json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "tools/call",
                "params": { "name": "devices", "arguments": {} },
            }),
        );
        let content = response["result"]["content"]
            .as_array()
            .expect("content must be an array");
        assert_eq!(content.len(), 1);
        assert_eq!(content[0]["type"], "text");
        let text = content[0]["text"].as_str().expect("text content");
        assert!(!text.is_empty());
        assert_eq!(response["result"]["isError"], false);
    }

    #[test]
    fn initialized_notification_stays_silent() {
        assert!(handle_request(
            &Config::default(),
            &json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
        )
        .is_none());
    }

    #[test]
    fn unknown_tool_reports_in_band_error() {
        let response = call(
            &Config::default(),
            json!({
                "jsonrpc": "2.0",
                "id": 4,
                "method": "tools/call",
                "params": { "name": "store_download", "arguments": {} },
            }),
        );
        assert_eq!(response["result"]["isError"], true);
        let text = response["result"]["content"][0]["text"]
            .as_str()
            .expect("text content");
        assert!(text.contains("unknown tool"));
    }
}
