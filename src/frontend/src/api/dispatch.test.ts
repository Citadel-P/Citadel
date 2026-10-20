import { Api } from './generated/api.types';
import { invokeResource, resourceQueryKey } from './dispatch';

function capturedClient() {
  const fetch = vi.fn<typeof globalThis.fetch>().mockResolvedValue(
    new Response('{}', {
      headers: { 'Content-Type': 'application/json' },
    }),
  );
  const client = new Api({
    baseUrl: 'https://core.test',
    customFetch: fetch,
    securityWorker: () => ({ headers: { Authorization: 'Bearer fixture' } }),
  });
  return { client, fetch };
}

describe('generated-client dispatch', () => {
  it.each([undefined, { scope: 'Stack', resourceId: 'stack-id' }])(
    'sends secret body and optional scope %j separately from options',
    async (query) => {
      const { client, fetch } = capturedClient();
      const signal = new AbortController().signal;
      const data = { name: 'fixture', value: 'secret-value' };
      await invokeResource(client, 'createInternalSecret', {
        data,
        query,
        params: { headers: { 'X-Fixture': 'header-value' }, signal, credentials: 'include' },
      });
      const [url, options] = fetch.mock.calls[0];
      const parsed = new URL(String(url));
      expect(parsed.pathname).toBe('/api/v1/resourceBindings/secrets');
      expect(Object.fromEntries(parsed.searchParams)).toEqual(query ?? {});
      expect(JSON.parse(String(options?.body))).toEqual(data);
      expect(options?.signal).toBe(signal);
      expect(options?.credentials).toBe('include');
      expect(new Headers(options?.headers).get('X-Fixture')).toBe('header-value');
      expect(new Headers(options?.headers).get('Authorization')).toBe('Bearer fixture');
    },
  );

  it('keeps a path argument distinct from the body and forwards the response', async () => {
    const { client, fetch } = capturedClient();
    const response = await invokeResource(client, 'updateRegistry', { id: 'registry-id', data: { name: 'Renamed' } });
    expect(fetch.mock.calls[0][0]).toBe('https://core.test/api/v1/registries/registry-id');
    expect(JSON.parse(String(fetch.mock.calls[0][1]?.body))).toEqual({ name: 'Renamed' });
    expect(response.data).toEqual({});
  });

  it('keeps transport options out of read identity', () => {
    expect(
      resourceQueryKey('getRegistry', {
        id: 'registry-id',
        params: { headers: { Authorization: 'private' }, signal: new AbortController().signal },
      }),
    ).toEqual(['getRegistry', { id: 'registry-id' }]);
  });
});

// Checked by tsc; do not execute requests during this compile-time assertion.
function argumentContracts(client: Api<unknown>) {
  // @ts-expect-error The body is required, even when the optional query is present.
  invokeResource(client, 'createInternalSecret', { query: { scope: 'Stack' } });
  // @ts-expect-error A raw body is not the named argument convention.
  invokeResource(client, 'createInternalSecret', { name: 'fixture', value: 'secret' });
  invokeResource(client, 'createInternalSecret', {
    data: { name: 'fixture', value: 'secret' },
    // @ts-expect-error Request options cannot masquerade as a query.
    query: { signal: new AbortController().signal },
  });
  // @ts-expect-error Path arguments retain their generated type.
  invokeResource(client, 'updateRegistry', { id: 123, data: {} });
}
void argumentContracts;
