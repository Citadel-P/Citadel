import fs from 'fs';
import path from 'path';
import { clientSignatures } from './client-signatures';

const OPENAPI_PATH = path.resolve('./src/api/schema/swagger.json');
const OUTPUT_PATH = path.resolve('./src/api/generated/resources.ts');

interface OpenAPIOperation {
  operationId: string;
  tags?: string[];
  parameters?: Array<{ name: string; in: string; required?: boolean }>;
  requestBody?: unknown;
}

type Endpoint = {
  key: string;
  namespace: string;
  path: string;
  method: string;
  tag: string;
  group: string;
  params: string[];
  pathParams: string[];
  requiredParams: string[];
  queryParams: string[];
  requiredQueryParams: string[];
  hasBody: boolean;
};

const doc = JSON.parse(fs.readFileSync(OPENAPI_PATH, 'utf-8'));
const endpoints: Endpoint[] = [];
const signatures = clientSignatures(fs.readFileSync('./src/api/generated/api.types.ts', 'utf-8'));

for (const pathStr in doc.paths) {
  const pathItem = doc.paths[pathStr];

  for (const method of Object.keys(pathItem)) {
    const op: OpenAPIOperation = pathItem[method];
    if (!op.operationId) continue;

    const signature = signatures.get(op.operationId);
    if (!signature) throw new Error(`Missing generated client method: ${op.operationId}`);
    const { namespace, params, requiredParams } = signature;
    const tag = op.tags?.[0] ?? 'Default';
    const pathParams = op.parameters?.filter((p) => p.in === 'path') ?? [];
    const qParams = op.parameters?.filter((p) => p.in === 'query') ?? [];
    const queryParams = qParams.map((p) => p.name);
    const requiredQueryParams = qParams.filter((p) => p.required).map((p) => p.name);
    const hasBody = params.includes('data');

    endpoints.push({
      key: op.operationId,
      namespace,
      path: pathStr,
      method: method.toUpperCase(),
      tag,
      group: toGroupName(tag),
      params,
      pathParams: pathParams.map((p) => p.name),
      requiredParams,
      queryParams,
      requiredQueryParams,
      hasBody,
    });
  }
}

const scriptEndpoints = endpoints.filter((endpoint) => isScriptEndpointAllowed(endpoint.path, endpoint.tag));
const scriptGroups = scriptEndpoints.reduce<Record<string, string[]>>((acc, endpoint) => {
  acc[endpoint.group] ??= [];
  acc[endpoint.group].push(endpoint.key);
  return acc;
}, {});

const header = `// AUTO-GENERATED FILE. DO NOT EDIT.\n\n`;
const body =
  'export const resources = {\n' +
  endpoints.map((endpoint) => `  ${endpoint.key}: ${tsEndpoint(endpoint)},`).join('\n') +
  '\n} as const;\n\n' +
  'export const scriptResourceKeys = [\n' +
  scriptEndpoints.map((endpoint) => `  "${endpoint.key}",`).join('\n') +
  '\n] as const;\n\n' +
  'export type ScriptResourceName = (typeof scriptResourceKeys)[number];\n\n' +
  'export const scriptResources = {\n' +
  scriptEndpoints.map((endpoint) => `  ${endpoint.key}: resources.${endpoint.key},`).join('\n') +
  '\n} as const;\n\n' +
  'export const scriptResourceGroups = {\n' +
  Object.entries(scriptGroups)
    .map(([group, keys]) => `  ${group}: ${JSON.stringify(keys)},`)
    .join('\n') +
  '\n} as const;\n\n' +
  'export type ResourceName = keyof typeof resources;\n' +
  'export type ScriptResourceGroupName = keyof typeof scriptResourceGroups;\n';

fs.writeFileSync(OUTPUT_PATH, header + body);

console.log(`Generated ${OUTPUT_PATH} with ${endpoints.length} endpoints.`);
console.log(`Generated ${scriptEndpoints.length} scripts endpoints.`);

function tsEndpoint(endpoint: Endpoint) {
  return `{ method: "${endpoint.method}", key: "${endpoint.key}", namespace: "${endpoint.namespace}", path: ${JSON.stringify(endpoint.path)}, tag: ${JSON.stringify(
    endpoint.tag,
  )}, group: ${JSON.stringify(endpoint.group)}, params: ${JSON.stringify(endpoint.params)}, pathParams: ${JSON.stringify(
    endpoint.pathParams,
  )}, requiredParams: ${JSON.stringify(endpoint.requiredParams)}, queryParams: ${JSON.stringify(
    endpoint.queryParams,
  )}, requiredQueryParams: ${JSON.stringify(endpoint.requiredQueryParams)}, hasBody: ${endpoint.hasBody} }`;
}

function toGroupName(tag: string) {
  if (!tag) return 'default';
  return tag[0].toLowerCase() + tag.slice(1);
}

function isScriptEndpointAllowed(pathStr: string, tag: string) {
  if (tag === 'Authentication' || tag === 'Scripts' || tag === 'WebhookListener') return false;

  const lower = pathStr.toLowerCase();
  return (
    !lower.startsWith('/api/v1/authentication') &&
    !lower.startsWith('/api/v1/installation') &&
    !lower.startsWith('/api/v1/scripts') &&
    !lower.startsWith('/api/v1/resourcebindings/secrets') &&
    !lower.startsWith('/api/v1/resourcebindings/secret-providers') &&
    !lower.includes('/terminal') &&
    !lower.includes('/exec')
  );
}
