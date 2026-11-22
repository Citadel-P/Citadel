import type { ComponentType } from 'react';
import { Copy, Layers, Wrench } from 'lucide-react';
import { CitadelIcons } from '@/lib/icons';

const resourceIcons: Record<string, ComponentType<{ className?: string }>> = {
  Alert: CitadelIcons.Alert,
  Build: CitadelIcons.Build,
  Instance: CitadelIcons.Instance,
  'Backup policy': CitadelIcons.BackupPolicy,
  Backup: CitadelIcons.BackupPolicy,
  Script: CitadelIcons.Script,
  'Git repository': CitadelIcons.GitRepository,
  Deployment: CitadelIcons.Deployment,
  'Swarm service': CitadelIcons.SwarmService,
  Stack: CitadelIcons.Stack,
  Image: CitadelIcons.Image,
  'Resource copy': Copy,
  Volume: CitadelIcons.Volume,
  Container: CitadelIcons.Container,
  Updates: CitadelIcons.Updates,
  Platform: CitadelIcons.Platform,
  Maintenance: Wrench,
  Core: CitadelIcons.Instance,
};

export function instanceResourceOption(name: string) {
  const Icon = resourceIcons[name] ?? Layers;
  return { value: name, label: name, icon: <Icon className="size-3.5" /> };
}
