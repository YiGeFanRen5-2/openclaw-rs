#!/usr/bin/env node
/**
 * OpenClaw Node.js Integration Example
 * 
 * Demonstrates how to use the OpenClaw Rust runtime from Node.js
 * via the native N-API bindings.
 */

const assert = require('assert');
const { ProviderMode, OpenClawRuntime } = require('../target/release/openclaw_node_bridge.node');

// ============================================================
// Example 1: Basic Runtime Setup
// ============================================================

console.log('\n=== Example 1: Basic Runtime Setup ===\n');

// Create runtime with Mock provider (for testing without API keys)
const rt = new OpenClawRuntime(ProviderMode.Mock, null, null, 'mock-v1');
console.log('✅ Runtime created with Mock provider');

// ============================================================
// Example 2: Tool Listing and Execution
// ============================================================

console.log('\n=== Example 2: Tool Listing ===\n');

// List available tools
const tools = rt.listTools();
console.log(`✅ Found ${tools.length} tools:`);
tools.forEach((tool, i) => {
  // Tools might be returned as array or object
  if (typeof tool === 'string') {
    console.log(`   - ${tool}`);
  } else {
    console.log(`   - [Tool ${i+1}]`);
  }
});
});

// ============================================================
// Example 3: File Operations (via MCP Server Tools)
// ============================================================

console.log('\n=== Example 3: File Operations ===\n');

// Read a file
try {
  const readResult = rt.executeTool('read_file', JSON.stringify({
    path: '/tmp/test.txt',
    encoding: 'utf8'
  }));
  const parsed = JSON.parse(readResult);
  console.log('✅ Read file result:', parsed);
} catch (e) {
  console.log('⚠️ Read file (expected to work with sandbox):', e.message);
}

// List files in a directory
try {
  const listResult = rt.executeTool('list_files', JSON.stringify({
    path: '/tmp',
    recursive: false
  }));
  const parsed = JSON.parse(listResult);
  console.log('✅ Listed /tmp directory');
} catch (e) {
  console.log('⚠️ List files (expected to work with sandbox):', e.message);
}

// ============================================================
// Example 4: Session Management
// ============================================================

console.log('\n=== Example 4: Session Management ===\n');

// Create a session
const sessionId = 'example-session-' + Date.now();
rt.createSession(sessionId);
console.log(`✅ Created session: ${sessionId}`);

// Add messages to the session
rt.addMessage(sessionId, 'user', 'Hello, OpenClaw!');
rt.addMessage(sessionId, 'assistant', 'Hello! How can I help you?');
console.log('✅ Added 2 messages to session');

// Get session info
const session = rt.getSession(sessionId);
const sessionData = JSON.parse(session);
console.log(`✅ Session has ${sessionData.messages.length} messages`);

// List all sessions
const sessions = rt.listSessions();
console.log(`✅ Found ${sessions.length} total sessions`);

// ============================================================
// Example 5: Chat with Mock Provider
// ============================================================

console.log('\n=== Example 5: Chat with Mock Provider ===\n');

const chatRequest = JSON.stringify({
  messages: [
    { role: 'user', content: 'What is 2+2?' }
  ],
  model: 'mock-model'
});

const chatResponse = rt.chat(chatRequest);
const chatData = JSON.parse(chatResponse);
console.log('✅ Chat response:', chatData.content || chatData);

// ============================================================
// Example 6: Runtime Status
// ============================================================

console.log('\n=== Example 6: Runtime Status ===\n');

const status = rt.runtimeStatus();
const statusData = JSON.parse(status);
console.log('✅ Runtime status:', statusData);

// ============================================================
// Example 7: Session Persistence
// ============================================================

console.log('\n=== Example 7: Session Persistence ===\n');

// Set session store directory
rt.setSessionStore('/tmp/openclaw-sessions');

// Persist the session
rt.persistSession(sessionId);
console.log('✅ Session persisted');

// Restore the session
rt.restoreSession(sessionId);
console.log('✅ Session restored');

// Clean up
rt.deleteSession(sessionId);
console.log('✅ Session deleted');

console.log('\n=== All Examples Completed Successfully! ===\n');
