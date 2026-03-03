# Stepping Commands Implementation

## Overview

Implemented step over, step into, and step out commands for Java debugging via JDWP.

## Commands

### debug.step_over
Steps over the current line (executes the line without entering method calls).

**Parameters:**
- `thread_id` (required): Thread ID in hex format (e.g., "0x1")

**Usage:**
```
debugstep_over(thread_id="0x1")
```

### debug.step_into
Steps into method calls on the current line.

**Parameters:**
- `thread_id` (required): Thread ID in hex format (e.g., "0x1")

**Usage:**
```
debugstep_into(thread_id="0x1")
```

### debug.step_out
Steps out of the current method (runs until the method returns).

**Parameters:**
- `thread_id` (required): Thread ID in hex format (e.g., "0x1")

**Usage:**
```
debugstep_out(thread_id="0x1")
```

## Implementation Details

Each step command:
1. Sets a JDWP single-step event request with appropriate depth (OVER/INTO/OUT)
2. Resumes the specified thread
3. The JVM will suspend again when the step completes
4. A step event will be generated and can be retrieved with `debugget_last_event()`

## Testing

Use `StepTest.java` to test stepping:

```bash
cd .kiro/mcpserver/jdwp-mcp
java -agentlib:jdwp=transport=dt_socket,server=y,suspend=y,address=5005 StepTest
```

Then:
1. Attach to debugger
2. Set breakpoint at line 7 (int sum = add(a, b);)
3. Continue execution
4. When breakpoint hits, use step commands:
   - `step_into` will enter the add() method
   - `step_over` will execute add() and move to next line
   - `step_out` will return from current method

## Files Modified

- `jdwp-client/src/eventrequest.rs` - Added `set_step()` and `clear_step()` methods
- `jdwp-client/src/thread.rs` - Added `resume_thread()` and `suspend_thread()` methods
- `mcp-server/src/handlers.rs` - Implemented step handlers
