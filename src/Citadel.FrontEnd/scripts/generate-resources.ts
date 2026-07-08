import fs from 'fs';
import path from 'path';

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
  path: string;
  method: string;
  tag: string;
  group: string;
  params: string[];
  pathParams: string[];
  requiredParams: string[];
  queryParams: string[];
  hasBody: boolean;
};

const doc = JSON.parse(fs.readFileSync(OPENAPI_PATH, 'utf-8'));
const endpoints: Endpoint[] = [];

for (const pathStr in doc.paths) {
  const pathItem = doc.paths[pathStr];

  for (const method of Object.keys(pathItem)) {
    const op: OpenAPIOperation = pathItem[method];
    if (!op.operationId) continue;

    const params: string[] = [];
    const requiredParams: string[] = [];
    const tag = op.tags?.[0] ?? 'Default';

    const pathParams = op.parameters?.filter((p) => p.in === 'path') ?? [];
    for (const p of pathParams) {
      if (!params.includes(p.name)) params.push(p.name);
      if (p.required) requiredParams.push(p.name);
    }

    const qParams = op.parameters?.filter((p) => p.in === 'query') ?? [];
    const queryParams = qParams.map((p) => p.name);
    if (qParams.length > 0) {
      params.push('query');
      requiredParams.push(...qParams.filter((p) => p.required && p.name !== 'tagIds').map((p) => p.name));
    }

    const hasBody = Boolean(op.requestBody && ['POST', 'PATCH', 'PUT', 'DELETE'].includes(method.toUpperCase()));
    if (hasBody) params.push('data');

    if (!params.includes('params')) params.push('params');

    endpoints.push({
      key: op.operationId,
      path: pathStr,
      method: method.toUpperCase(),
      tag,
      group: toGroupName(tag),
      params,
      pathParams: pathParams.map((p) => p.name),
      requiredParams,
      queryParams,
      hasBody,
    });
  }
}

const automationEndpoints = endpoints.filter((endpoint) => isAutomationEndpointAllowed(endpoint.path, endpoint.tag));
const automationGroups = automationEndpoints.reduce<Record<string, string[]>>((acc, endpoint) => {
  acc[endpoint.group] ??= [];
  acc[endpoint.group].push(endpoint.key);
  return acc;
}, {});

const header = `// AUTO-GENERATED FILE. DO NOT EDIT.\n\n`;
const body =
  'export const resources = {\n' +
  endpoints.map((endpoint) => `  ${endpoint.key}: ${tsEndpoint(endpoint)},`).join('\n') +
  '\n} as const;\n\n' +
  'export const automationResourceKeys = [\n' +
  automationEndpoints.map((endpoint) => `  "${endpoint.key}",`).join('\n') +
  '\n] as const;\n\n' +
  'export type AutomationResourceName = (typeof automationResourceKeys)[number];\n\n' +
  'export const automationResources = {\n' +
  automationEndpoints.map((endpoint) => `  ${endpoint.key}: resources.${endpoint.key},`).join('\n') +
  '\n} as const;\n\n' +
  'export const automationResourceGroups = {\n' +
  Object.entries(automationGroups)
    .map(([group, keys]) => `  ${group}: ${JSON.stringify(keys)},`)
    .join('\n') +
  '\n} as const;\n\n' +
  'export type ResourceName = keyof typeof resources;\n' +
  'export type AutomationResourceGroupName = keyof typeof automationResourceGroups;\n';

fs.writeFileSync(OUTPUT_PATH, header + body);

console.log(`Generated ${OUTPUT_PATH} with ${endpoints.length} endpoints.`);
console.log(`Generated ${automationEndpoints.length} automation endpoints.`);

function tsEndpoint(endpoint: Endpoint) {
  return `{ method: "${endpoint.method}", key: "${endpoint.key}", path: ${JSON.stringify(endpoint.path)}, tag: ${JSON.stringify(
    endpoint.tag,
  )}, group: ${JSON.stringify(endpoint.group)}, params: ${JSON.stringify(endpoint.params)}, pathParams: ${JSON.stringify(
    endpoint.pathParams,
  )}, requiredParams: ${JSON.stringify(endpoint.requiredParams)}, queryParams: ${JSON.stringify(
    endpoint.queryParams,
  )}, hasBody: ${endpoint.hasBody} }`;
}

function toGroupName(tag: string) {
  if (!tag) return 'default';
  return tag[0].toLowerCase() + tag.slice(1);
}

function isAutomationEndpointAllowed(pathStr: string, tag: string) {
  if (tag === 'Authentication' || tag === 'AutomationActions' || tag === 'WebhookListener') return false;

  const lower = pathStr.toLowerCase();
  return (
    !lower.startsWith('/api/v1/authentication') &&
    !lower.startsWith('/api/v1/automation') &&
    !lower.startsWith('/api/v1/resourcebindings/secrets') &&
    !lower.startsWith('/api/v1/resourcebindings/secret-providers') &&
    !lower.includes('/terminal') &&
    !lower.includes('/exec')
  );
}
