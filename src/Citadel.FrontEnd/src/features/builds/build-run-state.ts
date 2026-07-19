import { BuildProjectView, BuildRunStatus, BuildRunView, ResourceControlState } from '@/api/generated/api.types';
import { parseCitadelDate } from '@/lib/date-time';

export function isActiveBuildRun(run: Pick<BuildRunView, 'status'>) {
  return (
    run.status === BuildRunStatus.Queued ||
    run.status === BuildRunStatus.Preparing ||
    run.status === BuildRunStatus.Running
  );
}

export function isTerminalBuildRunStatus(status: BuildRunStatus) {
  return (
    status === BuildRunStatus.Succeeded ||
    status === BuildRunStatus.Failed ||
    status === BuildRunStatus.TimedOut ||
    status === BuildRunStatus.Cancelled ||
    status === BuildRunStatus.Interrupted
  );
}

export function isBuildProjectActive(project: Pick<BuildProjectView, 'currentRunId' | 'controlState' | 'latestRun'>) {
  const hasCurrentRun = Boolean(project.currentRunId);
  const isProcessing = project.controlState === ResourceControlState.Processing;

  if (project.latestRun) {
    const latestRunIsCurrent = project.currentRunId === project.latestRun.id;
    if (isActiveBuildRun(project.latestRun)) return latestRunIsCurrent || (!hasCurrentRun && isProcessing);
    if (!hasCurrentRun || latestRunIsCurrent) return false;
  }

  return isProcessing || hasCurrentRun;
}

export function pickMostAdvancedBuildRun(
  left: BuildRunView | null | undefined,
  right: BuildRunView | null | undefined,
): BuildRunView | null | undefined {
  if (!left) return right;
  if (!right) return left;
  if (left.id !== right.id) return right;

  const leftRank = getBuildRunStatusRank(left.status);
  const rightRank = getBuildRunStatusRank(right.status);
  if (leftRank !== rightRank) return leftRank > rightRank ? left : right;

  const leftTime = getBuildRunUpdateTime(left);
  const rightTime = getBuildRunUpdateTime(right);
  return leftTime >= rightTime ? left : right;
}

export function selectBuildProjectLatestRun(
  project: Pick<BuildProjectView, 'currentRunId' | 'controlState' | 'latestRun'>,
  previousRun: BuildRunView | null | undefined,
): BuildRunView | null {
  if (project.latestRun) return pickMostAdvancedBuildRun(previousRun, project.latestRun) ?? project.latestRun;
  if (!previousRun) return null;
  if (!isActiveBuildRun(previousRun)) return previousRun;

  const projectStillOwnsPreviousRun =
    project.currentRunId === previousRun.id && project.controlState === ResourceControlState.Processing;
  return projectStillOwnsPreviousRun ? previousRun : null;
}

function getBuildRunStatusRank(status: BuildRunStatus) {
  switch (status) {
    case BuildRunStatus.Queued:
      return 0;
    case BuildRunStatus.Preparing:
      return 1;
    case BuildRunStatus.Running:
      return 2;
    case BuildRunStatus.Succeeded:
    case BuildRunStatus.Failed:
    case BuildRunStatus.TimedOut:
    case BuildRunStatus.Cancelled:
    case BuildRunStatus.Interrupted:
      return 3;
    default:
      return -1;
  }
}

function getBuildRunUpdateTime(run: BuildRunView) {
  return (
    parseCitadelDate(run.completedAt)?.getTime() ??
    parseCitadelDate(run.startedAt)?.getTime() ??
    parseCitadelDate(run.queuedAt)?.getTime() ??
    0
  );
}
