import fs from 'fs';
import path from 'path';

const OPENAPI_PATH = path.resolve('./src/api/swagger.json');
const OUTPUT_PATH = path.resolve('./src/api/generated/resources.ts');

interface OpenAPIOperation {
  operationId: string;
  parameters?: Array<{ name: string; in: string; required?: boolean }>;
  requestBody?: any;
  method: string;
}

const doc = JSON.parse(fs.readFileSync(OPENAPI_PATH, 'utf-8'));

const endpoints: {
  key: string;
  method: string;
  params: string[];
  requiredParams: string[];
  queryParams: string[];
}[] = [];

for (const pathStr in doc.paths) {
  const pathItem = doc.paths[pathStr];

  for (const method of Object.keys(pathItem)) {
    const op: OpenAPIOperation = pathItem[method];
    if (!op.operationId) continue;

    const params: string[] = [];
    const requiredParams: string[] = [];
    let queryParams: string[] = [];

    // Path parameters
    const pathParams = op.parameters?.filter((p) => p.in === 'path') ?? [];
    for (const p of pathParams) {
      if (!params.includes(p.name)) params.push(p.name);
      if (p.required) requiredParams.push(p.name);
    }

    // Query parameters
    const qParams = op.parameters?.filter((p) => p.in === 'query') ?? [];
    queryParams = qParams.map((p) => p.name);
    if (qParams.length > 0) {
      params.push('query');
      requiredParams.push(...qParams.filter((p) => p.required).map((p) => p.name));
    }

    // Request body (POST/PATCH/PUT)
    if (op.requestBody && ['POST', 'PATCH', 'PUT'].includes(method.toUpperCase())) {
      params.push('data');
    }

    // Always include optional RequestParams
    if (!params.includes('params')) params.push('params');

    endpoints.push({
      key: op.operationId,
      method: method.toUpperCase(),
      params,
      requiredParams,
      queryParams,
    });
  }
}

// Emit resources.ts
const header = `// AUTO-GENERATED FILE. DO NOT EDIT.\n\n`;

const body =
  'export const resources = {\n' +
  endpoints
    .map(
      (e) =>
        `  ${e.key}: { method: "${e.method}", key: "${e.key}", params: ${JSON.stringify(
          e.params,
        )}, requiredParams: ${JSON.stringify(e.requiredParams)}, queryParams: ${JSON.stringify(e.queryParams)} },`,
    )
    .join('\n') +
  '\n} as const;\n\nexport type ResourceName = keyof typeof resources;\n';

fs.writeFileSync(OUTPUT_PATH, header + body);
console.log(`✅ Generated ${OUTPUT_PATH} with ${endpoints.length} endpoints.`);
