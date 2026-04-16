//! End-to-end integration test for the SigOps Agent heartbeat / command loop.
//!
//! Spawns a [`wiremock`] mock server on a random port, configures the agent's
//! real `HeartbeatClient` to point at it, and drives one full cycle:
//!   1. Agent POSTs `/api/v1/agents/heartbeat` → server responds 200 (+ rotated
//!      token).
//!   2. Agent GETs `/api/v1/agents/{id}/command/next` → server returns a test
//!      command.
//!   3. Agent executes the command locally via `execute_task`.
//!   4. Agent POSTs `/api/v1/agents/command/{id}/result` with the output.
//!
//! All four interactions are asserted: the heartbeat payload is well-formed,
//! the command is delivered, execution reports `SUCCESS`, and the result
//! payload reaches the server with the correct `taskId` and stdout.

use serde_json::{json, Value};
use sigops_agent::discovery::{collect_host_info, discover_tools};
use sigops_agent::executor::{execute_task, TaskStatus};
use sigops_agent::heartbeat::HeartbeatClient;
use std::sync::{Arc, Mutex};
use wiremock::matchers::{bearer_token, header, method, path, path_regex};
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

/// Shared captures from the mock server, asserted after the agent cycle runs.
#[derive(Default, Clone)]
struct Captured {
    heartbeat_body: Arc<Mutex<Option<Value>>>,
    result_body: Arc<Mutex<Option<Value>>>,
    result_path: Arc<Mutex<Option<String>>>,
}

/// Custom responder for `/heartbeat` that records the payload and returns
/// the standard rotated-token response.
struct HeartbeatResponder {
    capture: Arc<Mutex<Option<Value>>>,
}

impl Respond for HeartbeatResponder {
    fn respond(&self, req: &Request) -> ResponseTemplate {
        if let Ok(body) = serde_json::from_slice::<Value>(&req.body) {
            *self.capture.lock().unwrap() = Some(body);
        }
        ResponseTemplate::new(200).set_body_json(json!({
            "agentId": "test-agent-001",
            "status": "ONLINE",
            "rotatedToken": "rotated-integration-token-xyz"
        }))
    }
}

/// Custom responder for `/command/{id}/result` that records path + payload.
struct ResultResponder {
    body_capture: Arc<Mutex<Option<Value>>>,
    path_capture: Arc<Mutex<Option<String>>>,
}

impl Respond for ResultResponder {
    fn respond(&self, req: &Request) -> ResponseTemplate {
        *self.path_capture.lock().unwrap() = Some(req.url.path().to_string());
        if let Ok(body) = serde_json::from_slice::<Value>(&req.body) {
            *self.body_capture.lock().unwrap() = Some(body);
        }
        ResponseTemplate::new(200).set_body_json(json!({ "ok": true }))
    }
}

#[tokio::test]
async fn agent_drives_full_heartbeat_command_result_cycle() {
    // ---- 1. stand up mock server --------------------------------------
    let server = MockServer::start().await;
    let captured = Captured::default();

    Mock::given(method("POST"))
        .and(path("/api/v1/agents/heartbeat"))
        .and(bearer_token("integration-initial-token"))
        .and(header("content-type", "application/json"))
        .respond_with(HeartbeatResponder {
            capture: captured.heartbeat_body.clone(),
        })
        .expect(1)
        .mount(&server)
        .await;

    // Command-next returns a deterministic task the agent can actually execute.
    // `sigops.condition` produces `{"result": true}` output — our stand-in for
    // `echo "integration test"` that still exercises the full executor path.
    let command_id = "cmd-int-42";
    Mock::given(method("GET"))
        .and(path("/api/v1/agents/test-agent-001/command/next"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "taskId": command_id,
            "toolName": "sigops.condition",
            "input": { "expression": "1 == 1" }
        })))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path_regex(r"^/api/v1/agents/command/[^/]+/result$"))
        .respond_with(ResultResponder {
            body_capture: captured.result_body.clone(),
            path_capture: captured.result_path.clone(),
        })
        .expect(1)
        .mount(&server)
        .await;

    // ---- 2. wire up the agent client -----------------------------------
    let client = HeartbeatClient::new(&server.uri(), "integration-initial-token");
    let host_info = collect_host_info();
    let tools = discover_tools();

    // ---- 3. one heartbeat ---------------------------------------------
    let hb_resp = client
        .send_heartbeat("test-agent-001", "tenant-integration", &host_info, &tools)
        .await
        .expect("heartbeat must succeed");
    assert_eq!(hb_resp.agent_id, "test-agent-001");
    assert_eq!(hb_resp.status, "ONLINE");
    assert_eq!(
        hb_resp.rotated_token.as_deref(),
        Some("rotated-integration-token-xyz")
    );

    // ---- 4. fetch command ---------------------------------------------
    let cmd = client
        .fetch_next_command("test-agent-001")
        .await
        .expect("fetch must succeed")
        .expect("a command must be returned");
    assert_eq!(cmd.task_id, command_id);
    assert_eq!(cmd.tool_name, "sigops.condition");

    // ---- 5. execute locally -------------------------------------------
    let result = execute_task(&cmd);
    assert_eq!(result.status, TaskStatus::Success);
    assert_eq!(result.task_id, command_id);
    assert_eq!(result.output["result"], true);
    assert!(result.error.is_none());

    // ---- 6. report result back ----------------------------------------
    client
        .report_result(&result)
        .await
        .expect("report_result must succeed");

    // ---- 7. assert captures --------------------------------------------
    let hb = captured
        .heartbeat_body
        .lock()
        .unwrap()
        .clone()
        .expect("heartbeat payload captured");
    assert_eq!(hb["agentId"], "test-agent-001");
    assert_eq!(hb["tenantId"], "tenant-integration");
    assert!(hb["hostname"].is_string());
    assert!(hb["os"].is_string());
    assert!(hb["agentVersion"].is_string());

    let result_path = captured
        .result_path
        .lock()
        .unwrap()
        .clone()
        .expect("result path captured");
    assert_eq!(
        result_path,
        format!("/api/v1/agents/command/{}/result", command_id)
    );

    let reported = captured
        .result_body
        .lock()
        .unwrap()
        .clone()
        .expect("result body captured");
    assert_eq!(reported["taskId"], command_id);
    assert_eq!(reported["status"], "SUCCESS");
    assert_eq!(reported["output"]["result"], true);

    // MockServer's Drop verifies every `.expect(1)` was satisfied.
}
