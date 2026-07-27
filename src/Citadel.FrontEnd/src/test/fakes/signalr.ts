import { HubConnection, HubConnectionState } from '@microsoft/signalr';
import { vi } from 'vitest';

type ReconnectingHandler = (error?: Error) => void;
type ReconnectedHandler = (connectionId?: string) => void;
type CloseHandler = (error?: Error) => void;
type EventHandler = (...args: any[]) => void;

export class FakeHubConnection {
  state = HubConnectionState.Disconnected;

  readonly start = vi.fn(async () => {
    this.state = HubConnectionState.Connected;
  });

  readonly stop = vi.fn(async () => {
    this.state = HubConnectionState.Disconnected;
  });

  readonly send = vi.fn(async (_methodName: string, ..._args: unknown[]) => {});
  readonly invoke = vi.fn(async (_methodName: string, ..._args: unknown[]) => {});

  private readonly reconnectingHandlers: ReconnectingHandler[] = [];
  private readonly reconnectedHandlers: ReconnectedHandler[] = [];
  private readonly closeHandlers: CloseHandler[] = [];
  private readonly eventHandlers = new Map<string, Set<EventHandler>>();

  readonly onreconnecting = vi.fn((handler: ReconnectingHandler) => {
    this.reconnectingHandlers.push(handler);
  });

  readonly onreconnected = vi.fn((handler: ReconnectedHandler) => {
    this.reconnectedHandlers.push(handler);
  });

  readonly onclose = vi.fn((handler: CloseHandler) => {
    this.closeHandlers.push(handler);
  });

  readonly on = vi.fn((eventName: string, handler: EventHandler) => {
    const handlers = this.eventHandlers.get(eventName) ?? new Set<EventHandler>();
    handlers.add(handler);
    this.eventHandlers.set(eventName, handlers);
  });

  readonly off = vi.fn((eventName: string, handler?: EventHandler) => {
    if (!handler) {
      this.eventHandlers.delete(eventName);
      return;
    }

    const handlers = this.eventHandlers.get(eventName);
    handlers?.delete(handler);
    if (handlers?.size === 0) {
      this.eventHandlers.delete(eventName);
    }
  });

  asHubConnection() {
    return this as unknown as HubConnection;
  }

  reconnecting(error?: Error) {
    this.state = HubConnectionState.Reconnecting;
    this.reconnectingHandlers.forEach((handler) => handler(error));
  }

  reconnected(connectionId = 'test-connection') {
    this.state = HubConnectionState.Connected;
    this.reconnectedHandlers.forEach((handler) => handler(connectionId));
  }

  closed(error?: Error) {
    this.state = HubConnectionState.Disconnected;
    this.closeHandlers.forEach((handler) => handler(error));
  }

  emit(eventName: string, ...args: any[]) {
    this.eventHandlers.get(eventName)?.forEach((handler) => handler(...args));
  }

  listenerCount(eventName: string) {
    return this.eventHandlers.get(eventName)?.size ?? 0;
  }
}
