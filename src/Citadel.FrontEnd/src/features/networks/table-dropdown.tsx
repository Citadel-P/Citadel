import { Trash, SearchCode } from 'lucide-react';
import { useNetworksContext } from './context';
import { DockerNetworkResult } from '@/api/generated/api.types';
import { formatId } from '@/lib/utils';
import { createTableDropdown } from '@/components/custom/dropdown-menu';

export const TableDropDown = ({ network }: { network: DockerNetworkResult }) => {
  const context = useNetworksContext();

  return createTableDropdown({
    resource: network,
    context,
    actions: ({ navigate, openDialog }) => [
      {
        id: 'inspect',
        label: 'Inspect',
        icon: <SearchCode className="mr-2 h-3 w-3" />,
        onClick: () => navigate(`${formatId(network.id)}`),
      },
      {
        id: 'delete',
        label: 'Delete',
        icon: <Trash className="mr-2 h-3 w-3 text-danger" />,
        onClick: openDialog,
        disabled: network.inUse ?? false,
        danger: true,
      },
    ],
  });
};
