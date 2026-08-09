import { clsx, type ClassValue } from 'clsx';
import { twMerge } from 'tailwind-merge';
import { truncate } from './truncate';
import yaml from 'js-yaml';

export function cn(...inputs: ClassValue[]): string {
  return twMerge(clsx(inputs));
}

export function toFixedNumber(
  input: number | undefined | null,
  style?: 'percent',
  digits: number = 2,
): string | undefined {
  if (input === null || input === undefined) {
    return undefined;
  }
  if (input === 0) {
    return style === 'percent' ? '0%' : '0';
  }

  if (style === 'percent') {
    return new Intl.NumberFormat('default', {
      style: 'percent',
      minimumFractionDigits: digits,
      maximumFractionDigits: digits,
    }).format(input / 100);
  }

  return input.toFixed(digits);
}

export function getEditedFields(dirtyFields: object | boolean, allValues: object): object {
  if (dirtyFields === true || Array.isArray(dirtyFields)) {
    return allValues;
  }

  return Object.fromEntries(
    Object.keys(dirtyFields).map((key) => [
      key,
      getEditedFields(dirtyFields[key as keyof typeof dirtyFields], allValues[key as keyof typeof allValues]),
    ]),
  );
}

export function formatNumber(value: number): string {
  if (value >= 1_000_000_000) {
    return `${(value / 1_000_000_000).toFixed(1)}B`;
  }
  if (value >= 1_000_000) {
    return `${(value / 1_000_000).toFixed(1)}M`;
  }
  if (value >= 1_000) {
    return `${(value / 1_000).toFixed(1)}k`;
  }
  return value.toString();
}

export function formatId(id?: string): string {
  if (!id) return '';

  const normalized = id.startsWith('sha256:') ? id.slice(7) : id;

  return truncate(normalized, 12, 'right', true);
}

export const filterBySplit = <T>(items: T[] | undefined, search: string, extract: (item: T) => string) => {
  const split = search.toLowerCase().split(' ');
  return (
    (split.length
      ? items?.filter((item) => {
          const target = extract(item).toLowerCase();
          return split.every((term) => target.includes(term));
        })
      : items) ?? []
  );
};
export const normalizeDockerId = (id?: string) => (id ? id.slice(0, 12).toLowerCase() : undefined);

export const getTaskName = (task: { id: string; name?: string | null; slot?: number | null }) =>
  task.name?.trim() || `Task ${task.slot ?? task.id.slice(0, 12)}`;

export const capitalize = (s: string) => s.charAt(0).toUpperCase() + s.slice(1);

export const pluralize = (word: string) => (word.endsWith('y') ? word.slice(0, -1) + 'ies' : word + 's');

export const formatActivityEvent = (event: string) =>
  event === 'SwarmServiceForceUpdated' ? 'Swarm Service Tasks Restarted' : event.replace(/([a-z])([A-Z])/g, '$1 $2');

const environmentNamePattern = /^[A-Za-z_][A-Za-z0-9_]*$/;
const environmentReferencePattern = /\$\{([^}]+)\}/g;

export const createEnvironmentVariableValidator = (configurationNames: Iterable<string>, resourceLabel: string) => {
  const knownNames = new Set(configurationNames);

  return (line: string): string | null => {
    const separator = line.indexOf('=');
    const key = (separator < 0 ? line : line.slice(0, separator)).trim();

    if (!environmentNamePattern.test(key)) {
      return `${key || 'Environment key'} is not a valid environment key.`;
    }

    if (separator < 0) {
      return knownNames.has(key) ? null : `${key} is not defined in ${resourceLabel} or global variables.`;
    }

    const value = line.slice(separator + 1);
    for (const match of value.matchAll(environmentReferencePattern)) {
      const name = match[1]?.trim() ?? '';
      if (!environmentNamePattern.test(name)) {
        return `${match[0]} is not a supported variable reference.`;
      }

      if (!knownNames.has(name)) {
        return `${name} is not defined in ${resourceLabel} or global variables.`;
      }
    }

    return null;
  };
};

export const serializeData = (data: unknown, format: 'json' | 'yaml' = 'yaml') => {
  try {
    if (format === 'yaml') {
      return yaml.dump(data, { noRefs: true });
    }
    return JSON.stringify(data, null, 2);
  } catch {
    return format === 'yaml' ? '# Error serializing YAML' : '// Error serializing JSON';
  }
};

export const isUnmanagedContainer = (container: {
  isSystem?: boolean;
  isSwarmTask?: boolean;
  hasCitadelOwnershipLabels?: boolean;
  deploymentId?: string | null;
  stackId?: string | null;
}) =>
  container.isSystem !== true &&
  container.isSwarmTask !== true &&
  container.hasCitadelOwnershipLabels !== true &&
  !container.deploymentId &&
  !container.stackId;
