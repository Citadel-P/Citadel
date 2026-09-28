import fs from 'node:fs';

// The frontend still consumes the accepted compatibility contract. Migrate the
// appearance DTOs from Rust without renaming unrelated legacy resource types.
// Run `cargo run --locked -p xtask -- openapi` before regenerating this client.
const contract = JSON.parse(fs.readFileSync('../schema/Citadel.WebApi.json', 'utf8'));
// The compatibility contract uses 3.1-compatible schemas; the installed parser
// does not yet accept its newer 3.2 document header.
contract.openapi = '3.1.1';
const rust = JSON.parse(fs.readFileSync('../../schema/v1.json', 'utf8'));
const aliases: Record<string, string> = { PatchUserPreferencesRequest: 'PatchUserPreferencesInput' };
const copied = new Set<string>();
function copySchema(name: string) {
  if (copied.has(name)) return;
  copied.add(name);
  const schema = rust.components.schemas[name];
  if (!schema) throw new Error(`Missing Rust appearance schema: ${name}`);
  const json = JSON.stringify(schema).replace(/#\/components\/schemas\/([^"\s]+)/g, (_, dependency: string) => {
    copySchema(dependency);
    return `#/components/schemas/${aliases[dependency] ?? dependency}`;
  });
  contract.components.schemas[aliases[name] ?? name] = JSON.parse(json);
}
// Preserve the accepted client nullability for computed optional descriptions.
// These values may be null in Rust responses even when newer .NET schemas say string.
for (const schema of Object.values(contract.components.schemas) as {
  properties?: Record<string, { type?: string | string[] }>;
  required?: string[];
}[]) {
  for (const field of ['stableKey', 'humanMessage']) {
    const property = schema.properties?.[field];
    if (property?.type === 'string') property.type = ['null', 'string'];
  }
}
copySchema('UserPreferencesView');
copySchema('PatchUserPreferencesRequest');
fs.mkdirSync('./node_modules/.cache', { recursive: true });
fs.writeFileSync('./node_modules/.cache/citadel-openapi.json', `${JSON.stringify(contract, null, 2)}\n`);
