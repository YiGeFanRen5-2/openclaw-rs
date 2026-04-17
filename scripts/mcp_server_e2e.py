#!/usr/bin/env python3
"""MCP Server End-to-End Smoke Tests."""

import subprocess
import json
import sys

SERVER = "/root/.openclaw/workspace/openclaw-rs/target/release/mcp-server"

def rpc(method, params, req_id):
    req = {"jsonrpc": "2.0", "id": req_id, "method": method, "params": params}
    proc = subprocess.Popen(
        [SERVER],
        stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE
    )
    out, _ = proc.communicate(input=(json.dumps(req) + "\n").encode(), timeout=3)
    return json.loads(out.strip().decode())

def init():
    return rpc("initialize", {
        "protocolVersion": "2024-11-05",
        "capabilities": {},
        "clientInfo": {"name": "test", "version": "1.0"}
    }, 1)

def main():
    passed = 0
    failed = 0

    # 1. Initialize
    try:
        r = init()
        assert r["id"] == 1
        assert r["result"]["serverInfo"]["name"] == "OpenClaw"
        print("✅ test_initialize")
        passed += 1
    except Exception as e:
        print(f"❌ test_initialize: {e}")
        failed += 1

    # 2. tools/list
    try:
        r = rpc("tools/list", {}, 2)
        tools = {t["name"] for t in r["result"]["tools"]}
        assert "read_file" in tools, f"read_file missing, got {tools}"
        assert "list_files" in tools
        print(f"✅ test_tools_list ({len(tools)} tools: {', '.join(sorted(tools))})")
        passed += 1
    except Exception as e:
        print(f"❌ test_tools_list: {e}")
        failed += 1

    # 3. tools/call read_file
    try:
        r = rpc("tools/call", {
            "name": "read_file",
            "arguments": {"path": "/etc/hostname"}
        }, 3)
        text = r["result"]["content"][0]["text"].strip()
        assert len(text) > 0, "hostname should not be empty"
        print(f"✅ test_tools_call_read_file (hostname={text})")
        passed += 1
    except Exception as e:
        print(f"❌ test_tools_call_read_file: {e}")
        failed += 1

    # 4. resources/list
    try:
        r = rpc("resources/list", {}, 4)
        resources = r["result"]["resources"]
        assert len(resources) > 0
        print(f"✅ test_resources_list ({len(resources)} resources)")
        passed += 1
    except Exception as e:
        print(f"❌ test_resources/list: {e}")
        failed += 1

    # 5. prompts/list
    try:
        r = rpc("prompts/list", {}, 5)
        prompts = {p["name"] for p in r["result"]["prompts"]}
        assert "openclaw_assistant" in prompts
        print(f"✅ test_prompts_list ({len(prompts)} prompts)")
        passed += 1
    except Exception as e:
        print(f"❌ test_prompts_list: {e}")
        failed += 1

    # 6. prompts/get with arg substitution
    try:
        r = rpc("prompts/get", {
            "name": "openclaw_assistant",
            "arguments": {"role": "Rust Expert", "guidelines": "Be concise."}
        }, 6)
        content = r["result"]["messages"][0]["content"]["text"]
        assert "Rust Expert" in content
        print(f"✅ test_prompts_get_with_args")
        passed += 1
    except Exception as e:
        print(f"❌ test_prompts_get_with_args: {e}")
        failed += 1

    # 7. invalid method returns error
    try:
        r = rpc("nonexistent/method", {}, 7)
        assert r["error"]["code"] == -32601
        print("✅ test_invalid_method")
        passed += 1
    except Exception as e:
        print(f"❌ test_invalid_method: {e}")
        failed += 1

    print(f"\n{'='*40}")
    print(f"Results: {passed} passed, {failed} failed")
    return failed == 0

if __name__ == "__main__":
    ok = main()
    sys.exit(0 if ok else 1)
