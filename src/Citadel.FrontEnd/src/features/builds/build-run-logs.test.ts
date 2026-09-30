import { BuildLogEntry } from '@/api/generated/api.types';
import { formatBuildRunLogViewerEntries } from './build-run-logs';

const format = (message: string, stream = 'stderr') =>
  formatBuildRunLogViewerEntries({
    id: 'log-id',
    buildRunId: 'run-id',
    createdAt: '2026-09-22T00:00:00Z',
    message,
    stream,
  } as BuildLogEntry);

describe('build log severity', () => {
  it('leaves Docker progress neutral and colors only the error line within a chunk', () => {
    const entries = format(
      '#0 building with "default" instance\n#1 transferring dockerfile: 2.77kB done\n#1 DONE 0.0s\n#2 ERROR: missing source\nERROR: failed to build: missing source\n',
    );
    expect(entries.map((entry) => entry.severity)).toEqual([undefined, undefined, undefined, 'error', 'error']);
    expect(entries[0].message).toBe('[stderr] #0 building with "default" instance');
  });
  it.each(['stderr', 'stdout', 'system'])('detects errors by content on %s', (stream) => {
    expect(format('error[E0432]: unresolved import', stream)[0].severity).toBe('error');
    expect(format('npm ERR! command failed', stream)[0].severity).toBe('error');
    expect(format('Synchronizing Build Git source...', stream)[0].severity).toBeUndefined();
    expect(format('test handles_failed_login ... ok', stream)[0].severity).toBeUndefined();
  });
  it('handles ANSI colors, BuildKit timestamps, warnings and carriage returns', () => {
    const entries = format(
      '#7 1.42 \u001b[31merror: compilation failed\u001b[0m\r#8 CANCELED\r#9 0.2 warning: unused import',
    );
    expect(entries.map((entry) => entry.severity)).toEqual(['error', 'warning', 'warning']);
  });
});
