import { TagView } from '@/api/generated/api.types';
import { createActionsBuilder } from '@/components/custom/actions-builder';
import { useSelectedResources } from '@/lib/atoms';
import { useMutate } from '@/lib/hooks';
import { useQueryClient } from '@tanstack/react-query';
import { Pencil, Trash } from 'lucide-react';
import { toast } from 'sonner';

export type TagPageAction = { type: 'edit-tag'; tag: TagView };

const tagActions = new EventTarget();

export const dispatchTagPageAction = (action: TagPageAction) => {
  tagActions.dispatchEvent(new CustomEvent<TagPageAction>('tags-action', { detail: action }));
};

export const subscribeTagPageActions = (listener: (action: TagPageAction) => void) => {
  const wrapped = (event: Event) => listener((event as CustomEvent<TagPageAction>).detail);
  tagActions.addEventListener('tags-action', wrapped);
  return () => tagActions.removeEventListener('tags-action', wrapped);
};

export const invalidateTagQueries = async (queryClient: ReturnType<typeof useQueryClient>) => {
  await queryClient.invalidateQueries({ queryKey: ['listTags'] });
};

const { dropdown, group } = createActionsBuilder<TagView>()
  .addAction({
    key: 'edit',
    type: 'command',
    icon: Pencil,
    requiredCapabilities: ['canWrite'],
    useHandler: ({ resources }) => {
      const selected = Array.isArray(resources) ? resources[0] : resources;
      const multiSelect = Array.isArray(resources) && resources.length > 1;

      return {
        canExecute: !!selected && !multiSelect,
        run: () => {
          if (!selected || multiSelect) return;
          dispatchTagPageAction({ type: 'edit-tag', tag: selected });
        },
      };
    },
  })
  .addAction({
    key: 'delete',
    type: 'command',
    icon: Trash,
    confirm: true,
    destructive: true,
    resourceType: 'Tag',
    requiredCapabilities: ['canWrite'],
    useHandler: ({ resources }) => {
      const selected = Array.isArray(resources) ? resources : [resources];
      const queryClient = useQueryClient();
      const remove = useMutate('deleteTag');
      const [, setSelectedResources] = useSelectedResources<TagView>('Tag');

      return {
        canExecute: selected.length > 0,
        isPending: remove.isPending,
        run: async () => {
          if (selected.length === 0) return;

          try {
            await Promise.all(selected.map((tag) => remove.mutateAsync({ id: tag.id } as any)));
            await invalidateTagQueries(queryClient);
            setSelectedResources([]);
            toast.success(`${selected.length} ${selected.length === 1 ? 'tag' : 'tags'} deleted`);
          } catch (error) {
            toast.error(remove.validationErrors ?? 'Failed to delete selected tags');
            throw error;
          }
        },
      };
    },
  })
  .build();

export const TagDropdownActions = dropdown;
export const TagGroupActions = group;
