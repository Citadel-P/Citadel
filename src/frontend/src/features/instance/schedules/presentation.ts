import type { ScheduleView } from '@/api/generated/api.types';
import { formatId } from '@/lib/utils';

export const purposes: Record<string, [string, string, string]> = {
  Script: ['Script', 'Run script', 'scripts'],
  Backup: ['Backup policy', 'Run backup', 'backup-policies'],
  Install: ['Instance', 'Install Core and Agent updates', ''],
  Release: ['Instance', 'Check for Core releases', ''],
  Git: ['Git repository', 'Synchronize branches', 'git-repos'],
  DeploymentImages: ['Deployment', 'Check image updates', 'deployments'],
  ServiceImages: ['Swarm service', 'Check image updates', 'swarm-services'],
  StackImages: ['Stack', 'Check image updates', 'stacks'],
  StackGit: ['Stack', 'Reconcile Git changes', 'stacks'],
  Image: ['Image', 'Check registry tags', ''],
};
export function resourceLink(s: ScheduleView) {
  if (s.purpose === 'Image')
    return s.platformId && s.dockerImageId
      ? `/platforms/${s.platformId}/images/${formatId(s.dockerImageId)}`
      : undefined;
  if (s.purpose === 'Install' || s.purpose === 'Release') return '/updates';
  const resource = purposes[s.purpose]?.[2];
  return resource ? `/${resource}/edit/${s.ownerId}` : undefined;
}
export function recurrence(s: ScheduleView) {
  if (s.cron) {
    if (s.cron === '* * * * *') return 'Every minute';
    const daily = /^(\d+) (\d+) \* \* \*$/.exec(s.cron);
    if (daily) return `Daily at ${daily[2].padStart(2, '0')}:${daily[1].padStart(2, '0')}`;
    const step = /^\*\/(\d+) \* \* \* \*$/.exec(s.cron);
    if (step) return `Every ${step[1]} minutes`;
    const hourly = /^(\d+) \* \* \* \*$/.exec(s.cron);
    if (hourly) return `Hourly at :${hourly[1].padStart(2, '0')}`;
    const weekly = /^(\d+) (\d+) \* \* ([0-7])$/.exec(s.cron);
    if (weekly) {
      const days = ['Sunday', 'Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday'];
      return `Every ${days[Number(weekly[3]) % 7]} at ${weekly[2].padStart(2, '0')}:${weekly[1].padStart(2, '0')}`;
    }
    return 'Custom schedule';
  }
  if (!s.intervalSeconds) return 'Registry check deadline';
  const minutes = s.intervalSeconds / 60;
  return minutes % 60 === 0 ? `Every ${minutes / 60} hour${minutes === 60 ? '' : 's'}` : `Every ${minutes} min`;
}

export function scheduleState(schedule: ScheduleView) {
  if (!schedule.enabled) return 'Disabled';
  if (schedule.needsAttention) return 'Blocked';
  return schedule.nextAt ? 'Enabled' : 'Next run unknown';
}
