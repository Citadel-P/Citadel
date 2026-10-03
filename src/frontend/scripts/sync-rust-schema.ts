import { copyFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

// The full API includes authentication, setup and administration used by the UI.
// The public API is the smaller contract intended for external integrations.
copyFileSync(
  fileURLToPath(new URL('../../../schema/v1.json', import.meta.url)),
  fileURLToPath(new URL('../src/api/schema/swagger.json', import.meta.url)),
);
