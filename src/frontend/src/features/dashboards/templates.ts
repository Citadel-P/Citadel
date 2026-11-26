import {
  DashboardFleetMeasure,
  DashboardStateKind,
  DashboardAttentionTab,
  DashboardMetric,
  type DashboardConfiguration,
  type DashboardWidgetSettings,
} from '@/api/generated/api.types';
import { randomUuid } from '@/lib/utils';

type StarterWidget = { title: string; settings: DashboardWidgetSettings; size?: { w: number; h: number } };

import { nextPosition } from './layout';
import { catalogue, metricSettings } from './catalogue';
const fleet: StarterWidget = {
  title: 'Fleet summary',
  settings: { kind: 'FleetSummary', measures: Object.values(DashboardFleetMeasure) },
};
const attention: StarterWidget = {
  title: 'Needs attention',
  settings: { kind: 'NeedsAttention', kinds: [], limit: 5, tab: DashboardAttentionTab.All },
};
const states = (kind: DashboardStateKind): StarterWidget => ({
  title: 'Resource state',
  settings: { kind: 'ResourceState', resourceKind: kind },
});
const activity: StarterWidget = {
  title: 'Recent activity',
  settings: { kind: 'RecentActivity', limit: 5 },
  size: { w: 12, h: 5 },
};
const templates = {
  Blank: [],
  Overview: [fleet, attention, states(DashboardStateKind.Deployment), activity],
  'Infrastructure and capacity': [
    fleet,
    attention,
    states(DashboardStateKind.Platform),
    { title: 'CPU usage', settings: metricSettings(DashboardMetric.Cpu) },
    { title: 'Memory usage', settings: metricSettings(DashboardMetric.Memory) },
  ],
  Application: [
    attention,
    states(DashboardStateKind.Deployment),
    activity,
    { title: 'Application CPU', settings: metricSettings(DashboardMetric.Cpu) },
  ],
  Maintenance: [
    {
      ...attention,
      title: 'Updates',
      settings: { kind: 'NeedsAttention', kinds: [], limit: 5, tab: DashboardAttentionTab.Updates },
    },
    { title: 'Backup activity', settings: { kind: 'Backups', limit: 5 }, size: { w: 5, h: 7 } },
    { title: 'Upcoming schedules', settings: { kind: 'UpcomingSchedules', limit: 5 }, size: { w: 6, h: 4 } },
    { title: 'Instance updates', settings: { kind: 'Updates' }, size: { w: 6, h: 4 } },
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
  return { schemaVersion: 2, tags: [], rangeMinutes: 60, widgets };
}

export const dashboardTemplates = Object.keys(templates) as DashboardTemplate[];
