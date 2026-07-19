import { BuildRunLogEntry } from '@/api/generated/api.types';
import type { LogEntry, LogSeverity } from '@/components/custom/common';
import { parseCitadelDate } from '@/lib/date-time';

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

export function formatBuildRunLogViewerEntry(entry: BuildRunLogEntry): LogEntry {
  return {
    timestamp: formatBuildRunLogTimestamp(entry.createdAt),
    message: formatBuildRunLogEntry(entry),
    severity: getBuildRunLogSeverity(entry),
  };
}

function formatBuildRunLogTimestamp(value: unknown): string | undefined {
  return parseCitadelDate(value)?.toISOString();
}

function getBuildRunLogSeverity(entry: BuildRunLogEntry): LogSeverity | undefined {
  const stream = entry.stream.toLowerCase();
  const message = entry.message.toLowerCase();

  if (stream === 'stderr') return 'error';
  if (message.includes('completed successfully') || message.includes('succeeded')) return 'success';
  if (message.includes('failed') || message.includes('timed out') || message.includes('interrupted')) return 'error';
  if (message.includes('cancelled') || message.includes('canceled')) return 'warning';

  return undefined;
}
