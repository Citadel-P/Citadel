import { DiagnosticSeverity } from '@/lib/monaco/diagnostics';
import { isMap, isNode, isScalar, isSeq, LineCounter, parseDocument, type Node, type Pair, type YAMLMap } from 'yaml';

import type { MonacoDiagnostic } from '@/lib/monaco';

const topLevelKeys = keys('version', 'services', 'networks', 'volumes', 'secrets', 'configs');
const serviceKeys = keys(
  'image',
  'build',
  'command',
  'entrypoint',
  'working_dir',
  'user',
  'environment',
  'env_file',
  'labels',
  'healthcheck',
  'hostname',
  'stop_grace_period',
  'logging',
  'deploy',
  'endpoint_mode',
  'networks',
  'volumes',
  'ports',
  'secrets',
  'configs',
  'tmpfs',
);
const deployKeys = keys(
  'mode',
  'replicas',
  'placement',
  'resources',
  'restart_policy',
  'update_config',
  'rollback_config',
  'labels',
);
const placementKeys = keys('constraints', 'preferences', 'max_replicas_per_node');
const resourceKeys = keys('limits', 'reservations');
const resourceLimitKeys = keys('cpus', 'memory', 'pids');
const restartPolicyKeys = keys('condition', 'delay', 'max_attempts', 'window');
const updateKeys = keys('parallelism', 'delay', 'failure_action', 'monitor', 'max_failure_ratio', 'order');
const healthcheckKeys = keys('test', 'interval', 'timeout', 'retries', 'start_period', 'start_interval', 'disable');
const loggingKeys = keys('driver', 'options');
const volumeKeys = keys('driver', 'driver_opts', 'external', 'name', 'labels');
const networkKeys = keys(
  'driver',
  'driver_opts',
  'attachable',
  'external',
  'name',
  'labels',
  'internal',
  'enable_ipv4',
  'enable_ipv6',
);
const secretConfigKeys = keys('file', 'external', 'name', 'labels', 'driver', 'template_driver');
const mountKeys = keys('type', 'source', 'target', 'read_only', 'bind', 'volume', 'tmpfs', 'consistency');
const bindKeys = keys('propagation', 'create_host_path', 'selinux');
const namedVolumeMountKeys = keys('nocopy', 'subpath');
const tmpfsKeys = keys('size', 'mode');
const portKeys = keys('name', 'target', 'published', 'protocol', 'app_protocol', 'mode');
const referenceKeys = keys('source', 'target', 'uid', 'gid', 'mode');

type DiagnosticContext = {
  diagnostics: MonacoDiagnostic[];
  lineCounter: LineCounter;
};

function keys(...values: string[]) {
  return new Set(values);
}

const scalarText = (value: unknown): string | undefined =>
  isScalar(value) && value.value !== null && value.value !== undefined ? String(value.value) : undefined;

const findPair = (map: YAMLMap, name: string): Pair | undefined =>
  map.items.find((pair) => scalarText(pair.key)?.toLowerCase() === name.toLowerCase());

const nodeValue = (pair: Pair | undefined): Node | null => {
  const value = pair?.value;
  return isNode(value) ? value : null;
};

const addDiagnostic = (context: DiagnosticContext, node: unknown, message: string, severity: DiagnosticSeverity) => {
  const range = isNode(node) ? node.range : undefined;
  const start = context.lineCounter.linePos(range?.[0] ?? 0);
  const end = context.lineCounter.linePos(range?.[1] ?? (range?.[0] ?? 0) + 1);

  context.diagnostics.push({
    lineNumber: start.line,
    startColumn: start.col,
    endColumn: end.line === start.line ? Math.max(start.col + 1, end.col) : start.col + 1,
    message,
    severity,
  });
};

const validateKeys = (
  map: YAMLMap,
  allowedKeys: ReadonlySet<string>,
  path: string,
  context: DiagnosticContext,
  allowExtensions = false,
) => {
  for (const pair of map.items) {
    const key = scalarText(pair.key);
    if (!key) continue;

    const normalized = key.toLowerCase();
    const extensionAllowed = allowExtensions && normalized.startsWith('x-') && !normalized.startsWith('x-citadel.');
    if (allowedKeys.has(normalized) || extensionAllowed) continue;

    const fieldPath = path ? `${path}.${key}` : key;
    addDiagnostic(context, pair.key, `'${fieldPath}' is not supported by Swarm Stacks.`, DiagnosticSeverity.Error);
  }
};

const validateNestedMap = (
  parent: YAMLMap,
  key: string,
  allowedKeys: ReadonlySet<string>,
  parentPath: string,
  context: DiagnosticContext,
) => {
  const value = nodeValue(findPair(parent, key));
  if (isMap(value)) validateKeys(value, allowedKeys, `${parentPath}.${key}`, context);
};

const validateEnum = (
  map: YAMLMap,
  key: string,
  allowedValues: readonly string[],
  parentPath: string,
  context: DiagnosticContext,
) => {
  const pair = findPair(map, key);
  const value = scalarText(pair?.value);
  if (!value || allowedValues.some((allowed) => allowed.toLowerCase() === value.toLowerCase())) return;

  addDiagnostic(
    context,
    pair?.value ?? pair?.key,
    `'${parentPath}.${key}' has unsupported value '${value}'.`,
    DiagnosticSeverity.Error,
  );
};

const validateReservedLabels = (parent: YAMLMap, context: DiagnosticContext) => {
  const labels = nodeValue(findPair(parent, 'labels'));
  const candidates: Array<{ name: string; node: unknown }> = [];

  if (isMap(labels)) {
    for (const pair of labels.items) {
      const name = scalarText(pair.key);
      if (name) candidates.push({ name, node: pair.key });
    }
  } else if (isSeq(labels)) {
    for (const item of labels.items) {
      const value = scalarText(item);
      if (value) candidates.push({ name: value.split('=', 1)[0], node: item });
    }
  }

  for (const candidate of candidates) {
    const name = candidate.name.toLowerCase();
    if (!name.startsWith('com.citadel.') && !name.startsWith('x-citadel.')) continue;

    addDiagnostic(
      context,
      candidate.node,
      `Label '${candidate.name}' is reserved for Citadel ownership.`,
      DiagnosticSeverity.Error,
    );
  }
};

const validateDeploy = (service: YAMLMap, servicePath: string, context: DiagnosticContext) => {
  const deploy = nodeValue(findPair(service, 'deploy'));
  if (!isMap(deploy)) return;

  const deployPath = `${servicePath}.deploy`;
  validateKeys(deploy, deployKeys, deployPath, context);
  validateReservedLabels(deploy, context);
  validateEnum(deploy, 'mode', ['replicated', 'global'], deployPath, context);
  validateNestedMap(deploy, 'placement', placementKeys, deployPath, context);
  validateNestedMap(deploy, 'restart_policy', restartPolicyKeys, deployPath, context);
  validateNestedMap(deploy, 'update_config', updateKeys, deployPath, context);
  validateNestedMap(deploy, 'rollback_config', updateKeys, deployPath, context);

  const resources = nodeValue(findPair(deploy, 'resources'));
  if (isMap(resources)) {
    const resourcesPath = `${deployPath}.resources`;
    validateKeys(resources, resourceKeys, resourcesPath, context);
    validateNestedMap(resources, 'limits', resourceLimitKeys, resourcesPath, context);
    validateNestedMap(resources, 'reservations', resourceLimitKeys, resourcesPath, context);
  }

  const restartPolicy = nodeValue(findPair(deploy, 'restart_policy'));
  if (isMap(restartPolicy))
    validateEnum(restartPolicy, 'condition', ['none', 'on-failure', 'any'], `${deployPath}.restart_policy`, context);

  const updateConfig = nodeValue(findPair(deploy, 'update_config'));
  if (isMap(updateConfig)) {
    validateEnum(
      updateConfig,
      'failure_action',
      ['pause', 'continue', 'rollback'],
      `${deployPath}.update_config`,
      context,
    );
    validateEnum(updateConfig, 'order', ['stop-first', 'start-first'], `${deployPath}.update_config`, context);
  }

  const rollbackConfig = nodeValue(findPair(deploy, 'rollback_config'));
  if (isMap(rollbackConfig)) {
    validateEnum(rollbackConfig, 'failure_action', ['pause', 'continue'], `${deployPath}.rollback_config`, context);
    validateEnum(rollbackConfig, 'order', ['stop-first', 'start-first'], `${deployPath}.rollback_config`, context);
  }
};

const isLikelyHostPath = (value: string) =>
  value.startsWith('/') ||
  value.startsWith('./') ||
  value.startsWith('../') ||
  value.startsWith('~/') ||
  value.includes('\\');

const validateMounts = (service: YAMLMap, servicePath: string, context: DiagnosticContext) => {
  const volumes = nodeValue(findPair(service, 'volumes'));
  if (!isSeq(volumes)) return;

  volumes.items.forEach((item, index) => {
    const path = `${servicePath}.volumes[${index}]`;
    const shortSyntax = scalarText(item);
    if (shortSyntax) {
      const source = shortSyntax.split(':', 1)[0];
      if (source && isLikelyHostPath(source)) {
        addDiagnostic(
          context,
          item,
          'Bind mounts require the same host path on every eligible Swarm node.',
          DiagnosticSeverity.Warning,
        );
      }
      return;
    }

    if (!isMap(item)) return;
    validateKeys(item, mountKeys, path, context);
    validateNestedMap(item, 'bind', bindKeys, path, context);
    validateNestedMap(item, 'volume', namedVolumeMountKeys, path, context);
    validateNestedMap(item, 'tmpfs', tmpfsKeys, path, context);
    validateEnum(item, 'type', ['volume', 'bind', 'tmpfs'], path, context);

    if (scalarText(findPair(item, 'type')?.value)?.toLowerCase() === 'bind') {
      addDiagnostic(
        context,
        findPair(item, 'type')?.value ?? item,
        'Bind mounts require the same host path on every eligible Swarm node.',
        DiagnosticSeverity.Warning,
      );
    }
  });
};

const validatePorts = (service: YAMLMap, servicePath: string, context: DiagnosticContext) => {
  const ports = nodeValue(findPair(service, 'ports'));
  if (!isSeq(ports)) return;

  ports.items.forEach((item, index) => {
    if (!isMap(item)) return;
    const path = `${servicePath}.ports[${index}]`;
    validateKeys(item, portKeys, path, context);
    validateEnum(item, 'mode', ['ingress', 'host'], path, context);
    validateEnum(item, 'protocol', ['tcp', 'udp', 'sctp'], path, context);

    const mode = scalarText(findPair(item, 'mode')?.value)?.toLowerCase();
    const published = scalarText(findPair(item, 'published')?.value)?.trim();
    if (mode === 'host' && published) {
      addDiagnostic(
        context,
        item,
        'A fixed Host-mode published port can schedule only where that port is available.',
        DiagnosticSeverity.Warning,
      );
    }
  });
};

const validateReferences = (
  service: YAMLMap,
  key: 'secrets' | 'configs',
  servicePath: string,
  context: DiagnosticContext,
) => {
  const references = nodeValue(findPair(service, key));
  if (!isSeq(references)) return;

  references.items.forEach((item, index) => {
    if (isMap(item)) validateKeys(item, referenceKeys, `${servicePath}.${key}[${index}]`, context);
  });
};

const validateDefinitions = (
  root: YAMLMap,
  key: 'networks' | 'volumes' | 'secrets' | 'configs',
  allowedKeys: ReadonlySet<string>,
  context: DiagnosticContext,
) => {
  const definitions = nodeValue(findPair(root, key));
  if (!isMap(definitions)) return;

  for (const pair of definitions.items) {
    const name = scalarText(pair.key);
    const definition = nodeValue(pair);
    if (!name || !isMap(definition)) continue;

    validateKeys(definition, allowedKeys, `${key}.${name}`, context, true);
    validateReservedLabels(definition, context);

    if (key === 'volumes' && scalarText(findPair(definition, 'driver')?.value)?.toLowerCase() === 'local') {
      addDiagnostic(
        context,
        pair.key,
        `Volume '${name}' uses the node-local driver and is not portable between nodes.`,
        DiagnosticSeverity.Warning,
      );
    }
  }
};

export const getSwarmComposeDiagnostics = (
  compose: string | undefined,
  boundBuildServices: readonly string[] = [],
): MonacoDiagnostic[] => {
  if (!compose?.trim()) return [];

  const lineCounter = new LineCounter();
  const document = parseDocument(compose, { lineCounter, prettyErrors: false, uniqueKeys: true });
  if (document.errors.length > 0 || !isMap(document.contents)) return [];

  const context: DiagnosticContext = { diagnostics: [], lineCounter };
  const root = document.contents;
  validateKeys(root, topLevelKeys, '', context, true);
  validateDefinitions(root, 'networks', networkKeys, context);
  validateDefinitions(root, 'volumes', volumeKeys, context);
  validateDefinitions(root, 'secrets', secretConfigKeys, context);
  validateDefinitions(root, 'configs', secretConfigKeys, context);

  const services = nodeValue(findPair(root, 'services'));
  if (!isMap(services) || services.items.length === 0) {
    addDiagnostic(
      context,
      findPair(root, 'services')?.key ?? root,
      'Compose must define at least one Service.',
      DiagnosticSeverity.Error,
    );
    return context.diagnostics;
  }

  const bindings = new Set(boundBuildServices.map((name) => name.trim().toLowerCase()).filter(Boolean));
  for (const pair of services.items) {
    const serviceName = scalarText(pair.key);
    const service = nodeValue(pair);
    if (!serviceName || !isMap(service)) continue;

    const servicePath = `services.${serviceName}`;
    validateKeys(service, serviceKeys, servicePath, context);
    validateReservedLabels(service, context);
    validateEnum(service, 'endpoint_mode', ['vip', 'dnsrr'], servicePath, context);
    validateNestedMap(service, 'healthcheck', healthcheckKeys, servicePath, context);
    validateNestedMap(service, 'logging', loggingKeys, servicePath, context);
    validateDeploy(service, servicePath, context);
    validateMounts(service, servicePath, context);
    validatePorts(service, servicePath, context);
    validateReferences(service, 'secrets', servicePath, context);
    validateReferences(service, 'configs', servicePath, context);

    const image = scalarText(findPair(service, 'image')?.value)?.trim();
    const build = findPair(service, 'build');
    const hasBuildBinding = bindings.has(serviceName.toLowerCase());
    if (!image && !hasBuildBinding) {
      addDiagnostic(
        context,
        pair.key,
        `Service '${serviceName}' requires an image or a Citadel Build binding that produces a registry image.`,
        DiagnosticSeverity.Error,
      );
    }
    if (build && !hasBuildBinding) {
      addDiagnostic(
        context,
        build.key,
        `Service '${serviceName}' uses build without a Citadel Build binding.`,
        DiagnosticSeverity.Error,
      );
    }
  }

  return context.diagnostics;
};
