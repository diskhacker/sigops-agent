#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::process::Command as SysCommand;
use std::time::Instant;
use tracing::{info, warn};

/// A command dispatched from the SigOps server for this agent to execute.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskCommand {
    pub task_id: String,
    pub tool_name: String,
    pub input: serde_json::Value,
}

/// Result of executing a task command.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskResult {
    pub task_id: String,
    pub status: TaskStatus,
    pub output: serde_json::Value,
    pub error: Option<String>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TaskStatus {
    Success,
    Failed,
}

/// Execute a task command locally.
pub fn execute_task(cmd: &TaskCommand) -> TaskResult {
    let start = Instant::now();

    let result = match cmd.tool_name.as_str() {
        "sigops.restart" => execute_restart(cmd),
        "sigops.http" => execute_http_sync(cmd),
        "sigops.notify_slack" => execute_notify_slack(cmd),
        "sigops.condition" => execute_condition(cmd),
        "sigops.wait" => execute_wait(cmd),
        _ => Err(format!("unknown tool: {}", cmd.tool_name)),
    };

    let duration_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok(output) => {
            info!(task_id = %cmd.task_id, tool = %cmd.tool_name, "task succeeded");
            TaskResult {
                task_id: cmd.task_id.clone(),
                status: TaskStatus::Success,
                output,
                error: None,
                duration_ms,
            }
        }
        Err(error) => {
            warn!(task_id = %cmd.task_id, tool = %cmd.tool_name, error = %error, "task failed");
            TaskResult {
                task_id: cmd.task_id.clone(),
                status: TaskStatus::Failed,
                output: serde_json::Value::Null,
                error: Some(error),
                duration_ms,
            }
        }
    }
}

fn execute_restart(cmd: &TaskCommand) -> Result<serde_json::Value, String> {
    let service = cmd.input["service"]
        .as_str()
        .ok_or("missing 'service' field")?;
    let method = cmd.input["method"].as_str().unwrap_or("systemctl");

    // Sanitize service name to prevent injection
    if !service
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        return Err("invalid service name".to_string());
    }

    let output = SysCommand::new(method)
        .args(["restart", service])
        .output()
        .map_err(|e| format!("failed to execute {}: {}", method, e))?;

    Ok(serde_json::json!({
        "ok": output.status.success(),
        "service": service,
        "method": method,
        "stdout": String::from_utf8_lossy(&output.stdout).to_string(),
        "stderr": String::from_utf8_lossy(&output.stderr).to_string(),
    }))
}

fn execute_http_sync(cmd: &TaskCommand) -> Result<serde_json::Value, String> {
    let url = cmd.input["url"].as_str().ok_or("missing 'url' field")?;
    let method = cmd.input["method"].as_str().unwrap_or("GET");

    // Use curl for synchronous HTTP in the executor context
    let mut args = vec!["-s", "-o", "/dev/null", "-w", "%{http_code}", "-X", method];
    args.push(url);

    let output = SysCommand::new("curl")
        .args(&args)
        .output()
        .map_err(|e| format!("curl failed: {}", e))?;

    let status_code = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<u16>()
        .unwrap_or(0);

    Ok(serde_json::json!({
        "status": status_code,
        "ok": (200..300).contains(&status_code),
    }))
}

fn execute_notify_slack(cmd: &TaskCommand) -> Result<serde_json::Value, String> {
    let channel = cmd.input["channel"]
        .as_str()
        .ok_or("missing 'channel' field")?;
    let message = cmd.input["message"]
        .as_str()
        .ok_or("missing 'message' field")?;

    let webhook_url = cmd.input["webhookUrl"]
        .as_str()
        .unwrap_or("https://hooks.slack.com/services/placeholder");

    let payload = serde_json::json!({
        "channel": channel,
        "text": message,
    });

    let payload_str =
        serde_json::to_string(&payload).map_err(|e| format!("json encode failed: {}", e))?;

    let output = SysCommand::new("curl")
        .args([
            "-s",
            "-o",
            "/dev/null",
            "-w",
            "%{http_code}",
            "-X",
            "POST",
            "-H",
            "Content-Type: application/json",
            "-d",
            &payload_str,
            webhook_url,
        ])
        .output()
        .map_err(|e| format!("curl failed: {}", e))?;

    let status_code = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<u16>()
        .unwrap_or(0);

    Ok(serde_json::json!({
        "ok": (200..300).contains(&status_code),
        "channel": channel,
        "status": status_code,
    }))
}

fn execute_condition(cmd: &TaskCommand) -> Result<serde_json::Value, String> {
    let expr = cmd.input["expression"]
        .as_str()
        .ok_or("missing 'expression' field")?;

    // Simple numeric comparison: "10 > 5", "3 == 3"
    let parts: Vec<&str> = expr.split_whitespace().collect();
    if parts.len() != 3 {
        return Err(format!("unsupported expression: {}", expr));
    }

    let lhs: f64 = parts[0]
        .parse()
        .map_err(|_| format!("invalid LHS: {}", parts[0]))?;
    let rhs: f64 = parts[2]
        .parse()
        .map_err(|_| format!("invalid RHS: {}", parts[2]))?;

    let result = match parts[1] {
        "==" => lhs == rhs,
        "!=" => lhs != rhs,
        ">" => lhs > rhs,
        "<" => lhs < rhs,
        ">=" => lhs >= rhs,
        "<=" => lhs <= rhs,
        op => return Err(format!("unknown operator: {}", op)),
    };

    Ok(serde_json::json!({ "result": result }))
}

fn execute_wait(cmd: &TaskCommand) -> Result<serde_json::Value, String> {
    let seconds = cmd.input["seconds"]
        .as_f64()
        .ok_or("missing 'seconds' field")?;
    if seconds > 3600.0 {
        return Err("wait time exceeds maximum (3600s)".to_string());
    }
    let ms = (seconds * 1000.0) as u64;
    std::thread::sleep(std::time::Duration::from_millis(ms));
    Ok(serde_json::json!({ "waitedMs": ms }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_condition_equals() {
        let cmd = TaskCommand {
            task_id: "t1".to_string(),
            tool_name: "sigops.condition".to_string(),
            input: serde_json::json!({"expression": "10 == 10"}),
        };
        let result = execute_task(&cmd);
        assert_eq!(result.status, TaskStatus::Success);
        assert_eq!(result.output["result"], true);
    }

    #[test]
    fn test_execute_condition_greater() {
        let cmd = TaskCommand {
            task_id: "t1".to_string(),
            tool_name: "sigops.condition".to_string(),
            input: serde_json::json!({"expression": "10 > 5"}),
        };
        let result = execute_task(&cmd);
        assert_eq!(result.status, TaskStatus::Success);
        assert_eq!(result.output["result"], true);
    }

    #[test]
    fn test_execute_condition_false() {
        let cmd = TaskCommand {
            task_id: "t1".to_string(),
            tool_name: "sigops.condition".to_string(),
            input: serde_json::json!({"expression": "3 > 5"}),
        };
        let result = execute_task(&cmd);
        assert_eq!(result.status, TaskStatus::Success);
        assert_eq!(result.output["result"], false);
    }

    #[test]
    fn test_execute_unknown_tool() {
        let cmd = TaskCommand {
            task_id: "t1".to_string(),
            tool_name: "sigops.nonexistent".to_string(),
            input: serde_json::json!({}),
        };
        let result = execute_task(&cmd);
        assert_eq!(result.status, TaskStatus::Failed);
        assert!(result.error.unwrap().contains("unknown tool"));
    }

    #[test]
    fn test_execute_wait_short() {
        let cmd = TaskCommand {
            task_id: "t1".to_string(),
            tool_name: "sigops.wait".to_string(),
            input: serde_json::json!({"seconds": 0.01}),
        };
        let result = execute_task(&cmd);
        assert_eq!(result.status, TaskStatus::Success);
        assert!(result.duration_ms < 1000);
    }

    #[test]
    fn test_execute_wait_too_long() {
        let cmd = TaskCommand {
            task_id: "t1".to_string(),
            tool_name: "sigops.wait".to_string(),
            input: serde_json::json!({"seconds": 9999}),
        };
        let result = execute_task(&cmd);
        assert_eq!(result.status, TaskStatus::Failed);
        assert!(result.error.unwrap().contains("maximum"));
    }

    #[test]
    fn test_execute_condition_bad_expression() {
        let cmd = TaskCommand {
            task_id: "t1".to_string(),
            tool_name: "sigops.condition".to_string(),
            input: serde_json::json!({"expression": "not valid"}),
        };
        let result = execute_task(&cmd);
        assert_eq!(result.status, TaskStatus::Failed);
    }

    #[test]
    fn test_task_result_serialization() {
        let result = TaskResult {
            task_id: "t1".to_string(),
            status: TaskStatus::Success,
            output: serde_json::json!({"result": true}),
            error: None,
            duration_ms: 42,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("taskId"));
        assert!(json.contains("SUCCESS"));
        assert!(json.contains("durationMs"));
    }

    #[test]
    fn test_restart_invalid_service_name() {
        let cmd = TaskCommand {
            task_id: "t1".to_string(),
            tool_name: "sigops.restart".to_string(),
            input: serde_json::json!({"service": "nginx; rm -rf /"}),
        };
        let result = execute_task(&cmd);
        assert_eq!(result.status, TaskStatus::Failed);
        assert!(result.error.unwrap().contains("invalid service name"));
    }

    #[test]
    fn test_notify_slack_missing_channel() {
        let cmd = TaskCommand {
            task_id: "t1".to_string(),
            tool_name: "sigops.notify_slack".to_string(),
            input: serde_json::json!({"message": "hello"}),
        };
        let result = execute_task(&cmd);
        assert_eq!(result.status, TaskStatus::Failed);
        assert!(result.error.unwrap().contains("channel"));
    }

    #[test]
    fn test_notify_slack_missing_message() {
        let cmd = TaskCommand {
            task_id: "t1".to_string(),
            tool_name: "sigops.notify_slack".to_string(),
            input: serde_json::json!({"channel": "#alerts"}),
        };
        let result = execute_task(&cmd);
        assert_eq!(result.status, TaskStatus::Failed);
        assert!(result.error.unwrap().contains("message"));
    }
}
