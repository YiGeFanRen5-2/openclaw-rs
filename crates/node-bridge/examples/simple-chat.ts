import { OpenClawRuntime, ProviderMode } from '@openclaw/node-bridge';

async function demo() {
  // Create a runtime with mock provider
  const runtime = new OpenClawRuntime(ProviderMode.Mock, null, null, null);

  // Optionally configure session persistence
  // runtime.withSessionStore('./sessions');

  // Simple chat
  const messages = [
    { role: 'user', content: 'Hello, what is 2+2?' }
  ];

  const response = await runtime.chat(messages);
  console.log('Response:', response);

  // Execute a tool plan
  const steps = [
    { tool: 'http_get', args: { url: 'https://api.example.com/data' } }
  ];
  const result = await runtime.execute_plan(steps);
  console.log('Plan result:', result);

  runtime.shutdown();
}

demo().catch(console.error);