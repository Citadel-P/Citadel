import {
  DashboardWorkloadKind,
  DashboardMetric,
  type DashboardConfiguration,
  type DashboardWidgetSettings,
} from '@/api/generated/api.types';
import { randomUuid } from '@/lib/utils';

type StarterWidget = { title: string; settings: DashboardWidgetSettings; size?: { w: number; h: number } };

import { nextPosition } from './layout';
import { catalogue, metricSettings } from './catalogue';
const templates = {
  Blank: [],
  Overview: [
    { title: 'Resource summary', settings: { kind: 'ResourceSummary' } },
    { title: 'Platforms', settings: { kind: 'Platforms', limit: 5 } },
    { title: 'Open alerts', settings: { kind: 'OpenAlerts', limit: 5 } },
    { title: 'Backup activity', settings: { kind: 'Backups', limit: 5 } },
    { title: 'Recent activity', settings: { kind: 'RecentActivity', limit: 5 } },
  ],
  'Infrastructure and capacity': [
    { title: 'Platforms', settings: { kind: 'Platforms', limit: 5 } },
    { title: 'CPU usage', settings: metricSettings(DashboardMetric.Cpu) },
    { title: 'Memory usage', settings: metricSettings(DashboardMetric.Memory) },
    { title: 'Network traffic', settings: metricSettings(DashboardMetric.Network) },
  ],
  Application: [
    { title: 'Workloads', settings: { kind: 'Workloads', resourceKind: DashboardWorkloadKind.Deployment, limit: 5 } },
    { title: 'Application CPU', settings: metricSettings(DashboardMetric.Cpu) },
    { title: 'Open alerts', settings: { kind: 'OpenAlerts', limit: 5 } },
    { title: 'Backup activity', settings: { kind: 'Backups', limit: 5 } },
    { title: 'Recent activity', settings: { kind: 'RecentActivity', limit: 5 } },
  ],
  Maintenance: [
    { title: 'Backup activity', settings: { kind: 'Backups', limit: 5 } },
    { title: 'Upcoming schedules', size: { w: 6, h: 4 }, settings: { kind: 'UpcomingSchedules', limit: 5 } },
    { title: 'Updates', size: { w: 6, h: 4 }, settings: { kind: 'Updates' } },
    { title: 'Recent activity', settings: { kind: 'RecentActivity', limit: 5 } },
  ],
} satisfies Record<string, StarterWidget[]>;

export type DashboardTemplate = keyof typeof templates;

export function fromTemplate(template: DashboardTemplate): DashboardConfiguration {
  const widgets: DashboardConfiguration['widgets'] = [];
  const starters: readonly StarterWidget[] = templates[template];
  for (const { title, settings, size } of starters) {
    const { w, h } = size ?? catalogue[settings.kind].size;
    const layout = nextPosition(widgets, w, h);
    if (!layout) throw new Error('Dashboard template exceeds grid bounds');
    widgets.push({ id: randomUuid(), title, settings: { ...settings }, layout });
  }
  return { schemaVersion: 1, tags: [], rangeMinutes: 60, widgets };
}

export const dashboardTemplates = Object.keys(templates) as DashboardTemplate[];
