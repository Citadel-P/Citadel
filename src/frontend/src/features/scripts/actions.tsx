import { AuthorizedScript, ResourceControlState } from '@/api/generated/api.types';
import { createActionsBuilder } from '@/components/custom/actions-builder';
import { useSelectedResources, useTaskSheet } from '@/lib/atoms';
import { useMutate, useRead } from '@/lib/hooks';
import { useQueryClient } from '@tanstack/react-query';
import { Copy, Pencil, Play, Trash2 } from 'lucide-react';
import { useLocation, useNavigate } from 'react-router';
import { toast } from 'sonner';

export const invalidateScriptQueries = async (queryClient: ReturnType<typeof useQueryClient>, id?: string) => {
  await queryClient.invalidateQueries({ queryKey: ['listScripts'] });
  if (id) {
    await queryClient.invalidateQueries({ queryKey: ['getScript', { id }] });
    await queryClient.invalidateQueries({ queryKey: ['listScriptRuns', { id }] });
  } else {
    await queryClient.invalidateQueries({ queryKey: ['getScript'] });
    await queryClient.invalidateQueries({ queryKey: ['listScriptRuns'] });
  }
};

const singleSelection = (resources: AuthorizedScript | AuthorizedScript[]) => {
  const selected = Array.isArray(resources) ? resources[0] : resources;
  const multiSelect = Array.isArray(resources) && resources.length > 1;
  return { selected, multiSelect };
};

const useScriptRunTaskSheet = () => useTaskSheet('Script');

const { dropdown, group, info } = createActionsBuilder<AuthorizedScript>()
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
      const queryClient = useQueryClient();
      const remove = useMutate('deleteScript');
      const [, setSelectedResources] = useSelectedResources<AuthorizedScript>('Script');
      const location = useLocation();
      const navigate = useNavigate();
      const isCurrentResource = selected.some(
        (script) => location.pathname.replace(/\/$/, '') === `/scripts/edit/${script.id}`,
      );

      return {
        canExecute: selected.length > 0,
        isPending: remove.isPending,
        run: async () => {
          if (selected.length === 0) return;

          try {
            await Promise.all(selected.map((script) => remove.mutateAsync({ id: script.id } as any)));
            if (isCurrentResource) navigate('/scripts', { replace: true });
            await queryClient.invalidateQueries({ queryKey: ['listScripts'] });
            setSelectedResources([]);
            toast.success(`${selected.length} ${selected.length === 1 ? 'script' : 'scripts'} deleted`);
          } catch (error) {
            toast.error(remove.validationErrors ?? 'Failed to delete selected scripts');
            throw error;
          }
        },
      };
    },
  })
  .build();

export const ScriptDropdownActions = dropdown;
export const ScriptGroupActions = group;
export const ScriptInfoActions = info;
