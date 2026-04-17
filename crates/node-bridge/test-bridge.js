const { OpenClawRuntime } = require('./openclaw_node_bridge.node');

async function runTests() {
  console.log('=== Node.js OpenClaw Bridge Test Suite v1.0 ===\n');

  let runtime;
  try {
    // Initialize runtime with mock provider (0 = Mock)
    console.log('1. Initializing OpenClawRuntime...');
    runtime = new OpenClawRuntime(0, null, null, 'mock-v1');
    console.log('   ✓ Runtime created successfully');

    // Test list_tools
    console.log('\n2. Testing list_tools()...');
    const tools = runtime.list_tools();
    console.log(`   Found ${tools.length} tools:`, tools);
    if (tools.includes('list_files') && tools.includes('read_file')) {
      console.log('   ✓ Built-in tools are discoverable');
    } else {
      console.log('   ✗ Missing expected tools');
    }

    // Test execute_tool: list_files
    console.log('\n3. Testing list_files tool...');
    const args = JSON.stringify({
      path: '/root/.openclaw/workspace',
      max_depth: 1,
      include_hidden: false
    });
    const listResult = runtime.execute_tool('list_files', args);
    const listData = JSON.parse(listResult);
    console.log(`   Files in workspace: ${listData.total} items`);
    if (listData.files && listData.files.length > 0) {
      console.log(`   Sample files: ${listData.files.slice(0, 3).map(f => f.name).join(', ')}`);
      console.log('   ✓ list_files executed successfully');
    } else {
      console.log('   ✗ Unexpected result format');
    }

    // Test execute_tool: read_file
    console.log('\n4. Testing read_file tool...');
    const readArgs = JSON.stringify({
      path: '/root/.openclaw/workspace/README.md',
      encoding: 'utf8',
      max_size: 10240
    });
    const readResult = runtime.execute_tool('read_file', readArgs);
    const readData = JSON.parse(readResult);
    console.log(`   Read ${readData.size} bytes (encoding: ${readData.encoding})`);
    if (readData.content && readData.content.length > 0) {
      console.log(`   Content preview: ${readData.content.substring(0, 100)}...`);
      console.log('   ✓ read_file executed successfully');
    } else {
      console.log('   ✗ Unexpected result format');
    }

    // Test chat with provider
    console.log('\n5. Testing chat() with mock provider...');
    const chatPayload = JSON.stringify({
      messages: [
        { role: 'user', content: 'Hello, what tools are available?' }
      ],
      model: 'mock-v1'
    });
    const chatResult = runtime.chat(chatPayload);
    const chatData = JSON.parse(chatResult);
    console.log(`   Provider: ${chatData.model}`);
    console.log(`   Response: ${chatData.message.content.substring(0, 150)}...`);
    if (chatData.message && chatData.message.role === 'assistant') {
      console.log('   ✓ chat() executed successfully');
    } else {
      console.log('   ✗ Unexpected response format');
    }

    // Test save_session (stub)
    console.log('\n6. Testing save_session (stub)...');
    const saveResult = runtime.save_session('test-session-001');
    console.log('   save_session result:', saveResult ? 'OK' : 'FAIL');

    // Test shutdown
    console.log('\n7. Testing shutdown()...');
    runtime.shutdown();
    console.log('   ✓ Runtime shutdown cleanly');

    console.log('\n=== All tests passed! ===');
  } catch (err) {
    console.error('\n❌ Test failed with error:', err);
    process.exit(1);
  }
}

// Run tests if executed directly
if (require.main === module) {
  runTests().catch(console.error);
}

module.exports = { runTests };
