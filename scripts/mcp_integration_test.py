#!/usr/bin/env python3
"""
MCP Server Integration Test
Tests the MCP server via stdio JSON-RPC protocol
"""

import json
import subprocess
import sys
import time
from typing import Dict, Any, Optional

class MCPClient:
    def __init__(self, command: list):
        self.process = subprocess.Popen(
            command,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            universal_newlines=True
        )
        self.request_id = 0
        self.capabilities = None
    
    def send_request(self, method: str, params: Dict[str, Any] = None) -> Dict[str, Any]:
        self.request_id += 1
        request = {
            "jsonrpc": "2.0",
            "id": self.request_id,
            "method": method,
            "params": params or {}
        }
        
        request_json = json.dumps(request)
        self.process.stdin.write(request_json + "\n")
        self.process.stdin.flush()
        
        response_line = self.process.stdout.readline()
        if not response_line:
            stderr = self.process.stderr.read()
            raise Exception(f"No response, stderr: {stderr}")
        
        return json.loads(response_line)
    
    def initialize(self) -> Dict[str, Any]:
        response = self.send_request("initialize", {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        })
        if "result" in response:
            self.capabilities = response["result"]["capabilities"]
        return response
    
    def send_notification(self, method: str, params: Dict[str, Any] = None):
        notification = {
            "jsonrpc": "2.0",
            "method": method,
            "params": params or {}
        }
        self.process.stdin.write(json.dumps(notification) + "\n")
        self.process.stdin.flush()
    
    def close(self):
        self.process.stdin.close()
        self.process.wait()

def test_initialize(client: MCPClient) -> bool:
    print("\n[TEST] Initialize handshake...")
    try:
        response = client.initialize()
        assert "result" in response, "No result in initialize response"
        assert "protocolVersion" in response["result"], "No protocolVersion"
        print(f"  ✓ Initialize successful: {response['result'].get('serverInfo', {})}")
        return True
    except Exception as e:
        print(f"  ✗ Initialize failed: {e}")
        return False

def test_list_tools(client: MCPClient) -> bool:
    print("\n[TEST] List tools...")
    try:
        response = client.send_request("tools/list")
        assert "result" in response, "No result"
        tools = response["result"].get("tools", [])
        print(f"  ✓ Found {len(tools)} tools:")
        for tool in tools:
            print(f"    - {tool['name']}: {tool.get('description', '')[:50]}...")
        return len(tools) > 0
    except Exception as e:
        print(f"  ✗ List tools failed: {e}")
        return False

def test_call_tool(client: MCPClient) -> bool:
    print("\n[TEST] Call tool (list_files)...")
    try:
        response = client.send_request("tools/call", {
            "name": "list_files",
            "arguments": {"path": "/tmp"}
        })
        assert "result" in response, "No result"
        content = response["result"].get("content", [])
        print(f"  ✓ Tool call successful, {len(content)} content blocks")
        return True
    except Exception as e:
        print(f"  ✗ Call tool failed: {e}")
        return False

def test_list_resources(client: MCPClient) -> bool:
    print("\n[TEST] List resources...")
    try:
        response = client.send_request("resources/list")
        assert "result" in response, "No result"
        resources = response["result"].get("resources", [])
        print(f"  ✓ Found {len(resources)} resources")
        return True
    except Exception as e:
        print(f"  ✗ List resources failed: {e}")
        return False

def test_list_prompts(client: MCPClient) -> bool:
    print("\n[TEST] List prompts...")
    try:
        response = client.send_request("prompts/list")
        assert "result" in response, "No result"
        prompts = response["result"].get("prompts", [])
        print(f"  ✓ Found {len(prompts)} prompts:")
        for prompt in prompts:
            print(f"    - {prompt['name']}")
        return True
    except Exception as e:
        print(f"  ✗ List prompts failed: {e}")
        return False

def test_shutdown(client: MCPClient) -> bool:
    print("\n[TEST] Shutdown...")
    try:
        response = client.send_request("shutdown")
        assert "result" in response, "No result"
        print("  ✓ Shutdown successful")
        return True
    except Exception as e:
        print(f"  ✗ Shutdown failed: {e}")
        return False

def run_tests():
    print("=" * 60)
    print("OpenClaw MCP Server Integration Test")
    print("=" * 60)
    
    server_path = "/root/.openclaw/workspace/openclaw-rs/target/release/mcp-server"
    
    results = []
    
    # Test 1: Initialize
    client = MCPClient([server_path])
    results.append(("Initialize", test_initialize(client)))
    
    # Test 2: List tools
    results.append(("List Tools", test_list_tools(client)))
    
    # Test 3: Call tool
    results.append(("Call Tool", test_call_tool(client)))
    
    # Test 4: List resources
    results.append(("List Resources", test_list_resources(client)))
    
    # Test 5: List prompts
    results.append(("List Prompts", test_list_prompts(client)))
    
    # Test 6: Shutdown
    results.append(("Shutdown", test_shutdown(client)))
    
    client.close()
    
    # Summary
    print("\n" + "=" * 60)
    print("TEST SUMMARY")
    print("=" * 60)
    passed = sum(1 for _, r in results if r)
    total = len(results)
    for name, result in results:
        status = "✓ PASS" if result else "✗ FAIL"
        print(f"  {status}: {name}")
    print(f"\nTotal: {passed}/{total} passed")
    
    return passed == total

if __name__ == "__main__":
    success = run_tests()
    sys.exit(0 if success else 1)
