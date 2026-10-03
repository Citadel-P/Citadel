import process from 'node:process';

const options = parseArguments(process.argv.slice(2));
const url = required(options, 'url');
const platformId = required(options, 'platform-id');
const mode = options.mode ?? 'verify';
const token = process.env.CITADEL_REALTIME_TEST_TOKEN;
if (!token) throw new Error('CITADEL_REALTIME_TEST_TOKEN is required.');
if (typeof WebSocket === 'undefined') throw new Error('This client requires Node.js with the built-in WebSocket API.');

async function verifyProtocol() {
  const first = await connectAndSubscribe(0);
  const initial = await receiveEnvelope(first);
  assertEnvelope(initial, { sequence: 1, eventKind: 'snapshot' });

  first.socket.send(JSON.stringify({
    protocolVersion: 1,
    kind: 'resync',
    lastSequence: initial.sequence,
    lastResourceRevision: initial.resourceRevision,
  }));
  const resync = await receiveUntil(first, initial, 'snapshot');
  first.socket.close(1000, 'verified');
  await waitForClose(first.socket);

  const second = await connectAndSubscribe(resync.resourceRevision);
  const reconnect = await receiveEnvelope(second);
  assertEnvelope(reconnect, { sequence: 1, eventKind: 'snapshot' });
  if (reconnect.connectionId === initial.connectionId) {
    throw new Error('Reconnect reused a connectionId.');
  }
  if (reconnect.resourceRevision < resync.resourceRevision) {
    throw new Error('Reconnect snapshot moved the resource revision backwards.');
  }
  second.socket.close(1000, 'verified');
  await waitForClose(second.socket);

  return {
    protocolVersion: reconnect.protocolVersion,
    initialConnectionId: initial.connectionId,
    reconnectConnectionId: reconnect.connectionId,
    initialRevision: initial.resourceRevision,
    reconnectRevision: reconnect.resourceRevision,
    reconnectSnapshot: true,
  };
}

async function runWorkload(durationSeconds) {
  if (!Number.isFinite(durationSeconds) || durationSeconds <= 0) {
    throw new Error('duration-seconds must be greater than zero.');
  }
  const deadline = Date.now() + durationSeconds * 1_000;
  let revision = 0;
  let reconnects = 0;
  let slowConnections = 0;
  while (Date.now() < deadline) {
    const normal = await connectAndSubscribe(revision);
    const snapshot = await receiveEnvelope(normal);
    assertEnvelope(snapshot, { sequence: 1, eventKind: 'snapshot' });
    revision = Math.max(revision, snapshot.resourceRevision);
    normal.socket.close(1000, 'reconnect-cycle');
    await waitForClose(normal.socket);
    reconnects += 1;

    const slow = await connectAndSubscribe(revision, 4);
    await delay(750);
    slow.socket.close(1000, 'slow-client-cycle');
    await waitForClose(slow.socket);
    slowConnections += 1;
  }
  return { reconnects, slowConnections, lastResourceRevision: revision };
}

async function connectAndSubscribe(lastResourceRevision, inboxCapacity = 64) {
  const socket = new WebSocket(url);
  const inbox = new BoundedInbox(socket, inboxCapacity);
  await new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error('WebSocket connection timed out.')), 5_000);
    socket.addEventListener('open', () => {
      clearTimeout(timer);
      resolve();
    }, { once: true });
    socket.addEventListener('error', () => {
      clearTimeout(timer);
      reject(new Error('WebSocket connection failed.'));
    }, { once: true });
  });
  socket.send(JSON.stringify({
    protocolVersion: 1,
    kind: 'subscribe',
    accessToken: token,
    resourceType: 'Platform',
    resourceId: platformId,
    lastResourceRevision,
  }));
  return { socket, inbox };
}

async function receiveUntil(client, previous, eventKind) {
  let current = previous;
  for (let index = 0; index < 100; index += 1) {
    const next = await receiveEnvelope(client);
    if (next.connectionId !== current.connectionId) throw new Error('connectionId changed without reconnect.');
    if (next.sequence !== current.sequence + 1) {
      client.socket.send(JSON.stringify({
        protocolVersion: 1,
        kind: 'resync',
        lastSequence: current.sequence,
        lastResourceRevision: current.resourceRevision,
      }));
      current = next;
      continue;
    }
    if (next.resourceRevision < current.resourceRevision) throw new Error('Resource revision moved backwards.');
    current = next;
    if (current.eventKind === eventKind) return current;
  }
  throw new Error(`Did not receive '${eventKind}' within the bounded message limit.`);
}

async function receiveEnvelope(client) {
  const text = await client.inbox.next(5_000);
  const envelope = JSON.parse(text);
  assertEnvelope(envelope, {});
  return envelope;
}

class BoundedInbox {
  #capacity;
  #closed;
  #messages = [];
  #waiter;

  constructor(socket, capacity) {
    this.#capacity = capacity;
    socket.addEventListener('message', event => this.#push(String(event.data)));
    socket.addEventListener('close', event => {
      this.#closed = new Error(`Realtime connection closed before a message (${event.code}).`);
      this.#flushWaiter();
    });
  }

  next(timeoutMilliseconds) {
    if (this.#messages.length > 0) return Promise.resolve(this.#messages.shift());
    if (this.#closed) return Promise.reject(this.#closed);
    if (this.#waiter) return Promise.reject(new Error('Only one pending realtime read is supported.'));
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this.#waiter = undefined;
        reject(new Error('Realtime message timed out.'));
      }, timeoutMilliseconds);
      this.#waiter = {
        resolve: message => {
          clearTimeout(timer);
          resolve(message);
        },
        reject: error => {
          clearTimeout(timer);
          reject(error);
        },
      };
    });
  }

  #push(message) {
    if (this.#waiter) {
      const waiter = this.#waiter;
      this.#waiter = undefined;
      waiter.resolve(message);
      return;
    }
    if (this.#messages.length === this.#capacity) this.#messages.shift();
    this.#messages.push(message);
  }

  #flushWaiter() {
    if (!this.#waiter) return;
    const waiter = this.#waiter;
    this.#waiter = undefined;
    waiter.reject(this.#closed);
  }
}

function assertEnvelope(envelope, expected) {
  const requiredFields = [
    'protocolVersion', 'connectionId', 'sequence', 'resourceType', 'resourceId',
    'resourceRevision', 'eventKind', 'payloadSchemaVersion', 'payload',
  ];
  for (const field of requiredFields) {
    if (!(field in envelope)) throw new Error(`Realtime envelope is missing '${field}'.`);
  }
  if (envelope.protocolVersion !== 1) throw new Error(`Unsupported protocol version ${envelope.protocolVersion}.`);
  if (envelope.resourceType !== 'Platform' || envelope.resourceId !== platformId) {
    throw new Error('Realtime envelope targets the wrong resource.');
  }
  if (expected.sequence !== undefined && envelope.sequence !== expected.sequence) {
    throw new Error(`Expected sequence ${expected.sequence}, received ${envelope.sequence}.`);
  }
  if (expected.eventKind !== undefined && envelope.eventKind !== expected.eventKind) {
    throw new Error(`Expected '${expected.eventKind}', received '${envelope.eventKind}'.`);
  }
}

async function waitForClose(socket) {
  if (socket.readyState === WebSocket.CLOSED) return;
  await new Promise(resolve => {
    const timer = setTimeout(resolve, 2_000);
    socket.addEventListener('close', () => {
      clearTimeout(timer);
      resolve();
    }, { once: true });
  });
}

function delay(milliseconds) {
  return new Promise(resolve => setTimeout(resolve, milliseconds));
}

function parseArguments(arguments_) {
  const parsed = {};
  for (let index = 0; index < arguments_.length; index += 2) {
    const key = arguments_[index];
    if (!key?.startsWith('--') || arguments_[index + 1] === undefined) {
      throw new Error(`Invalid argument '${key ?? ''}'.`);
    }
    parsed[key.slice(2)] = arguments_[index + 1];
  }
  return parsed;
}

function required(values, name) {
  const value = values[name];
  if (!value) throw new Error(`--${name} is required.`);
  return value;
}

const result = mode === 'verify'
  ? await verifyProtocol()
  : mode === 'workload'
    ? await runWorkload(Number(options['duration-seconds'] ?? 30))
    : (() => { throw new Error(`Unsupported mode '${mode}'.`); })();

process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
