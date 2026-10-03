import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const docsRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const schemaPath = path.resolve(docsRoot, '..', 'schema', 'public-v1.json');
const schema = JSON.parse(await readFile(schemaPath, 'utf8'));
const errors = [];
const operations = [];
const operationIds = new Set();

if (!schema.openapi?.startsWith('3.1.')) errors.push(`expected OpenAPI 3.1, found ${schema.openapi}`);
if (schema.info?.title !== 'Citadel Public API') errors.push('missing public API title');
if (!schema.components?.securitySchemes?.Bearer) errors.push('missing Bearer security scheme');

for (const [route, pathItem] of Object.entries(schema.paths ?? {})) {
  for (const method of ['get', 'post', 'put', 'patch', 'delete', 'head', 'options']) {
    const operation = pathItem[method];
    if (!operation) continue;
    operations.push({ route, method, operation });
    if (!operation.operationId) errors.push(`${method.toUpperCase()} ${route}: missing operationId`);
    else if (operationIds.has(operation.operationId)) errors.push(`duplicate operationId ${operation.operationId}`);
    else operationIds.add(operation.operationId);
    if (!operation.summary) errors.push(`${method.toUpperCase()} ${route}: missing summary`);
    if (!Array.isArray(operation.tags) || operation.tags.length === 0) errors.push(`${method.toUpperCase()} ${route}: missing tag`);
    const anonymous = route === '/health' && method === 'get';
    if (!anonymous && !operation.security?.some((requirement) => 'Bearer' in requirement)) errors.push(`${method.toUpperCase()} ${route}: missing bearer security requirement`);
    if (!anonymous && (!operation.responses?.['401'] || !operation.responses?.['403'])) errors.push(`${method.toUpperCase()} ${route}: missing authorization responses`);
  }
}

for (const required of [
  '/api/v1/platforms',
  '/api/v1/deployments',
  '/api/v1/stacks',
  '/api/v1/swarmServices',
]) {
  if (!schema.paths?.[required]) errors.push(`missing representative public path ${required}`);
}

for (const forbidden of [
  '/api/v1/setup',
  '/api/v1/authentication',
  '/api/v1/profile',
  '/api/v1/users',
  '/api/v1/serviceAccounts',
  '/api/v1/teams',
  '/api/v1/roles',
  '/api/v1/lookup',
  '/api/v1/search',
  '/listener',
]) {
  if (Object.keys(schema.paths ?? {}).some((route) => route.startsWith(forbidden))) {
    errors.push(`internal path leaked into public schema: ${forbidden}`);
  }
}

if (operations.length === 0) errors.push('public schema has no operations');

if (errors.length > 0) {
  console.error(errors.join('\n'));
  process.exitCode = 1;
} else {
  console.log(`Public OpenAPI validation passed for ${operations.length} operations.`);
}
