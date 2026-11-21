import { useRemoveResourceAction } from '@/components/custom/useSettledResourceAction';
import { ScriptSummaryView, ResourceControlState } from '@/api/generated/api.types';
import { createActionsBuilder } from '@/components/custom/actions-builder';
import { useTaskSheet } from '@/lib/atoms';
import { useMutate, useRead } from '@/lib/hooks';
import { Copy, Pencil, Play, Trash2 } from 'lucide-react';
import { useNavigate } from 'react-router';

const singleSelection = (resources: ScriptSummaryView | ScriptSummaryView[]) => {
  const selected = Array.isArray(resources) ? resources[0] : resources;
  const multiSelect = Array.isArray(resources) && resources.length > 1;
  return { selected, multiSelect };
};

const useScriptRunTaskSheet = () => useTaskSheet('Script');

const { dropdown, group, info } = createActionsBuilder<ScriptSummaryView>()
  .addAction({
    key: 'edit',
    type: 'command',
    icon: Pencil,
    requiredCapabilities: ['canWrite'],
    useHandler: ({ resources }) => {
      const { selected, multiSelect } = singleSelection(resources);
      const navigate = useNavigate();

      return {
        canExecute: !!selected && !multiSelect,
        run: () => {
          if (!selected || multiSelect) return;
          navigate(`/scripts/edit/${selected.id}`);
        },
      };
    },
  })
  .addAction({
    key: 'duplicate',
    title: 'Duplicate configuration',
    type: 'command',
    icon: Copy,
    requiredCapabilities: ['canRead'],
    useHandler: ({ resources }) => {
      const { selected, multiSelect } = singleSelection(resources);
      const navigate = useNavigate();
      const { data } = useRead('listScripts');
      const canCreate = data?.data.capabilities.canWrite === true;
      return {
        canExecute: !!selected && !multiSelect && canCreate,
        disabledReason: !canCreate ? 'Create permission is required to duplicate this resource.' : undefined,
        run: () => {
          if (selected && !multiSelect && canCreate) navigate(`/scripts/add?duplicateFrom=${selected.id}`);
        },
      };
    },
  })
  .addAction({
    key: 'run',
    type: 'command',
    icon: Play,
    requiredCapabilities: ['canExecute'],
    useHandler: ({ resources }) => {
      const { selected, multiSelect } = singleSelection(resources);
      const { open: openSheet } = useScriptRunTaskSheet();
      const blocked = selected?.controlState === ResourceControlState.Processing;

      return {
        canExecute: !!selected && !multiSelect && selected.enabled && !blocked,
        run: () => {
          if (!selected || multiSelect) return;
          openSheet({
            kind: 'scriptRun',
            payload: {
              id: selected.id,
              name: selected.name,
              mode: 'run',
              argsJson: null,
              timeoutSeconds: null,
            },
          });
        },
      };
    },
  })
  .addAction({
    key: 'delete',
    type: 'command',
    icon: Trash2,
    confirm: true,
    destructive: true,
    resourceType: 'Script',
    requiredCapabilities: ['canWrite'],
    useHandler: ({ resources }) => {
      const selected = Array.isArray(resources) ? resources : [resources];
      const remove = useMutate('deleteScript');
      return useRemoveResourceAction(selected, {
        type: 'Script',
        listPath: '/scripts',
        mutate: (item) => remove.mutateAsync({ id: item.id }),
        message: (count, total) => `${count} of ${total} ${total === 1 ? 'script' : 'scripts'} deleted`,
      });
    },
  })
  .build();

export const ScriptDropdownActions = dropdown;
export const ScriptGroupActions = group;
export const ScriptInfoActions = info;
