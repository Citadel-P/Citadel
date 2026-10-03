import { OidcProviderView } from '@/api/generated/api.types';
import { createActionsBuilder } from '@/components/custom/actions-builder';
import { useSelectedResources } from '@/lib/atoms';
import { useMutate } from '@/lib/hooks';
import { useQueryClient } from '@tanstack/react-query';
import { Pencil, RefreshCw, Trash2 } from 'lucide-react';
import { useNavigate } from 'react-router';
import { toast } from 'sonner';

export const invalidateOidcProviderQueries = async (queryClient: ReturnType<typeof useQueryClient>) => {
  await queryClient.invalidateQueries({ queryKey: ['listOidcProviders'] });
  await queryClient.invalidateQueries({ queryKey: ['listOidcLoginProviders'] });
};

const { dropdown, group, info } = createActionsBuilder<OidcProviderView>()
  .addAction({
    key: 'edit',
    type: 'command',
    icon: Pencil,
    requiredCapabilities: ['canWrite'],
    useHandler: ({ resources }) => {
      const selected = Array.isArray(resources) ? resources[0] : resources;
      const multiSelect = Array.isArray(resources) && resources.length > 1;
      const navigate = useNavigate();

      return {
        canExecute: !!selected && !multiSelect,
        run: () => {
          if (!selected || multiSelect) return;
          navigate(`/oidc-providers/edit/${selected.id}`);
        },
      };
    },
  })
  .addAction({
    key: 'test',
    type: 'command',
    icon: RefreshCw,
    requiredCapabilities: ['canRead'],
    useHandler: ({ resources }) => {
      const selected = Array.isArray(resources) ? resources[0] : resources;
      const multiSelect = Array.isArray(resources) && resources.length > 1;
      const testDiscovery = useMutate('testOidcProviderDiscovery');

      return {
        canExecute: !!selected && !multiSelect,
        isPending: testDiscovery.isPending,
        run: async () => {
          if (!selected || multiSelect) return;

          try {
            const result = await testDiscovery.mutateAsync({ id: selected.id } as any);
            toast.success(`Discovery OK: ${result.data.issuer}`);
          } catch {
            toast.error(testDiscovery.validationErrors ?? 'Discovery failed');
          }
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
    resourceType: 'OidcProvider',
    requiredCapabilities: ['canWrite'],
    useHandler: ({ resources }) => {
      const selected = Array.isArray(resources) ? resources : [resources];
      const queryClient = useQueryClient();
      const remove = useMutate('deleteOidcProvider');
      const [, setSelectedResources] = useSelectedResources<OidcProviderView>('OidcProvider');

      return {
        canExecute: selected.length > 0,
        isPending: remove.isPending,
        run: async () => {
          if (selected.length === 0) return;

          try {
            await Promise.all(selected.map((provider) => remove.mutateAsync({ id: provider.id } as any)));
            await invalidateOidcProviderQueries(queryClient);
            setSelectedResources([]);
            toast.success(`${selected.length} ${selected.length === 1 ? 'provider' : 'providers'} deleted`);
          } catch (error) {
            toast.error(remove.validationErrors ?? 'Failed to delete selected providers');
            throw error;
          }
        },
      };
    },
  })
  .build();

export const OidcProviderDropdownActions = dropdown;
export const OidcProviderGroupActions = group;
export const OidcProviderInfoActions = info;
