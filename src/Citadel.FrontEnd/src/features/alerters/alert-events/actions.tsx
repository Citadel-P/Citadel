import { Check, CheckCheck } from 'lucide-react';
import { AlertEventStatus, AlertEventView } from '@/api/generated/api.types';
import { ActionConfig, createActionsBuilder } from '@/components/custom/actions-builder';

type AlertEventListItem = AlertEventView & { name: string };

const useIds = (resources: AlertEventListItem | AlertEventListItem[]) =>
  Array.isArray(resources) ? resources.map((resource) => resource.id) : [resources.id];

const acknowledgeAction: ActionConfig<AlertEventListItem, 'acknowledgeAlertEvents'> = {
  key: 'acknowledge',
  type: 'command',
  icon: Check,
  mutateKey: 'acknowledgeAlertEvents',
  useVariables: (resources) => ({ ids: useIds(resources) }),
  canExecute: (resources) => {
    const can = (resource: AlertEventListItem) => resource.status === AlertEventStatus.Active;
    return Array.isArray(resources) ? resources.every(can) : can(resources);
  },
};

const resolveAction: ActionConfig<AlertEventListItem, 'resolveAlertEvents'> = {
  key: 'resolve',
  type: 'command',
  icon: CheckCheck,
  mutateKey: 'resolveAlertEvents',
  useVariables: (resources) => ({ ids: useIds(resources), resolutionNote: null }),
  canExecute: (resources) => {
    const can = (resource: AlertEventListItem) => resource.status !== AlertEventStatus.Resolved;
    return Array.isArray(resources) ? resources.every(can) : can(resources);
  },
};

export const { dropdown: AlertEventDropdownActions, group: AlertEventGroupActions } =
  createActionsBuilder<AlertEventListItem>().addAction(acknowledgeAction).addAction(resolveAction).build();
