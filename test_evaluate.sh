#!/bin/bash
# Test script for evaluate functionality

echo "Starting JDWP MCP server..."
cargo run --quiet &
SERVER_PID=$!
sleep 2

echo "Starting Java program with debug enabled..."
java -agentlib:jdwp=transport=dt_socket,server=y,suspend=y,address=5005 EvaluateTest &
JAVA_PID=$!

echo ""
echo "Test setup complete!"
echo "Server PID: $SERVER_PID"
echo "Java PID: $JAVA_PID"
echo ""
echo "To test evaluate:"
echo "1. Connect to the debugger using debug.attach"
echo "2. Set a breakpoint at EvaluateTest:11 (the 'Done' line)"
echo "3. Resume execution with debug.continue"
echo "4. When breakpoint hits, use debug.evaluate to inspect variables:"
echo "   - debug.evaluate with expression='x' should return 42"
echo "   - debug.evaluate with expression='message' should return 'Hello, World!'"
echo "   - debug.evaluate with expression='pi' should return 3.14159"
echo ""
echo "To cleanup: kill $SERVER_PID $JAVA_PID"
