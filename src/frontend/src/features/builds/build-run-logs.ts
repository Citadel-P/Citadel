import { BuildLogEntry } from '@/api/generated/api.types';
import type { LogEntry, LogSeverity } from '@/components/custom/common';
import { parseCitadelDate } from '@/lib/date-time';

export function mergeBuildRunLogEntries(current: BuildLogEntry[], incoming: BuildLogEntry[]) {
  if (incoming.length === 0) return current;

  const byId = new Map<string, BuildLogEntry>();
  for (const entry of current) byId.set(entry.id, entry);
  for (const entry of incoming) byId.set(entry.id, entry);

  return [...byId.values()].sort(
    (a, b) => String(a.createdAt).localeCompare(String(b.createdAt)) || a.id.localeCompare(b.id),
  );
}

export function formatBuildRunLogEntry(entry: BuildLogEntry) {
  return `[${entry.stream}] ${entry.message}`;
}

export function formatBuildRunLogViewerEntries(entry: BuildLogEntry): LogEntry[] {
  return entry.message
    .split(/\r\n|\r|\n/)
    .filter((line) => line.trim().length > 0)
    .map((message) => ({
      timestamp: formatBuildRunLogTimestamp(entry.createdAt),
      message: formatBuildRunLogEntry({ ...entry, message }),
      severity: getBuildRunLogSeverity(message),
    }));
}

function formatBuildRunLogTimestamp(value: unknown): string | undefined {
  return parseCitadelDate(value)?.toISOString();
}

function getBuildRunLogSeverity(message: string): LogSeverity | undefined {
  // ANSI escape sequences start with the ESC control character.
  // eslint-disable-next-line no-control-regex
  const plain = message.replace(/\x1b\[[0-?]*[ -/]*[@-~]/g, '').trim();
  const text = plain.replace(/^#\d+\s+(?:\d+(?:\.\d+)?\s+)?/, '');

  if (
    /^(?:error(?:\[.*?\])?|fatal|panic)(?:[\s:!]|$)/i.test(text) ||
    /^(?:npm|yarn|pnpm)\s+(?:ERR!|error)(?:\s|:|$)/i.test(text) ||
    /^(?:failed to |failed:|build failed\b|process .+ did not complete successfully)/i.test(text) ||
    /\blevel=(?:error|fatal|panic)\b/i.test(text)
  )
    return 'error';
  if (/^(?:warn(?:ing)?(?:[\s:!]|$)|npm warn\b)/i.test(text) || /^(?:cancell?ed|interrupted|timed out)\b/i.test(text))
    return 'warning';
  if (/^(?:build )?(?:completed successfully|succeeded)\b/i.test(text)) return 'success';
  return undefined;
}
