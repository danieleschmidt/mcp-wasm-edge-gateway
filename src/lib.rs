use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

impl JsonRpcResponse {
    pub fn success(id: Option<Value>, result: Value) -> Self {
        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: Option<Value>, code: i32, message: String) -> Self {
        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message,
                data: None,
            }),
        }
    }
}

pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub handler: Box<dyn Fn(Option<Value>) -> Result<Value, String> + Send + Sync>,
}

pub struct McpGateway {
    tools: HashMap<String, ToolDef>,
}

impl McpGateway {
    pub fn new() -> Self {
        McpGateway {
            tools: HashMap::new(),
        }
    }

    pub fn register_tool(
        &mut self,
        name: &str,
        description: &str,
        handler: Box<dyn Fn(Option<Value>) -> Result<Value, String> + Send + Sync>,
    ) {
        self.tools.insert(
            name.to_string(),
            ToolDef {
                name: name.to_string(),
                description: description.to_string(),
                handler,
            },
        );
    }

    pub fn call_tool(&self, name: &str, params: Option<Value>) -> Result<Value, String> {
        match self.tools.get(name) {
            Some(tool) => (tool.handler)(params),
            None => Err(format!("Tool not found: {}", name)),
        }
    }

    pub fn list_tools(&self) -> Vec<&ToolDef> {
        self.tools.values().collect()
    }

    pub fn handle_jsonrpc(&self, request_json: &str) -> String {
        // Parse request
        let request: JsonRpcRequest = match serde_json::from_str(request_json) {
            Ok(r) => r,
            Err(e) => {
                let resp = JsonRpcResponse::error(
                    None,
                    -32700,
                    format!("Parse error: {}", e),
                );
                return serde_json::to_string(&resp).unwrap_or_default();
            }
        };

        // Notification (no id) — process but return empty string
        let is_notification = request.id.is_none();

        // Dispatch to tool
        let response = match self.call_tool(&request.method, request.params) {
            Ok(result) => {
                if is_notification {
                    return String::new();
                }
                JsonRpcResponse::success(request.id, result)
            }
            Err(_) => {
                if is_notification {
                    return String::new();
                }
                JsonRpcResponse::error(
                    request.id,
                    -32601,
                    format!("Method not found: {}", request.method),
                )
            }
        };

        serde_json::to_string(&response).unwrap_or_default()
    }

    pub fn handle_batch(&self, requests_json: &str) -> String {
        let requests: Vec<Value> = match serde_json::from_str(requests_json) {
            Ok(r) => r,
            Err(e) => {
                let resp = JsonRpcResponse::error(
                    None,
                    -32700,
                    format!("Parse error: {}", e),
                );
                return serde_json::to_string(&resp).unwrap_or_default();
            }
        };

        let responses: Vec<String> = requests
            .iter()
            .filter_map(|req| {
                let req_str = serde_json::to_string(req).unwrap_or_default();
                let result = self.handle_jsonrpc(&req_str);
                if result.is_empty() {
                    None
                } else {
                    Some(result)
                }
            })
            .collect();

        format!("[{}]", responses.join(","))
    }
}

impl Default for McpGateway {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_gateway_new() {
        let gw = McpGateway::new();
        assert!(gw.list_tools().is_empty());
    }

    #[test]
    fn test_register_tool() {
        let mut gw = McpGateway::new();
        gw.register_tool("ping", "Ping tool", Box::new(|_| Ok(json!("pong"))));
        assert_eq!(gw.list_tools().len(), 1);
    }

    #[test]
    fn test_call_tool_success() {
        let mut gw = McpGateway::new();
        gw.register_tool(
            "echo",
            "Echo input back",
            Box::new(|params| Ok(params.unwrap_or(json!(null)))),
        );
        let result = gw.call_tool("echo", Some(json!({"msg": "hello"})));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), json!({"msg": "hello"}));
    }

    #[test]
    fn test_call_tool_not_found() {
        let gw = McpGateway::new();
        let result = gw.call_tool("nonexistent", None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Tool not found"));
    }

    #[test]
    fn test_jsonrpc_parse_request() {
        let json_str = r#"{"jsonrpc":"2.0","id":1,"method":"echo","params":{"msg":"hi"}}"#;
        let req: JsonRpcRequest = serde_json::from_str(json_str).unwrap();
        assert_eq!(req.jsonrpc, "2.0");
        assert_eq!(req.method, "echo");
        assert_eq!(req.id, Some(json!(1)));
        assert_eq!(req.params, Some(json!({"msg": "hi"})));
    }

    #[test]
    fn test_jsonrpc_response_success() {
        let mut gw = McpGateway::new();
        gw.register_tool(
            "echo",
            "Echo input back",
            Box::new(|params| Ok(params.unwrap_or(json!(null)))),
        );
        let response_str = gw.handle_jsonrpc(
            r#"{"jsonrpc":"2.0","id":1,"method":"echo","params":{"msg":"hi"}}"#,
        );
        let response: JsonRpcResponse = serde_json::from_str(&response_str).unwrap();
        assert_eq!(response.jsonrpc, "2.0");
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[test]
    fn test_jsonrpc_response_error_unknown_method() {
        let gw = McpGateway::new();
        let response_str = gw.handle_jsonrpc(
            r#"{"jsonrpc":"2.0","id":2,"method":"unknown","params":null}"#,
        );
        let response: JsonRpcResponse = serde_json::from_str(&response_str).unwrap();
        assert!(response.error.is_some());
        assert!(response.result.is_none());
    }

    #[test]
    fn test_jsonrpc_notification() {
        let gw = McpGateway::new();
        // Notification has no id field
        let response_str = gw.handle_jsonrpc(
            r#"{"jsonrpc":"2.0","method":"some_notification","params":null}"#,
        );
        // Should return empty string for notifications
        assert!(response_str.is_empty());
    }

    #[test]
    fn test_tool_registry_multiple() {
        let mut gw = McpGateway::new();
        gw.register_tool("tool1", "Tool 1", Box::new(|_| Ok(json!(1))));
        gw.register_tool("tool2", "Tool 2", Box::new(|_| Ok(json!(2))));
        gw.register_tool("tool3", "Tool 3", Box::new(|_| Ok(json!(3))));
        assert_eq!(gw.list_tools().len(), 3);
    }

    #[test]
    fn test_jsonrpc_error_code() {
        let gw = McpGateway::new();
        let response_str = gw.handle_jsonrpc(
            r#"{"jsonrpc":"2.0","id":3,"method":"nonexistent"}"#,
        );
        let response: JsonRpcResponse = serde_json::from_str(&response_str).unwrap();
        let err = response.error.unwrap();
        assert_eq!(err.code, -32601);
    }

    #[test]
    fn test_tool_with_params() {
        let mut gw = McpGateway::new();
        gw.register_tool(
            "add",
            "Add two numbers",
            Box::new(|params| {
                let p = params.ok_or("No params")?;
                let a = p["a"].as_f64().ok_or("Missing a")?;
                let b = p["b"].as_f64().ok_or("Missing b")?;
                Ok(json!(a + b))
            }),
        );
        let response_str = gw.handle_jsonrpc(
            r#"{"jsonrpc":"2.0","id":4,"method":"add","params":{"a":3,"b":4}}"#,
        );
        let response: JsonRpcResponse = serde_json::from_str(&response_str).unwrap();
        assert_eq!(response.result.unwrap(), json!(7.0));
    }

    #[test]
    fn test_gateway_list_tools_names() {
        let mut gw = McpGateway::new();
        gw.register_tool("alpha", "Alpha", Box::new(|_| Ok(json!(null))));
        gw.register_tool("beta", "Beta", Box::new(|_| Ok(json!(null))));
        let tools = gw.list_tools();
        let mut names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        names.sort();
        assert_eq!(names, vec!["alpha", "beta"]);
    }
}
