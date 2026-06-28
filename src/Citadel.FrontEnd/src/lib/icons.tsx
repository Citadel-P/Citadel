import { ResourceType } from '@/api/types';
import { Activity, Cable, ChevronsLeftRightEllipsis, createLucideIcon, GitBranch, KeyRound, Layers, Megaphone, Rocket, Rss, Server, Settings, Shield, TriangleAlert, User, UserKey, Users, Webhook } from 'lucide-react';

export const CitadelIcons: Record<Partial<ResourceType>, React.ComponentType<{ className?: string }>> = {
  ['Deployment']: Rocket,
  ['Registry']: Cable,
  ['Platform']: Server,
  ['Stack']: Layers,
  ['Alert']:TriangleAlert, 
  ['AlertRule']: Megaphone,
  ['AlertChannel']: Rss,
  ['GitRepository']: GitBranch,
  ['GitAccount']: KeyRound,
  ['Configuration']: Settings,
  ['Webhook']: Webhook,
  ['Access'] : UserKey,
  ['Team'] : Users,
  ['User']: User,
  ['Role']: Shield,
  ['Variable']: ChevronsLeftRightEllipsis,
  ['Activity']: Activity
};
export const DockerIcon = createLucideIcon('DockerIcon', [
  [
    'path',
    {
      d: 'M3 15c1 4 4 6 9 6 9 0 12-6 11-10h-5v-3h-4v3h-3v-5H7v5H3v4z',
      strokeWidth: '2',
      strokeLinecap: 'round',
      strokeLinejoin: 'round',
    },
  ],
  ['rect', { x: '7', y: '7', width: '3', height: '3', rx: '0.5' }],
  ['rect', { x: '11', y: '7', width: '3', height: '3', rx: '0.5' }],
  ['rect', { x: '15', y: '7', width: '3', height: '3', rx: '0.5' }],
]);

export const GitHubIcon = createLucideIcon('GitHubIcon', [
  [
    'path',
    {
      d: 'M12 2C6.5 2 2 6.5 2 12a10 10 0 0 0 6.8 9.5c.5.1.7-.2.7-.5v-2c-2.8.6-3.4-1.4-3.4-1.4-.5-1.2-1.2-1.6-1.2-1.6-1-.7.1-.7.1-.7 1 .1 1.6 1 1.6 1 .9 1.6 2.6 1.1 3.2.8a2.5 2.5 0 0 1 .7-1.6c-2.2-.3-4.5-1.1-4.5-5A4 4 0 0 1 8 8.1a3.7 3.7 0 0 1 .1-2.8s.8-.3 2.7 1a9.2 9.2 0 0 1 5 0c1.8-1.3 2.7-1 2.7-1a3.7 3.7 0 0 1 .1 2.8A4 4 0 0 1 19 12c0 3.9-2.3 4.7-4.5 5a2.6 2.6 0 0 1 .8 2v3c0 .3.2.6.7.5A10 10 0 0 0 22 12c0-5.5-4.5-10-10-10z',
    },
  ],
]);

export const GitLabIcon = createLucideIcon('GitLabIcon', [['path', { d: 'M12 22l9-7-4-12h-1l-4 9-4-9H7L3 15l9 7z' }]]);
