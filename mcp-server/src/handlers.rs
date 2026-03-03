// MCP request handlers
//
// Handles initialize, list tools, and debug tool execution

use crate::protocol::*;
use crate::session::SessionManager;
use crate::tools;
use serde_json::json;
use tracing::{debug, info, warn};

pub struct RequestHandler {
    session_manager: SessionManager,
}

impl RequestHandler {
    pub fn new() -> Self {
        Self {
            session_manager: SessionManager::new(),
        }
    }

    pub async fn handle_request(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let result = match request.method.as_str() {
            "initialize" => self.handle_initialize(request.params),
            "tools/list" => self.handle_list_tools(),
            "tools/call" => self.handle_call_tool(request.params).await,
            _ => Err(JsonRpcError {
                code: METHOD_NOT_FOUND,
                message: format!("Method not found: {}", request.method),
                data: None,
            }),
        };

        match result {
            Ok(value) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: Some(value),
                error: None,
            },
            Err(error) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: None,
                error: Some(error),
            },
        }
    }

    pub async fn handle_notification(&self, notification: JsonRpcNotification) {
        match notification.method.as_str() {
            "notifications/initialized" => {
                info!("Client initialized");
            }
            "notifications/cancelled" => {
                debug!("Request cancelled");
            }
            _ => {
                warn!("Unknown notification: {}", notification.method);
            }
        }
    }

    fn handle_initialize(&self, params: Option<serde_json::Value>) -> Result<serde_json::Value, JsonRpcError> {
        let _params: InitializeParams = serde_json::from_value(params.unwrap_or(json!({})))
            .map_err(|e| JsonRpcError {
                code: INVALID_PARAMS,
                message: format!("Invalid initialize params: {}", e),
                data: None,
            })?;

        let result = InitializeResult {
            protocol_version: "2024-11-05".to_string(),
            capabilities: ServerCapabilities {
                tools: ToolsCapability {},
            },
            server_info: ServerInfo {
                name: "jdwp-mcp".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            instructions: Some(
                "JDWP debugging server for Java applications. \
                Start by using debug.attach to connect to a JVM, \
                then use debug.set_breakpoint, debug.get_stack, etc."
                    .to_string(),
            ),
        };

        Ok(serde_json::to_value(result).unwrap())
    }

    fn handle_list_tools(&self) -> Result<serde_json::Value, JsonRpcError> {
        let result = ListToolsResult {
            tools: tools::get_tools(),
        };

        Ok(serde_json::to_value(result).unwrap())
    }

    async fn handle_call_tool(&self, params: Option<serde_json::Value>) -> Result<serde_json::Value, JsonRpcError> {
        let call_params: CallToolParams = serde_json::from_value(params.unwrap_or(json!({})))
            .map_err(|e| JsonRpcError {
                code: INVALID_PARAMS,
                message: format!("Invalid tool call params: {}", e),
                data: None,
            })?;

        // Route to appropriate handler based on tool name
        let result = match call_params.name.as_str() {
            "debug.attach" => self.handle_attach(call_params.arguments).await,
            "debug.set_breakpoint" => self.handle_set_breakpoint(call_params.arguments).await,
            "debug.list_breakpoints" => self.handle_list_breakpoints(call_params.arguments).await,
            "debug.clear_breakpoint" => self.handle_clear_breakpoint(call_params.arguments).await,
            "debug.continue" => self.handle_continue(call_params.arguments).await,
            "debug.step_over" => self.handle_step_over(call_params.arguments).await,
            "debug.step_into" => self.handle_step_into(call_params.arguments).await,
            "debug.step_out" => self.handle_step_out(call_params.arguments).await,
            "debug.get_stack" => self.handle_get_stack(call_params.arguments).await,
            "debug.evaluate" => self.handle_evaluate(call_params.arguments).await,
            "debug.list_threads" => self.handle_list_threads(call_params.arguments).await,
            "debug.pause" => self.handle_pause(call_params.arguments).await,
            "debug.disconnect" => self.handle_disconnect(call_params.arguments).await,
            "debug.get_last_event" => self.handle_get_last_event(call_params.arguments).await,
            _ => Err(format!("Unknown tool: {}", call_params.name)),
        };

        match result {
            Ok(content) => {
                let call_result = CallToolResult {
                    content: vec![ContentBlock::Text { text: content }],
                    is_error: None,
                };
                Ok(serde_json::to_value(call_result).unwrap())
            }
            Err(error) => {
                let call_result = CallToolResult {
                    content: vec![ContentBlock::Text { text: error.clone() }],
                    is_error: Some(true),
                };
                Ok(serde_json::to_value(call_result).unwrap())
            }
        }
    }

    // Tool implementations (stubs for now)
    async fn handle_attach(&self, args: serde_json::Value) -> Result<String, String> {
        let host = args.get("host").and_then(|v| v.as_str()).unwrap_or("localhost");
        let port = args.get("port").and_then(|v| v.as_u64()).unwrap_or(5005) as u16;

        match jdwp_client::JdwpConnection::connect(host, port).await {
            Ok(connection) => {
                // Create session
                let session_id = self.session_manager.create_session(connection).await;

                // Get session guard once to prevent race between spawn and store
                let session_guard = self.session_manager.get_current_session().await
                    .ok_or_else(|| "Failed to get session after creation".to_string())?;

                // Clone connection, spawn task, and store handle in single critical section
                {
                    let mut session = session_guard.lock().await;
                    let connection_clone = session.connection.clone();

                    // Spawn event listener task
                    let session_manager = self.session_manager.clone();
                    let task_handle = tokio::spawn(async move {
                        loop {
                            // Receive event without holding any locks!
                            let event_opt = connection_clone.recv_event().await;

                            // Store event (brief lock acquisition)
                            if let Some(event_set) = event_opt {
                                if let Some(session_guard) = session_manager.get_current_session().await {
                                    let mut session = session_guard.lock().await;
                                    session.last_event = Some(event_set);
                                } else {
                                    break; // Session gone
                                }
                            } else {
                                break; // Connection closed
                            }
                        }
                        info!("Event listener task stopped");
                    });

                    // Store task handle before releasing lock - prevents race with disconnect
                    session.event_listener_task = Some(task_handle);
                }

                Ok(format!("Connected to JVM at {}:{} (session: {})", host, port, session_id))
            }
            Err(e) => Err(format!("Failed to connect: {}", e)),
        }
    }

    async fn handle_set_breakpoint(&self, args: serde_json::Value) -> Result<String, String> {
        let class_pattern = args.get("class_pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing 'class_pattern' parameter".to_string())?;

        let line = args.get("line")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| "Missing 'line' parameter".to_string())? as i32;

        let method_hint = args.get("method").and_then(|v| v.as_str());

        // Get current session
        let session_guard = self.session_manager.get_current_session().await
            .ok_or_else(|| "No active debug session. Use debug.attach first.".to_string())?;

        let mut session = session_guard.lock().await;

        // Convert class name to JVM signature format
        // e.g., "com.example.MyClass" -> "Lcom/example/MyClass;"
        let signature = if class_pattern.starts_with('L') && class_pattern.ends_with(';') {
            class_pattern.to_string()
        } else {
            format!("L{};", class_pattern.replace('.', "/"))
        };

        // Find the class
        let classes = session.connection.classes_by_signature(&signature).await
            .map_err(|e| format!("Failed to find class: {}", e))?;

        if classes.is_empty() {
            return Err(format!("Class not found: {}", class_pattern));
        }

        let class = &classes[0];

        // Get methods
        let methods = session.connection.get_methods(class.type_id).await
            .map_err(|e| format!("Failed to get methods: {}", e))?;

        // Find the right method (use hint if provided, otherwise find first method containing the line)
        let mut target_method = None;

        for method in &methods {
            if let Some(hint) = method_hint {
                if method.name == hint {
                    target_method = Some(method);
                    break;
                }
            }

            // Check if this method contains the line
            if let Ok(line_table) = session.connection.get_line_table(class.type_id, method.method_id).await {
                if line_table.lines.iter().any(|e| e.line_number == line) {
                    target_method = Some(method);
                    break;
                }
            }
        }

        let method = target_method.ok_or_else(|| {
            format!("No method found containing line {} in class {}", line, class_pattern)
        })?;

        // Get line table and find bytecode index for the line
        let line_table = session.connection.get_line_table(class.type_id, method.method_id).await
            .map_err(|e| format!("Failed to get line table: {}", e))?;

        let line_entry = line_table.lines.iter()
            .find(|e| e.line_number == line)
            .ok_or_else(|| format!("Line {} not found in method {}", line, method.name))?;

        // Set the breakpoint!
        let request_id = session.connection.set_breakpoint(
            class.type_id,
            method.method_id,
            line_entry.line_code_index,
            jdwp_client::SuspendPolicy::All,
        ).await.map_err(|e| format!("Failed to set breakpoint: {}", e))?;

        // Track the breakpoint in session
        let bp_id = format!("bp_{}", request_id);
        session.breakpoints.insert(bp_id.clone(), crate::session::BreakpointInfo {
            id: bp_id.clone(),
            request_id,
            class_pattern: class_pattern.to_string(),
            line: line as u32,
            method: Some(method.name.clone()),
            enabled: true,
            hit_count: 0,
        });

        Ok(format!(
            "✅ Breakpoint set at {}:{}\n   Method: {}\n   Breakpoint ID: {}\n   JDWP Request ID: {}",
            class_pattern, line, method.name, bp_id, request_id
        ))
    }

    async fn handle_list_breakpoints(&self, _args: serde_json::Value) -> Result<String, String> {
        let session_guard = self.session_manager.get_current_session().await
            .ok_or_else(|| "No active debug session".to_string())?;

        let session = session_guard.lock().await;

        if session.breakpoints.is_empty() {
            return Ok("No breakpoints set".to_string());
        }

        let mut output = format!("📍 {} breakpoint(s):\n\n", session.breakpoints.len());

        for (_, bp) in session.breakpoints.iter() {
            output.push_str(&format!(
                "  {} [{}] {}:{}\n",
                if bp.enabled { "✓" } else { "✗" },
                bp.id,
                bp.class_pattern,
                bp.line
            ));
            if let Some(method) = &bp.method {
                output.push_str(&format!("     Method: {}\n", method));
            }
            if bp.hit_count > 0 {
                output.push_str(&format!("     Hits: {}\n", bp.hit_count));
            }
        }

        Ok(output)
    }

    async fn handle_clear_breakpoint(&self, args: serde_json::Value) -> Result<String, String> {
        let bp_id = args.get("breakpoint_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing 'breakpoint_id' parameter".to_string())?;

        let session_guard = self.session_manager.get_current_session().await
            .ok_or_else(|| "No active debug session".to_string())?;

        let mut session = session_guard.lock().await;

        // Find the breakpoint
        let bp_info = session.breakpoints.get(bp_id)
            .ok_or_else(|| format!("Breakpoint not found: {}", bp_id))?
            .clone();

        // Clear the breakpoint in the JVM
        session.connection.clear_breakpoint(bp_info.request_id).await
            .map_err(|e| format!("Failed to clear breakpoint: {}", e))?;

        // Remove from session
        session.breakpoints.remove(bp_id);

        Ok(format!(
            "✅ Breakpoint cleared: {} at {}:{}\n   JDWP Request ID: {}",
            bp_id, bp_info.class_pattern, bp_info.line, bp_info.request_id
        ))
    }

    async fn handle_continue(&self, _args: serde_json::Value) -> Result<String, String> {
        let session_guard = self.session_manager.get_current_session().await
            .ok_or_else(|| "No active debug session".to_string())?;

        let mut session = session_guard.lock().await;

        session.connection.resume_all().await
            .map_err(|e| format!("Failed to resume: {}", e))?;

        Ok("▶️  Execution resumed".to_string())
    }

    async fn handle_step_over(&self, args: serde_json::Value) -> Result<String, String> {
        let session_guard = self.session_manager.get_current_session().await
            .ok_or_else(|| "No active debug session".to_string())?;

        let mut session = session_guard.lock().await;

        let thread_id = args.get("thread_id")
            .and_then(|v| v.as_str())
            .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            .ok_or_else(|| "Missing or invalid 'thread_id' parameter".to_string())?;

        // Set step request: LINE size, OVER depth
        let request_id = session.connection.set_step(
            thread_id,
            jdwp_client::commands::step_sizes::LINE,
            jdwp_client::commands::step_depths::OVER,
        ).await
            .map_err(|e| format!("Failed to set step: {}", e))?;

        // Resume the thread
        session.connection.resume_thread(thread_id).await
            .map_err(|e| format!("Failed to resume thread: {}", e))?;

        Ok(format!("⏭️  Stepping over (request ID: {})", request_id))
    }

    async fn handle_step_into(&self, args: serde_json::Value) -> Result<String, String> {
        let session_guard = self.session_manager.get_current_session().await
            .ok_or_else(|| "No active debug session".to_string())?;

        let mut session = session_guard.lock().await;

        let thread_id = args.get("thread_id")
            .and_then(|v| v.as_str())
            .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            .ok_or_else(|| "Missing or invalid 'thread_id' parameter".to_string())?;

        // Set step request: LINE size, INTO depth
        let request_id = session.connection.set_step(
            thread_id,
            jdwp_client::commands::step_sizes::LINE,
            jdwp_client::commands::step_depths::INTO,
        ).await
            .map_err(|e| format!("Failed to set step: {}", e))?;

        // Resume the thread
        session.connection.resume_thread(thread_id).await
            .map_err(|e| format!("Failed to resume thread: {}", e))?;

        Ok(format!("⤵️  Stepping into (request ID: {})", request_id))
    }

    async fn handle_step_out(&self, args: serde_json::Value) -> Result<String, String> {
        let session_guard = self.session_manager.get_current_session().await
            .ok_or_else(|| "No active debug session".to_string())?;

        let mut session = session_guard.lock().await;

        let thread_id = args.get("thread_id")
            .and_then(|v| v.as_str())
            .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            .ok_or_else(|| "Missing or invalid 'thread_id' parameter".to_string())?;

        // Set step request: LINE size, OUT depth
        let request_id = session.connection.set_step(
            thread_id,
            jdwp_client::commands::step_sizes::LINE,
            jdwp_client::commands::step_depths::OUT,
        ).await
            .map_err(|e| format!("Failed to set step: {}", e))?;

        // Resume the thread
        session.connection.resume_thread(thread_id).await
            .map_err(|e| format!("Failed to resume thread: {}", e))?;

        Ok(format!("⤴️  Stepping out (request ID: {})", request_id))
    }

    async fn handle_get_stack(&self, args: serde_json::Value) -> Result<String, String> {
        let session_guard = self.session_manager.get_current_session().await
            .ok_or_else(|| "No active debug session".to_string())?;

        let mut session = session_guard.lock().await;

        let thread_id = args.get("thread_id")
            .and_then(|v| v.as_str())
            .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok());

        let max_frames = args.get("max_frames")
            .and_then(|v| v.as_i64())
            .unwrap_or(20) as usize;

        let include_variables = args.get("include_variables")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        // If no thread specified, get all threads and use the first suspended one
        let target_thread = if let Some(tid) = thread_id {
            tid
        } else {
            let threads = session.connection.get_all_threads().await
                .map_err(|e| format!("Failed to get threads: {}", e))?;

            *threads.first().ok_or_else(|| "No threads found".to_string())?
        };

        // Get frames (-1 means all frames to avoid INVALID_LENGTH errors)
        let mut frames = session.connection.get_frames(target_thread, 0, -1).await
            .map_err(|e| format!("Failed to get frames: {}", e))?;

        // Truncate to max_frames
        frames.truncate(max_frames);

        if frames.is_empty() {
            return Ok(format!("Thread {:x} has no stack frames", target_thread));
        }

        let mut output = format!("🔍 Stack for thread {:x} ({} frames):\n\n", target_thread, frames.len());

        for (idx, frame) in frames.iter().enumerate() {
            output.push_str(&format!("Frame {}:\n", idx));
            output.push_str(&format!("  Location: class={:x}, method={:x}, index={}\n",
                frame.location.class_id, frame.location.method_id, frame.location.index));

            // Try to get method name
            if let Ok(methods) = session.connection.get_methods(frame.location.class_id).await {
                if let Some(method) = methods.iter().find(|m| m.method_id == frame.location.method_id) {
                    output.push_str(&format!("  Method: {}\n", method.name));

                    // Get variables if requested
                    if include_variables {
                        match session.connection.get_variable_table(frame.location.class_id, frame.location.method_id).await {
                            Ok(var_table) => {
                                let current_index = frame.location.index;
                                let active_vars: Vec<_> = var_table.iter()
                                    .filter(|v| current_index >= v.code_index && current_index < v.code_index + v.length as u64)
                                    .collect();

                                if !active_vars.is_empty() {
                                    output.push_str(&format!("  Variables ({}):\n", active_vars.len()));

                                    let slots: Vec<jdwp_client::stackframe::VariableSlot> = active_vars.iter()
                                        .map(|v| jdwp_client::stackframe::VariableSlot {
                                            slot: v.slot as i32,
                                            sig_byte: v.signature.as_bytes()[0],
                                        })
                                        .collect();

                                    if let Ok(values) = session.connection.get_frame_values(target_thread, frame.frame_id, slots).await {
                                        for (var, value) in active_vars.iter().zip(values.iter()) {
                                            // Check if this is a string object (tag 115 = 's')
                                            let formatted_value = if value.tag == 115 {
                                                // This is a String object
                                                if let jdwp_client::types::ValueData::Object(object_id) = &value.data {
                                                    if *object_id != 0 {
                                                        // Try to get the string value
                                                        match session.connection.get_string_value(*object_id).await {
                                                            Ok(string_val) => format!("(String) \"{}\"", string_val),
                                                            Err(_) => value.format(), // Fall back to object ID
                                                        }
                                                    } else {
                                                        "(String) null".to_string()
                                                    }
                                                } else {
                                                    value.format()
                                                }
                                            } else {
                                                value.format()
                                            };
                                            output.push_str(&format!("    {} = {}\n", var.name, formatted_value));
                                        }
                                    }
                                }
                            }
                            Err(_) => {}
                        }
                    }
                }
            }

            output.push_str("\n");
        }

        Ok(output)
    }

    async fn handle_evaluate(&self, args: serde_json::Value) -> Result<String, String> {
        let session_guard = self.session_manager.get_current_session().await
            .ok_or_else(|| "No active debug session".to_string())?;

        let mut session = session_guard.lock().await;

        let thread_id = args.get("thread_id")
            .and_then(|v| v.as_str())
            .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            .ok_or_else(|| "Missing or invalid 'thread_id' parameter".to_string())?;

        let frame_index = args.get("frame_index")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as usize;

        let expression = args.get("expression")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing 'expression' parameter".to_string())?;

        let max_result_length = args.get("max_result_length")
            .and_then(|v| v.as_i64())
            .unwrap_or(500) as usize;

        // Get frames
        let frames = session.connection.get_frames(thread_id, 0, -1).await
            .map_err(|e| format!("Failed to get frames: {}", e))?;

        if frame_index >= frames.len() {
            return Err(format!("Frame index {} out of range (0-{})", frame_index, frames.len() - 1));
        }

        let frame = &frames[frame_index];

        // Evaluate the expression
        match evaluate_expression(&mut session.connection, thread_id, frame.frame_id, &frame.location, expression).await {
            Ok(value) => {
                let result = format_value(&mut session.connection, &value, max_result_length).await;
                Ok(format!("{} = {}", expression, result))
            }
            Err(e) => Err(e)
        }
    }

    async fn handle_list_threads(&self, _args: serde_json::Value) -> Result<String, String> {
        let session_guard = self.session_manager.get_current_session().await
            .ok_or_else(|| "No active debug session".to_string())?;

        let mut session = session_guard.lock().await;

        let threads = session.connection.get_all_threads().await
            .map_err(|e| format!("Failed to get threads: {}", e))?;

        let mut output = format!("🧵 {} thread(s):\n\n", threads.len());

        for (idx, thread_id) in threads.iter().enumerate() {
            output.push_str(&format!("  Thread {} (ID: 0x{:x})\n", idx + 1, thread_id));

            // Try to get frame count
            match session.connection.get_frames(*thread_id, 0, 1).await {
                Ok(frames) if !frames.is_empty() => {
                    output.push_str("     Status: Has frames (possibly suspended)\n");
                }
                Ok(_) => {
                    output.push_str("     Status: Running (no frames)\n");
                }
                Err(_) => {
                    output.push_str("     Status: Cannot inspect\n");
                }
            }
        }

        Ok(output)
    }

    async fn handle_pause(&self, _args: serde_json::Value) -> Result<String, String> {
        let session_guard = self.session_manager.get_current_session().await
            .ok_or_else(|| "No active debug session".to_string())?;

        let mut session = session_guard.lock().await;

        session.connection.suspend_all().await
            .map_err(|e| format!("Failed to suspend: {}", e))?;

        Ok("⏸️  Execution paused (all threads suspended)".to_string())
    }

    async fn handle_disconnect(&self, _args: serde_json::Value) -> Result<String, String> {
        let current_session_id = self.session_manager.get_current_session_id().await;

        if let Some(session_id) = current_session_id {
            // Remove the session (this will also clear current session)
            self.session_manager.remove_session(&session_id).await;
            Ok(format!("✅ Disconnected from debug session: {}", session_id))
        } else {
            Err("No active debug session to disconnect".to_string())
        }
    }

    async fn handle_get_last_event(&self, _args: serde_json::Value) -> Result<String, String> {
        let session_guard = self.session_manager.get_current_session().await
            .ok_or_else(|| "No active debug session".to_string())?;

        let session = session_guard.lock().await;

        if let Some(event_set) = &session.last_event {
            let mut output = format!("🎯 Last event (suspend_policy={})\n\n", event_set.suspend_policy);

            for (idx, event) in event_set.events.iter().enumerate() {
                output.push_str(&format!("Event {}:\n", idx + 1));
                output.push_str(&format!("  Request ID: {}\n", event.request_id));

                match &event.details {
                    jdwp_client::events::EventKind::Breakpoint { thread, location } => {
                        output.push_str("  Type: Breakpoint\n");
                        output.push_str(&format!("  ⚡ Thread ID: 0x{:x}\n", thread));
                        output.push_str(&format!("  Location: class=0x{:x}, method=0x{:x}, index={}\n",
                            location.class_id, location.method_id, location.index));
                    }
                    jdwp_client::events::EventKind::Step { thread, location } => {
                        output.push_str("  Type: Step\n");
                        output.push_str(&format!("  Thread ID: 0x{:x}\n", thread));
                        output.push_str(&format!("  Location: class=0x{:x}, method=0x{:x}, index={}\n",
                            location.class_id, location.method_id, location.index));
                    }
                    jdwp_client::events::EventKind::VMStart { thread } => {
                        output.push_str("  Type: VM Start\n");
                        output.push_str(&format!("  Thread ID: 0x{:x}\n", thread));
                    }
                    jdwp_client::events::EventKind::VMDeath => {
                        output.push_str("  Type: VM Death\n");
                    }
                    jdwp_client::events::EventKind::ThreadStart { thread } => {
                        output.push_str("  Type: Thread Start\n");
                        output.push_str(&format!("  Thread ID: 0x{:x}\n", thread));
                    }
                    jdwp_client::events::EventKind::ThreadDeath { thread } => {
                        output.push_str("  Type: Thread Death\n");
                        output.push_str(&format!("  Thread ID: 0x{:x}\n", thread));
                    }
                    jdwp_client::events::EventKind::ClassPrepare { thread, ref_type, signature, .. } => {
                        output.push_str("  Type: Class Prepare\n");
                        output.push_str(&format!("  Thread ID: 0x{:x}\n", thread));
                        output.push_str(&format!("  Class: {} (0x{:x})\n", signature, ref_type));
                    }
                    _ => {
                        output.push_str("  Type: Other\n");
                    }
                }

                output.push_str("\n");
            }

            Ok(output)
        } else {
            Ok("No events received yet. Set a breakpoint and trigger it.".to_string())
        }
    }
}

async fn evaluate_expression(
    connection: &mut jdwp_client::JdwpConnection,
    thread_id: u64,
    frame_id: u64,
    location: &jdwp_client::types::Location,
    expression: &str,
) -> Result<jdwp_client::types::Value, String> {
    // Parse and evaluate expression recursively
    // Support: "var", "var[index]", "var.field", "var[index].field", etc.
    
    let expr = expression.trim();
    
    // Find the base variable name (everything before first [ or .)
    let base_end = expr.find(|c| c == '[' || c == '.').unwrap_or(expr.len());
    let base_var = &expr[..base_end];
    
    // Get the base variable value
    let mut current_value = get_variable(connection, thread_id, frame_id, location, base_var).await?;
    
    // Process the rest of the expression
    let mut pos = base_end;
    while pos < expr.len() {
        let ch = expr.chars().nth(pos).unwrap();
        
        if ch == '[' {
            // Array indexing
            let close_bracket = expr[pos..].find(']')
                .ok_or_else(|| "Missing closing bracket".to_string())?;
            let index_str = &expr[pos + 1..pos + close_bracket];
            let index: i32 = index_str.parse()
                .map_err(|_| format!("Invalid array index: {}", index_str))?;
            
            // Get array element
            if let jdwp_client::types::ValueData::Object(array_id) = current_value.data {
                if array_id == 0 {
                    return Err("Array is null".to_string());
                }
                
                let values = connection.get_array_values(array_id, index, 1).await
                    .map_err(|e| format!("Failed to get array element: {}", e))?;
                
                current_value = values.into_iter().next()
                    .ok_or_else(|| "Array index out of bounds".to_string())?;
            } else {
                return Err("Not an array".to_string());
            }
            
            pos += close_bracket + 1;
        } else if ch == '.' {
            // Field access
            pos += 1;
            let field_end = expr[pos..].find(|c| c == '[' || c == '.').unwrap_or(expr.len() - pos);
            let field_name = &expr[pos..pos + field_end];
            
            // Get field value
            if let jdwp_client::types::ValueData::Object(object_id) = current_value.data {
                if object_id == 0 {
                    return Err("Object is null".to_string());
                }
                
                let class_id = connection.get_object_reference_type(object_id).await
                    .map_err(|e| format!("Failed to get object type: {}", e))?;
                
                let fields = connection.get_fields(class_id).await
                    .map_err(|e| format!("Failed to get fields: {}", e))?;
                
                let field = fields.iter().find(|f| f.name == field_name)
                    .ok_or_else(|| format!("Field '{}' not found", field_name))?;
                
                let values = connection.get_object_values(object_id, vec![field.field_id]).await
                    .map_err(|e| format!("Failed to get field value: {}", e))?;
                
                current_value = values.into_iter().next()
                    .ok_or_else(|| "Failed to retrieve field value".to_string())?;
            } else {
                return Err("Not an object".to_string());
            }
            
            pos += field_end;
        } else {
            return Err(format!("Unexpected character: {}", ch));
        }
    }
    
    Ok(current_value)
}

async fn get_variable(
    connection: &mut jdwp_client::JdwpConnection,
    thread_id: u64,
    frame_id: u64,
    location: &jdwp_client::types::Location,
    var_name: &str,
) -> Result<jdwp_client::types::Value, String> {
    // Get variable table
    let var_table = connection.get_variable_table(location.class_id, location.method_id).await
        .map_err(|e| format!("Failed to get variable table: {}", e))?;
    
    // Find active variables at current location
    let active_vars: Vec<_> = var_table.iter()
        .filter(|v| location.index >= v.code_index && location.index < v.code_index + v.length as u64)
        .collect();
    
    // Look for variable by name
    let var = active_vars.iter().find(|v| v.name == var_name)
        .ok_or_else(|| format!("Variable '{}' not found in current scope", var_name))?;
    
    let slots = vec![jdwp_client::stackframe::VariableSlot {
        slot: var.slot as i32,
        sig_byte: var.signature.as_bytes()[0],
    }];
    
    let values = connection.get_frame_values(thread_id, frame_id, slots).await
        .map_err(|e| format!("Failed to get variable value: {}", e))?;
    
    values.into_iter().next()
        .ok_or_else(|| format!("Failed to retrieve variable '{}'", var_name))
}

async fn format_value(connection: &mut jdwp_client::JdwpConnection, value: &jdwp_client::types::Value, max_length: usize) -> String {
    format_value_impl(connection, value, max_length, 0).await
}

fn format_value_impl<'a>(
    connection: &'a mut jdwp_client::JdwpConnection,
    value: &'a jdwp_client::types::Value,
    max_length: usize,
    depth: usize,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = String> + 'a>> {
    Box::pin(async move {
        // Always expand Strings regardless of depth
        if value.tag == 115 {
            // String object
            if let jdwp_client::types::ValueData::Object(object_id) = &value.data {
                if *object_id != 0 {
                    match connection.get_string_value(*object_id).await {
                        Ok(string_val) => {
                            let truncated = if string_val.len() > max_length {
                                format!("{}...", &string_val[..max_length])
                            } else {
                                string_val
                            };
                            return format!("\"{}\"", truncated);
                        }
                        Err(_) => return value.format(),
                    }
                } else {
                    return "null".to_string();
                }
            }
        }
        
        // Check depth limit for non-String objects
        if depth > 2 {
            return value.format();
        }
        
        if value.tag == 91 {
            // Array object
            if let jdwp_client::types::ValueData::Object(array_id) = &value.data {
                if *array_id != 0 {
                    match connection.get_array_length(*array_id).await {
                        Ok(length) => {
                            if length == 0 {
                                return "[]".to_string();
                            }
                            // Get first few elements (max 10)
                            let fetch_count = length.min(10);
                            match connection.get_array_values(*array_id, 0, fetch_count).await {
                                Ok(values) => {
                                    let mut result = "[".to_string();
                                    for (i, val) in values.iter().enumerate() {
                                        if i > 0 {
                                            result.push_str(", ");
                                        }
                                        result.push_str(&format_value_impl(connection, val, 50, depth + 1).await);
                                    }
                                    if length > fetch_count {
                                        result.push_str(&format!(", ... ({} more)", length - fetch_count));
                                    }
                                    result.push(']');
                                    return result;
                                }
                                Err(_) => return format!("(array) @{:x} [length={}]", array_id, length),
                            }
                        }
                        Err(_) => return format!("(array) @{:x}", array_id),
                    }
                } else {
                    return "null".to_string();
                }
            }
        } else if value.tag == 76 {
            // Regular object (not string or array)
            if let jdwp_client::types::ValueData::Object(object_id) = &value.data {
                if *object_id != 0 {
                    // Get object's class
                    match connection.get_object_reference_type(*object_id).await {
                        Ok(class_id) => {
                            // Get class fields
                            match connection.get_fields(class_id).await {
                                Ok(fields) => {
                                    if fields.is_empty() {
                                        return format!("{{}}");
                                    }
                                    // Get field values (limit to first 5 fields)
                                    let field_ids: Vec<_> = fields.iter().take(5).map(|f| f.field_id).collect();
                                    match connection.get_object_values(*object_id, field_ids).await {
                                        Ok(values) => {
                                            let mut result = "{".to_string();
                                            for (i, (field, val)) in fields.iter().zip(values.iter()).enumerate() {
                                                if i > 0 {
                                                    result.push_str(", ");
                                                }
                                                result.push_str(&format!("{}: {}", field.name, format_value_impl(connection, val, 30, depth + 1).await));
                                            }
                                            if fields.len() > 5 {
                                                result.push_str(&format!(", ... ({} more fields)", fields.len() - 5));
                                            }
                                            result.push('}');
                                            return result;
                                        }
                                        Err(_) => return format!("(object) @{:x}", object_id),
                                    }
                                }
                                Err(_) => return format!("(object) @{:x}", object_id),
                            }
                        }
                        Err(_) => return format!("(object) @{:x}", object_id),
                    }
                } else {
                    return "null".to_string();
                }
            }
        }
        value.format()
    })
}
