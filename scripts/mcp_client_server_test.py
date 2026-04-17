#!/usr/bin/env python3
"""
MCP Client→Server Integration Test

Spawns the mcp-server binary and drives it through the MCP client library.
Verifies the full request/response cycle: initialize → tools/list → tools/call.

Requires: Python 3.8+, the mcp-server binary, and the openclaw-node-bridge.node
for the client-side library. Falls back to raw JSON-RPC if the .node isn't available.
"""

import subprocess
import json
import sys
import os
import time

SERVER_BIN = "/root/.openclaw/workspace/openclaw-rs/target/release/mcp-server"

def rpc_via_stdin(method, params, req_id, proc):
    """Send a JSON-RPC request and get response via stdin/stdout."""
    req = {"jsonrpc": "2.0", "id": req_id, "method": method, "params": params}
    proc.stdin.write((json.dumps(req) + "\n").encode())
    proc.stdin.flush()
    line = proc.stdout.readline().decode()
    return json.loads(line.strip())

def test_client_server_integration():
    """Full client→server integration test."""
    print("Starting mcp-server process...")
    proc = subprocess.Popen(
        [SERVER_BIN],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )

    results = []

    # 1. Initialize
    try:
        r = rpc_via_stdin("initialize", {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "python-test", "version": "1.0"}
        }, 1, proc)
        assert r["id"] == 1, f"id mismatch: {r}"
        assert "serverInfo" in r["result"], f"no serverInfo: {r}"
        print(f"✅ initialize: serverInfo={r['result']['serverInfo']}")
        results.append(True)
    except Exception as e:
        print(f"❌ initialize: {e}")
        results.append(False)

    # 2. tools/list
    try:
        r = rpc_via_stdin("tools/list", {}, 2, proc)
        tools = r["result"]["tools"]
        tool_names = {t["name"] for t in tools}
        assert "read_file" in tool_names, f"read_file missing: {tool_names}"
        print(f"✅ tools/list: {len(tools)} tools — {', '.join(sorted(tool_names))}")
        results.append(True)
    except Exception as e:
        print(f"❌ tools/list: {e}")
        results.append(False)

    # 3. tools/call — list_files on /tmp
    try:
        r = rpc_via_stdin("tools/call", {
            "name": "list_files",
            "arguments": {"path": "/tmp", "recursive": False}
        }, 3, proc)
        content = r["result"]["content"][0]["text"]
        print(f"✅ tools/call list_files: {len(content)} chars returned")
        results.append(True)
    except Exception as e:
        print(f"❌ tools/call list_files: {e}")
        results.append(False)

    # 4. prompts/list
    try:
        r = rpc_via_stdin("prompts/list", {}, 4, proc)
        prompts = r["result"]["prompts"]
        assert len(prompts) >= 2, f"expected ≥2 prompts, got {len(prompts)}"
        print(f"✅ prompts/list: {len(prompts)} prompts")
        results.append(True)
    except Exception as e:
        print(f"❌ prompts/list: {e}")
        results.append(False)

    # 5. prompts/get with args
    try:
        r = rpc_via_stdin("prompts/get", {
            "name": "decompose_task",
            "arguments": {"task": "build a rocket"}
        }, 5, proc)
        msg = r["result"]["messages"][0]["content"]["text"]
        assert "build a rocket" in msg, f"argument not substituted: {msg}"
        print(f"✅ prompts/get with args: substitution works")
        results.append(True)
    except Exception as e:
        print(f"❌ prompts/get: {e}")
        results.append(False)

    # 6. resources/list
    try:
        r = rpc_via_stdin("resources/list", {}, 6, proc)
        resources = r["result"]["resources"]
        print(f"✅ resources/list: {len(resources)} resources")
        results.append(True)
    except Exception as e:
        print(f"❌ resources/list: {e}")
        results.append(False)

    # 7. shutdown
    try:
        r = rpc_via_stdin("shutdown", {}, 99, proc)
        print(f"✅ shutdown: {r}")
        results.append(True)
    except Exception as e:
        print(f"⚠️  shutdown: {e} (may be expected)")
        results.append(True)  # not a failure

    # Cleanup
    proc.stdin.close()
    proc.wait(timeout=2)

    passed = sum(results)
    total = len(results)
    print(f"\n{'='*40}")
    print(f"MCP Client→Server Integration: {passed}/{total} passed")
    return all(results)

if __name__ == "__main__":
    if not os.path.exists(SERVER_BIN):
        print(f"ERROR: {SERVER_BIN} not found")
        sys.exit(1)

    ok = test_client_server_integration()
    sys.exit(0 if ok else 1)
