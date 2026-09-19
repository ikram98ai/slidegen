//! Thin MCP stdio server. Tools wrap the v1 service API so hosts never
//! talk to Qdrant or S3 directly.
//!
//! Tools: `search_book`, `get_chapter`, `get_scene`, `get_citation`.

use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

const PROTOCOL: &str = "2024-11-05";

#[derive(Clone)]
pub struct SlidegenHttp {
    client: Client,
    base_url: String,
    api_key: String,
}

impl SlidegenHttp {
    pub fn new(base_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.into().trim_end_matches('/').to_string(),
            api_key: api_key.into(),
        }
    }

    async fn get(&self, path: &str) -> Result<Value> {
        let url = format!("{}{path}", self.base_url);
        let resp = self
            .client
            .get(&url)
            .header("X-Api-Key", &self.api_key)
            .send()
            .await
            .with_context(|| format!("GET {path}"))?;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            anyhow::bail!("{path} failed ({status}): {body}");
        }
        serde_json::from_str(&body).context("invalid JSON from slidegen")
    }

    async fn post(&self, path: &str, body: &Value) -> Result<Value> {
        let url = format!("{}{path}", self.base_url);
        let resp = self
            .client
            .post(&url)
            .header("X-Api-Key", &self.api_key)
            .json(body)
            .send()
            .await
            .with_context(|| format!("POST {path}"))?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            anyhow::bail!("{path} failed ({status}): {text}");
        }
        serde_json::from_str(&text).context("invalid JSON from slidegen")
    }

    pub async fn search_book(
        &self,
        book_id: &str,
        query: &str,
        chapter_id: Option<&str>,
    ) -> Result<Value> {
        let mut body = json!({ "query": query });
        if let Some(chapter_id) = chapter_id {
            body["chapter_id"] = json!(chapter_id);
        }
        self.post(&format!("/api/v1/books/{book_id}/search"), &body)
            .await
    }

    pub async fn get_chapter(&self, book_id: &str, chapter_id: &str) -> Result<Value> {
        self.get(&format!("/api/v1/books/{book_id}/chapters/{chapter_id}"))
            .await
    }

    pub async fn get_scene(
        &self,
        book_id: &str,
        chapter_id: &str,
        scene_id: &str,
    ) -> Result<Value> {
        self.get(&format!(
            "/api/v1/books/{book_id}/chapters/{chapter_id}/scenes/{scene_id}"
        ))
        .await
    }

    pub async fn get_citation(&self, book_id: &str, paragraph_id: &str) -> Result<Value> {
        self.get(&format!(
            "/api/v1/books/{book_id}/paragraphs/{paragraph_id}"
        ))
        .await
    }
}

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: Option<String>,
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
}

pub fn tools_list() -> Value {
    json!({
        "tools": [
            {
                "name": "search_book",
                "description": "Search a book for cited passages. Always returns page and paragraph_id.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "book_id": { "type": "string" },
                        "query": { "type": "string" },
                        "chapter_id": { "type": "string" }
                    },
                    "required": ["book_id", "query"]
                }
            },
            {
                "name": "get_chapter",
                "description": "Get chapter metadata, package status, and embed URL.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "book_id": { "type": "string" },
                        "chapter_id": { "type": "string" }
                    },
                    "required": ["book_id", "chapter_id"]
                }
            },
            {
                "name": "get_scene",
                "description": "Get one scene (summary, viz, citations) from a compiled chapter.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "book_id": { "type": "string" },
                        "chapter_id": { "type": "string" },
                        "scene_id": { "type": "string" }
                    },
                    "required": ["book_id", "chapter_id", "scene_id"]
                }
            },
            {
                "name": "get_citation",
                "description": "Resolve a paragraph_id to its quote and printed/PDF page.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "book_id": { "type": "string" },
                        "paragraph_id": { "type": "string" }
                    },
                    "required": ["book_id", "paragraph_id"]
                }
            }
        ]
    })
}

pub fn initialize_result() -> Value {
    json!({
        "protocolVersion": PROTOCOL,
        "capabilities": { "tools": {} },
        "serverInfo": { "name": "slidegen", "version": "0.1.0" }
    })
}

fn text_result(value: &Value) -> Value {
    json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
        }]
    })
}

fn required_str(params: &Value, key: &str) -> Result<String> {
    params
        .get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .with_context(|| format!("missing {key}"))
}

pub async fn dispatch_tool(client: &SlidegenHttp, name: &str, arguments: &Value) -> Result<Value> {
    match name {
        "search_book" => {
            let book_id = required_str(arguments, "book_id")?;
            let query = required_str(arguments, "query")?;
            let chapter_id = arguments.get("chapter_id").and_then(Value::as_str);
            client
                .search_book(&book_id, &query, chapter_id)
                .await
                .map(|v| text_result(&v))
        }
        "get_chapter" => {
            let book_id = required_str(arguments, "book_id")?;
            let chapter_id = required_str(arguments, "chapter_id")?;
            client
                .get_chapter(&book_id, &chapter_id)
                .await
                .map(|v| text_result(&v))
        }
        "get_scene" => {
            let book_id = required_str(arguments, "book_id")?;
            let chapter_id = required_str(arguments, "chapter_id")?;
            let scene_id = required_str(arguments, "scene_id")?;
            client
                .get_scene(&book_id, &chapter_id, &scene_id)
                .await
                .map(|v| text_result(&v))
        }
        "get_citation" => {
            let book_id = required_str(arguments, "book_id")?;
            let paragraph_id = required_str(arguments, "paragraph_id")?;
            client
                .get_citation(&book_id, &paragraph_id)
                .await
                .map(|v| text_result(&v))
        }
        other => anyhow::bail!("unknown tool {other}"),
    }
}

fn handle_sync(req: &JsonRpcRequest) -> Option<JsonRpcResponse> {
    match req.method.as_str() {
        "initialize" => Some(ok(req.id.clone(), initialize_result())),
        "tools/list" => Some(ok(req.id.clone(), tools_list())),
        "ping" => Some(ok(req.id.clone(), json!({}))),
        "notifications/initialized" | "notifications/cancelled" => None,
        _ => None,
    }
}

pub async fn handle_line(client: &SlidegenHttp, line: &str) -> Option<String> {
    let req: JsonRpcRequest = match serde_json::from_str(line) {
        Ok(r) => r,
        Err(e) => {
            return Some(serialize(&err(None, -32700, format!("parse error: {e}"))));
        }
    };
    if req.jsonrpc.as_deref().is_some_and(|v| v != "2.0") && req.jsonrpc.is_some() {
        return Some(serialize(&err(
            req.id,
            -32600,
            "jsonrpc must be 2.0".into(),
        )));
    }
    if let Some(resp) = handle_sync(&req) {
        return Some(serialize(&resp));
    }
    if req.method == "tools/call" {
        let name = req
            .params
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let arguments = req.params.get("arguments").cloned().unwrap_or(json!({}));
        let resp = match dispatch_tool(client, &name, &arguments).await {
            Ok(result) => ok(req.id, result),
            Err(e) => err(req.id, -32000, format!("{e:#}")),
        };
        return Some(serialize(&resp));
    }
    Some(serialize(&err(
        req.id,
        -32601,
        format!("method not found: {}", req.method),
    )))
}

fn ok(id: Option<Value>, result: Value) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0",
        id,
        result: Some(result),
        error: None,
    }
}

fn err(id: Option<Value>, code: i32, message: String) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0",
        id,
        result: None,
        error: Some(JsonRpcError { code, message }),
    }
}

fn serialize(resp: &JsonRpcResponse) -> String {
    serde_json::to_string(resp).unwrap_or_else(|_| {
        r#"{"jsonrpc":"2.0","error":{"code":-32603,"message":"serialize"}}"#.into()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_the_four_tools() {
        let listed = tools_list();
        let names: Vec<&str> = listed["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap())
            .collect();
        assert_eq!(
            names,
            ["search_book", "get_chapter", "get_scene", "get_citation"]
        );
    }

    #[test]
    fn initialize_is_sync() {
        let req = JsonRpcRequest {
            jsonrpc: Some("2.0".into()),
            id: Some(json!(1)),
            method: "initialize".into(),
            params: json!({}),
        };
        let resp = handle_sync(&req).unwrap();
        assert_eq!(resp.result.unwrap()["protocolVersion"], PROTOCOL);
    }

    #[test]
    fn required_str_rejects_empty() {
        assert!(required_str(&json!({"book_id": ""}), "book_id").is_err());
        assert_eq!(
            required_str(&json!({"book_id": "b1"}), "book_id").unwrap(),
            "b1"
        );
    }
}
