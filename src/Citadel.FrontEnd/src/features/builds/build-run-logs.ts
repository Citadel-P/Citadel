import { BuildRunLogEntry } from '@/api/generated/api.types';

export function mergeBuildRunLogEntries(current: BuildRunLogEntry[], incoming: BuildRunLogEntry[]) {
  if (incoming.length === 0) return current;

  const byId = new Map<string, BuildRunLogEntry>();
  for (const entry of current) byId.set(entry.id, entry);
  for (const entry of incoming) byId.set(entry.id, entry);

  return [...byId.values()].sort(
    (a, b) => String(a.createdAt).localeCompare(String(b.createdAt)) || a.id.localeCompare(b.id),
  );
}

export function formatBuildRunLogEntry(entry: BuildRunLogEntry) {
  return `[${entry.stream}] ${entry.message}`;
}
