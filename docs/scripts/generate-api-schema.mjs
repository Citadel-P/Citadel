import { readFile, rm } from 'node:fs/promises';
import { spawn } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const docsRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const repositoryRoot = path.resolve(docsRoot, '..');
const project = path.join(repositoryRoot, 'src', 'Citadel.WebApi', 'Citadel.WebApi.csproj');
const publicSchema = path.join(repositoryRoot, 'src', 'schema', 'Citadel.WebApi_public.json');
const generationCache = path.join(
  repositoryRoot,
  'src',
  'Citadel.WebApi',
  'obj',
  'Citadel.WebApi.OpenApiFiles.cache',
);

// Microsoft.Extensions.ApiDescription.Server does not include the output
// directory in its incremental-build inputs. Removing its bounded cache makes
// this command regenerate the canonical schema even after another build wrote
// OpenAPI files somewhere else.
await rm(generationCache, { force: true });

await run('dotnet', [
  'build',
  project,
  '--configuration',
  'Release',
  '--maxcpucount:1',
  '-p:OpenApiGenerateDocuments=true',
  '-p:OpenApiGenerateDocumentsOnBuild=true',
]);

const schema = JSON.parse(await readFile(publicSchema, 'utf8'));
if (schema.info?.title !== 'Citadel API Preview') {
  throw new Error(`${publicSchema} is not the generated Citadel API Preview document.`);
}

console.log(`Generated ${path.relative(repositoryRoot, publicSchema)}.`);

function run(command, args) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      cwd: repositoryRoot,
      stdio: 'inherit',
      shell: process.platform === 'win32',
    });
    child.once('error', reject);
    child.once('exit', (code) => {
      if (code === 0) resolve();
      else reject(new Error(`${command} exited with code ${code}.`));
    });
  });
}
