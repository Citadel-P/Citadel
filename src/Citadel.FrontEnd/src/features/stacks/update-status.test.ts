import { AutoUpdateStatus, ResourceControlState, StackReleaseStatus, StackView } from '@/api/generated/api.types';
import {
  canCheckStackUpdates,
  getStackImageUpdateCheckMessage,
  getStackUpdateStatus,
  hasStackUpdateAvailable,
} from './update-status';

describe('Stack update status', () => {
  it('does not allow update checks before the stack is applied', () => {
    expect(canCheckStackUpdates({ status: StackReleaseStatus.Created } as StackView)).toBe(false);
    expect(canCheckStackUpdates({ status: StackReleaseStatus.Healthy } as StackView)).toBe(true);
    expect(
      canCheckStackUpdates({
        status: StackReleaseStatus.Healthy,
        controlState: ResourceControlState.Processing,
      } as StackView),
    ).toBe(false);
  });

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

  it('reports that stack images are current when no update is available', () => {
    const stack = createStack({
      $type: 'WebEditor',
      recreateStackOnNewImageState: {
        autoUpdateStates: [createImageState('api', false), createImageState('worker', false)],
      },
    });

    expect(getStackImageUpdateCheckMessage(stack)).toEqual({
      kind: 'success',
      title: 'Stack images are up to date',
      description: 'No newer service image digest was found.',
    });
  });

  it('reports available updates with redeploy guidance', () => {
    const stack = createStack({
      $type: 'WebEditor',
      recreateStackOnNewImageState: {
        autoUpdateStates: [createImageState('api', true), createImageState('worker', true)],
      },
    });

    expect(getStackImageUpdateCheckMessage(stack)).toEqual({
      kind: 'info',
      title: 'Updates available',
      description: '2 service images have newer digests. Redeploy the stack to apply the changes.',
    });
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

const createImageState = (serviceName: string, updateAvailable: boolean) => ({
  serviceName,
  imageName: `example/${serviceName}:latest`,
  currentDigest: 'sha256:current',
  remoteDigest: updateAvailable ? 'sha256:new' : 'sha256:current',
  lastCheckedAt: '2026-07-26T12:00:00Z',
  updateAvailable,
});
