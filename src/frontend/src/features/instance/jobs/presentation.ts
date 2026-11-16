import { JobCategory, JobStateView, type JobStatusView } from '@/api/generated/api.types';

export type JobDisplayState = JobStateView | 'failed';
export const stateLabel = (value: JobDisplayState) =>
  ({
    disabled: 'Disabled',
    notStarted: 'Not started',
    waiting: 'Waiting',
    running: 'Running',
    backingOff: 'Backing off',
    stopped: 'Stopped',
    unavailable: 'Unmeasured',
    failed: 'Failed',
  })[value];

// Unmeasured listeners also report Unavailable. Historical errors alone are not failures.
export const displayState = (job: JobStatusView): JobDisplayState =>
  job.state === 'unavailable' && (job.lastOutcome === 'failed' || job.lastOutcome === 'interrupted')
    ? 'failed'
    : job.state;

const priority: JobDisplayState[] = [
  'failed',
  JobStateView.BackingOff,
  JobStateView.Running,
  JobStateView.Unavailable,
  JobStateView.NotStarted,
  JobStateView.Waiting,
  JobStateView.Stopped,
  JobStateView.Disabled,
];

const resourceLabels: Record<JobCategory, string> = {
  Alert: 'Alert',
  Build: 'Build',
  Backup: 'Backup',
  Script: 'Script',
  GitRepository: 'Git repository',
  Deployment: 'Deployment',
  SwarmService: 'Swarm service',
  Stack: 'Stack',
  Image: 'Image',
  ResourceCopy: 'Resource copy',
  Volume: 'Volume',
  Container: 'Container',
  Updates: 'Updates',
  Platform: 'Platform',
  Maintenance: 'Maintenance',
  Core: 'Core',
  Other: 'Other',
};

export function resourceGroup(job: JobStatusView) {
  return resourceLabels[job.resourceType];
}

export function groupJobs(jobs: JobStatusView[]) {
  const groups = new Map<string, JobStatusView[]>();
  for (const job of jobs) {
    const name = resourceGroup(job);
    const children = groups.get(name) ?? [];
    children.push(job);
    groups.set(name, children);
  }
  return [...groups]
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([name, children]) => {
      const states = children.map(displayState);
      const state = priority.find((candidate) => states.includes(candidate))!;
      const summary = priority
        .filter((candidate) => states.includes(candidate))
        .map((candidate) => `${states.filter((s) => s === candidate).length} ${stateLabel(candidate).toLowerCase()}`)
        .join(' · ');
      return { name, jobs: children, state, summary };
    });
}

export type JobGroup = ReturnType<typeof groupJobs>[number];
