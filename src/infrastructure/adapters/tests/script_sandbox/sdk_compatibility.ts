// Runs in the real sandbox with a local fetch stub; no external API or credentials.
import { createCitadelClient } from "citadel:sdk";
const requests: Array<{ url: string; method: string; body?: string; token: string }> = [];
let responseStatus = 200;
let responseBody = '{"ok":true}';
globalThis.fetch = async (url, options) => {
  await new Promise(resolve => setTimeout(resolve, 5));
  requests.push({url: String(url), method: options!.method!, body: options!.body as string,
    token: new Headers(options!.headers).get('authorization')!});
  return new Response(responseBody, {status: responseStatus});
};
function equal(actual: unknown, expected: unknown) {
  if (JSON.stringify(actual) !== JSON.stringify(expected)) throw Error('SDK result differs: ' + JSON.stringify(actual));
}
async function fails(operation: () => unknown, expected: string) {
  try { await operation(); } catch (error) { equal((error as Error).message, expected); return; }
  throw Error('Expected API failure');
}
const citadel = createCitadelClient({baseUrl: 'https://core.invalid///', token: context.token});
equal(await citadel.platforms.listPlatforms({name: 'a b', tag: ['x/y', null, 'z'], empty: null, skip: undefined, enabled: false, offset: 0}), {ok: true});
equal(requests.pop(), {url: 'https://core.invalid/api/v1/platforms?name=a+b&tag=x%2Fy&tag=z&enabled=false&offset=0', method: 'GET', token: 'Bearer ' + context.token});
await citadel.scripts.updateScript('id /?#', {code: 'await Promise.resolve();', args: context.args});
equal(requests.pop(), {url: 'https://core.invalid/api/v1/scripts/id%20%2F%3F%23', method: 'PATCH', body: JSON.stringify({code: 'await Promise.resolve();', args: context.args}), token: 'Bearer ' + context.token});
if (citadel.api.listPlatforms !== citadel.platforms.listPlatforms) throw Error('Group and flat API differ');
await fails(() => citadel.api.updateScript(null, {}), "Citadel API updateScript requires path parameter 'id'.");
await fails(() => citadel.get('relative'), "Citadel API path must start with '/'.");
for (const method of ['get', 'post', 'put', 'patch', 'delete'] as const) {
  await citadel[method]('/generic', {value: context.args});
  equal(requests.pop(), {url: 'https://core.invalid/generic', method: method.toUpperCase(),
    ...(method === 'get' ? {} : {body: JSON.stringify({value: context.args})}), token: 'Bearer ' + context.token});
}
responseStatus = 409; responseBody = 'conflict';
await fails(() => citadel.post('/conflict', {}), 'Citadel API POST /conflict failed: 409 conflict');
requests.length = 0;
responseStatus = 200; responseBody = '';
equal(await citadel.request('HEAD', '/empty'), null);
requests.length = 0;
responseBody = '{"ok":true}';
const other = createCitadelClient({baseUrl: 'https://other.invalid', token: 'other-client'});
await Promise.all([citadel.get('/first'), other.get('/second'), citadel.get('/last')]);
equal(requests.map(r => [r.url, r.token]), [
  ['https://core.invalid/first', 'Bearer ' + context.token],
  ['https://other.invalid/second', 'Bearer other-client'],
  ['https://core.invalid/last', 'Bearer ' + context.token],
]);
if (!Object.isFrozen(citadel) || !Object.isFrozen(citadel.api)) throw Error('Client freeze changed');
console.log('sdk-compatible', context.args.name);
