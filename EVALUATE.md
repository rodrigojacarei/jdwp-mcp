# Evaluate Implementation

## Overview

The `debug.evaluate` tool has been implemented to evaluate expressions in the context of a stack frame during Java debugging.

## Current Capabilities

The implementation currently supports:

- **Local variable evaluation**: Read values of local variables in the current stack frame
- **Type support**: All primitive types (int, long, double, float, boolean, byte, short, char) and String objects
- **String formatting**: Automatically retrieves and formats String object values
- **Frame selection**: Can evaluate in any frame of the call stack (default is frame 0)
- **Result truncation**: Configurable maximum result length to prevent overwhelming output

## Usage

```json
{
  "thread_id": "0x1",
  "frame_index": 0,
  "expression": "variableName",
  "max_result_length": 500
}
```

### Parameters

- `thread_id` (required): Thread ID in hex format (e.g., "0x1")
- `expression` (required): Variable name to evaluate
- `frame_index` (optional): Stack frame index, 0 = current frame (default: 0)
- `max_result_length` (optional): Maximum length for string results (default: 500)

### Example

```
debug.evaluate(thread_id="0x1", expression="x")
// Returns: "x = 42"

debug.evaluate(thread_id="0x1", expression="message")
// Returns: "message = \"Hello, World!\""
```

## Limitations

The current implementation has the following limitations:

1. **No method invocation**: Cannot call methods (e.g., `obj.toString()`)
2. **No field access**: Cannot access object fields (e.g., `obj.field`)
3. **No expressions**: Cannot evaluate complex expressions (e.g., `x + y`)
4. **Local variables only**: Only reads local variables in the current scope

## Future Enhancements

To support full expression evaluation, the following would need to be implemented:

1. **Method invocation** via JDWP `ObjectReference.InvokeMethod` command
2. **Field access** via JDWP `ObjectReference.GetValues` command
3. **Expression parsing** to break down complex expressions
4. **Type resolution** to handle casts and type conversions
5. **Array access** via JDWP `ArrayReference.GetValues` command

## Testing

A test program and script are provided:

```bash
cd .kiro/mcpserver/jdwp-mcp
./test_evaluate.sh
```

This will:
1. Start the JDWP MCP server
2. Launch EvaluateTest.java with debugging enabled
3. Provide instructions for testing the evaluate functionality

## Implementation Details

The evaluate handler:
1. Retrieves the stack frames for the specified thread
2. Gets the variable table for the method in the selected frame
3. Filters variables that are in scope at the current code location
4. Looks up the requested variable by name
5. Retrieves the variable value using `StackFrame.GetValues`
6. Formats the value (with special handling for String objects)

Location: `mcp-server/src/handlers.rs::handle_evaluate()`
