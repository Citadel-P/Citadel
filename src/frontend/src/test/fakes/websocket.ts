import type { Mock } from 'vitest';

export class FakeWebSocket {
  private readonly listeners = new Map<string, Set<EventListener>>();
  readonly send: Mock<(data: string) => void> = vi.fn();
  readonly close: Mock<WebSocket['close']> = vi.fn();
  readonly addEventListener = vi.fn((type: string, listener: EventListener) => {
    const listeners = this.listeners.get(type) ?? new Set<EventListener>();
    listeners.add(listener);
    this.listeners.set(type, listeners);
  });
  emit(type: string, event: Event) {
    this.listeners.get(type)?.forEach((listener) => listener.call(this, event));
  }
  message(envelope: object) {
    this.emit('message', new MessageEvent('message', { data: JSON.stringify(envelope) }));
  }
  asWebSocket(): WebSocket {
    return this as unknown as WebSocket;
  }
}
