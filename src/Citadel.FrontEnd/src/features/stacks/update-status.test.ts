import { AutoUpdateStatus, StackView } from '@/api/generated/api.types';
import { getStackUpdateStatus, hasStackUpdateAvailable } from './update-status';

describe('Stack update status', () => {
  it('reports an available service-image update', () => {
    const stack = createStack({
      $type: 'WebEditor',
      recreateStackOnNewImageState: {
        autoUpdateStates: [
          {
            serviceName: 'api',
            imageName: 'example/api:latest',
            currentDigest: 'sha256:old',
            remoteDigest: 'sha256:new',
            lastCheckedAt: '2026-07-26T12:00:00Z',
            updateAvailable: true,
          },
        ],
      },
    });

    expect(getStackUpdateStatus(stack)).toBe(AutoUpdateStatus.UpdateAvailable);
    expect(hasStackUpdateAvailable(stack)).toBe(true);
  });

  it('reports only a relevant newer Git commit as available', () => {
    const current = createGitStack(null);
    const relevant = createGitStack('new-commit');

    expect(getStackUpdateStatus(current)).toBe(AutoUpdateStatus.UpToDate);
    expect(hasStackUpdateAvailable(current)).toBe(false);
    expect(getStackUpdateStatus(relevant)).toBe(AutoUpdateStatus.UpdateAvailable);
    expect(hasStackUpdateAvailable(relevant)).toBe(true);
  });

  it('excludes an uninitialized state', () => {
    const stack = createStack({
      $type: 'WebEditor',
      recreateStackOnNewImageState: { autoUpdateStates: [] },
    });

    expect(getStackUpdateStatus(stack)).toBe(AutoUpdateStatus.Unknown);
    expect(hasStackUpdateAvailable(stack)).toBe(false);
  });
});

const createStack = (stackUpdateState: StackView['stackUpdateState']): StackView => ({ stackUpdateState }) as StackView;

const createGitStack = (remoteCommitSha: string | null): StackView =>
  createStack({
    $type: 'Git',
    recreateStackOnNewImageState: { autoUpdateStates: [] },
    recreateStackOnNewCommitState: {
      currentCommitSha: 'current-commit',
      remoteCommitSha,
      lastCheckedAt: '2026-07-26T12:00:00Z',
    },
  });
