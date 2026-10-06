import { readFile } from 'node:fs/promises';
import path from 'node:path';

export const dynamic = 'force-static';

export async function GET() {
  const schema = await readFile(path.resolve(process.cwd(), '../schema/public-v1.json'), 'utf8');
  return new Response(schema, {
    headers: { 'Content-Type': 'application/json' },
  });
}
