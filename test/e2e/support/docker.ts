import { setTimeout as delay } from 'node:timers/promises';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const execFileAsync = promisify(execFile);
const e2eRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const composeFile = path.join(e2eRoot, 'compose.yml');
const dockerExecutable = process.platform === 'win32' ? 'docker.exe' : 'docker';

export const runInDockerDaemon = async (...args: string[]) => {
  const result = await execFileAsync(
    dockerExecutable,
    ['compose', '-f', composeFile, 'exec', '-T', 'docker', 'docker', ...args],
    {
      cwd: e2eRoot,
      env: process.env,
      maxBuffer: 16 * 1024 * 1024,
    },
  );
  return result.stdout.trim();
};

export type RuntimeContainer = {
  id: string;
  image: string;
  labels: Record<string, string>;
  name: string;
  running: boolean;
};

export const getRuntimeContainer = async (stackId: string): Promise<RuntimeContainer> => {
  const ids = (await runInDockerDaemon(
    'ps',
    '-q',
    '--filter',
    `label=com.citadel.stack-id=${stackId}`,
  ))
    .split(/\r?\n/)
    .filter(Boolean);

  if (ids.length !== 1) {
    throw new Error(`Expected one Docker container for stack ${stackId}, found ${ids.length}`);
  }

  const inspected = JSON.parse(await runInDockerDaemon('inspect', ids[0])) as Array<{
    Config: {
      Image: string;
      Labels: Record<string, string>;
    };
    Id: string;
    Name: string;
    State: {
      Running: boolean;
    };
  }>;
  const container = inspected[0];

  return {
    id: container.Id,
    image: container.Config.Image,
    labels: container.Config.Labels,
    name: container.Name.replace(/^\//, ''),
    running: container.State.Running,
  };
};

export const getRuntimeContainerLogs = (containerId: string) =>
  runInDockerDaemon('logs', containerId);

export const emitRuntimeContainerLog = async (containerId: string, marker: string) => {
  if (!/^[A-Za-z0-9._-]+$/.test(marker)) {
    throw new Error(`Unsafe runtime log marker '${marker}'`);
  }

  await runInDockerDaemon(
    'exec',
    containerId,
    'sh',
    '-c',
    `echo ${marker} > /proc/1/fd/1`,
  );
};

// Lifecycle operations are scoped to the selected disposable Compose project.
export const controlRuntimeService = async (action: 'pause' | 'unpause' | 'kill' | 'start', service: 'docker' | 'citadel') => {
  await execFileAsync(dockerExecutable, [
    'compose', '-f', composeFile, action,
    ...(action === 'kill' ? ['--signal', 'SIGKILL'] : []), service,
  ], { cwd: e2eRoot, env: process.env });
  if (action === 'unpause' && service === 'docker') {
    // Docker can still report unhealthy from probes made while paused. Compose
    // refuses to start dependent services until the health check has recovered.
    const { stdout } = await execFileAsync(dockerExecutable, [
      'compose', '-f', composeFile, 'ps', '-q', 'docker',
    ], { cwd: e2eRoot, env: process.env });
    const containerId = stdout.trim();
    if (!containerId) throw new Error('Disposable Docker service was not found');
    const deadline = Date.now() + 60_000;
    while (true) {
      const { stdout: health } = await execFileAsync(dockerExecutable, [
        'inspect', '--format', '{{.State.Health.Status}}', containerId,
      ], { cwd: e2eRoot, env: process.env });
      if (health.trim() === 'healthy') break;
      if (Date.now() >= deadline) throw new Error('Disposable Docker service did not recover after unpause');
      await delay(500);
    }
  }
};

export const clearCopyFixtureHistory = async (stackId: string, policyId: string) => {
  const uuid = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
  if (!uuid.test(stackId) || !uuid.test(policyId)) throw new Error('Invalid fixture identifier');
  // Only test-owned history is removed, after API behavior has been asserted.
  // Otherwise restrictive history FKs retain the Local Platform and prevent
  // the next browser test from registering this same disposable Docker daemon.
  const sql = `BEGIN;
    DELETE FROM resourcecopyrestores WHERE copyid IN (SELECT id FROM resourcecopies WHERE sourceid='${stackId}');
    DELETE FROM resourcecopies WHERE sourceid='${stackId}';
    DELETE FROM backuprestoreruns WHERE backuprunid IN (SELECT id FROM backupruns WHERE backuppolicyid='${policyId}');
    DELETE FROM backupruns WHERE backuppolicyid='${policyId}';
    COMMIT;`;
  await execFileAsync(dockerExecutable, [
    'compose', '-f', composeFile, 'exec', '-T', 'postgres',
    'psql', '-U', 'citadel', '-d', 'citadel', '-v', 'ON_ERROR_STOP=1', '-c', sql,
  ], { cwd: e2eRoot, env: process.env });
};
