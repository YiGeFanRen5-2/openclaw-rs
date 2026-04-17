# Example Plugin: Weather Tool

This is an example plugin demonstrating the OpenClaw plugin manifest format.

## Files

- `plugin.json` - Plugin manifest
- `tools/` - Tool implementations

## Plugin Manifest

```json
{
  "id": "example-weather-plugin",
  "name": "Weather Plugin",
  "version": "1.0.0",
  "description": "Example plugin that provides weather information",
  "author": "Your Name",
  "hooks": [
    {
      "name": "before_tool_call",
      "description": "Log tool calls"
    }
  ],
  "tools": [
    {
      "name": "weather",
      "description": "Get current weather for a city",
      "input_schema": {
        "type": "object",
        "properties": {
          "city": {
            "type": "string",
            "description": "City name"
          },
          "units": {
            "type": "string",
            "enum": ["celsius", "fahrenheit"],
            "default": "celsius"
          }
        },
        "required": ["city"]
      }
    }
  ],
  "permissions": [
    {
      "type": "network",
      "destinations": ["api.openweathermap.org"],
      "protocols": ["https"]
    }
  ]
}
```

## Tool Implementation

Tools can be implemented in any language and communicate via JSON-RPC.

Example tool response:

```json
{
  "jsonrpc": "2.0",
  "result": {
    "city": "Beijing",
    "temperature": 22,
    "condition": "Sunny",
    "humidity": 45
  },
  "id": 1
}
```

## Testing

Test the plugin manifest:

```bash
# Validate JSON syntax
jq . plugin.json

# Check manifest structure
cat plugin.json | jq '.id, .name, .version'
```
