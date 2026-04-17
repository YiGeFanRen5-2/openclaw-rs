export enum ProviderMode {
  Mock = 0,
  Openai = 1,
  Anthropic = 2,
  Gemini = 3,
}

export class OpenClawRuntime {
  private native: any;

  constructor(
    provider: ProviderMode,
    apiKey?: string | null,
    baseUrl?: string | null,
    model?: string | null
  ) {
    // Note: actual native binding loading will be implemented by nari-rs
    this.native = null; // Placeholder
  }

  withSessionStore(path: string): void {
    // to be implemented
  }

  chat(messages: Array<{role: string, content: string}>): Promise<any> {
    // to be implemented
    return Promise.reject(new Error('Not implemented yet'));
  }

  execute_plan(steps: Array<{tool: string, args: any}>): Promise<any> {
    // to be implemented
    return Promise.reject(new Error('Not implemented yet'));
  }

  save_session(sessionId: string): Promise<void> {
    // to be implemented
    return Promise.reject(new Error('Not implemented yet'));
  }

  shutdown(): void {
    // to be implemented
  }
}