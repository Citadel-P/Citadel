import { SwarmSecretView } from '@/api/generated/api.types';
import { createActionsBuilder } from '@/components/custom/actions-builder';
import { useAppContext } from '@/lib/context/app-context';
import { ButtonActionComponent, DropdownActionComponent } from '@/pages/types';
import { Trash } from 'lucide-react';
import { SwarmResourceEditDropdownAction, SwarmResourceEditInfoAction } from '../resource-edit-dialog';

export const canDeleteSecret = (secret: SwarmSecretView) => !secret.inUse && !secret.isStale;

const secretActions = createActionsBuilder<SwarmSecretView>()
  .addAction({
    key: 'delete',
    type: 'command',
    icon: Trash,
    mutateKey: 'deleteSwarmSecrets',
    canExecute: (resources) => {
      return Array.isArray(resources) ? resources.every(canDeleteSecret) : canDeleteSecret(resources);
    },
    requiredCapabilities: ['canWrite'],
    separatorBefore: true,
    confirm: true,
    destructive: true,
    resourceType: 'Secret',
    argName: 'variables',
    useVariables: (resources) => {
      const { currentPlatform } = useAppContext();
      const selected = Array.isArray(resources) ? resources : [resources];
      return {
        platformId: currentPlatform?.id ?? '',
        data: { ids: selected.map((secret) => secret.id) },
      };
    },
  })
  .build();

const SecretEditDropdownAction: DropdownActionComponent<SwarmSecretView> = (props) => (
  <SwarmResourceEditDropdownAction {...props} />
);
const SecretEditInfoAction: ButtonActionComponent<SwarmSecretView> = ({ resource }) => (
  <SwarmResourceEditInfoAction resource={resource} kind="secret" />
);

export const SecretDropdownActions = { edit: SecretEditDropdownAction, ...secretActions.dropdown };
export const SecretGroupActions = secretActions.group;
export const SecretInfoActions = { edit: SecretEditInfoAction, ...secretActions.info };
