import {
  DashboardWorkloadKind,
  DashboardMetric,
  type DashboardConfiguration,
  type DashboardWidgetSettings,
} from '@/api/generated/api.types';
import { randomUuid } from '@/lib/utils';

type StarterWidget = { title: string; settings: DashboardWidgetSettings };

import { metricSettings } from './catalogue';
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
    { title: 'Upcoming schedules', settings: { kind: 'UpcomingSchedules', limit: 5 } },
    { title: 'Updates', settings: { kind: 'Updates' } },
    { title: 'Recent activity', settings: { kind: 'RecentActivity', limit: 5 } },
  ],
} satisfies Record<string, StarterWidget[]>;

export type DashboardTemplate = keyof typeof templates;

export function fromTemplate(template: DashboardTemplate): DashboardConfiguration {
  return {
    schemaVersion: 1,
    tags: [],
    rangeMinutes: 60,
    widgets: templates[template].map(({ title, settings }, index) => ({
      id: randomUuid(),
      title,
      settings: { ...settings },
      layout: { x: (index % 2) * 6, y: Math.floor(index / 2) * 4, w: 6, h: 4 },
    })),
  };
}

export const dashboardTemplates = Object.keys(templates) as DashboardTemplate[];
