import { SwarmConfigView } from '@/api/generated/api.types';
import { createActionsBuilder } from '@/components/custom/actions-builder';
import { useAppContext } from '@/lib/context/app-context';
import { ButtonActionComponent, DropdownActionComponent } from '@/pages/types';
import { Trash } from 'lucide-react';
import { SwarmResourceEditDropdownAction, SwarmResourceEditInfoAction } from '../resource-edit-dialog';

export const canDeleteConfig = (config: SwarmConfigView) => !config.inUse && !config.isStale;

const configActions = createActionsBuilder<SwarmConfigView>()
  .addAction({
    key: 'delete',
    type: 'command',
    icon: Trash,
    mutateKey: 'deleteSwarmConfigs',
    canExecute: (resources) => {
      return Array.isArray(resources) ? resources.every(canDeleteConfig) : canDeleteConfig(resources);
    },
    requiredCapabilities: ['canWrite'],
    separatorBefore: true,
    confirm: true,
    destructive: true,
    resourceType: 'Config',
    argName: 'variables',
    useVariables: (resources) => {
      const { currentPlatform } = useAppContext();
      const selected = Array.isArray(resources) ? resources : [resources];
      return {
        platformId: currentPlatform?.id ?? '',
        data: { ids: selected.map((config) => config.id) },
      };
    },
  })
  .build();

const ConfigEditDropdownAction: DropdownActionComponent<SwarmConfigView> = (props) => (
  <SwarmResourceEditDropdownAction {...props} />
);
const ConfigEditInfoAction: ButtonActionComponent<SwarmConfigView> = ({ resource }) => (
  <SwarmResourceEditInfoAction resource={resource} kind="config" />
);

export const ConfigDropdownActions = { edit: ConfigEditDropdownAction, ...configActions.dropdown };
export const ConfigGroupActions = configActions.group;
export const ConfigInfoActions = { edit: ConfigEditInfoAction, ...configActions.info };
