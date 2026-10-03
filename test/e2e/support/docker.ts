import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const execFileAsync = promisify(execFile);
const e2eRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const composeFile = path.join(e2eRoot, 'compose.yml');
const dockerExecutable = process.platform === 'win32' ? 'docker.exe' : 'docker';

const runInDockerDaemon = async (...args: string[]) => {
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
