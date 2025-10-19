import { Trash, SearchCode } from 'lucide-react';
import { DockerNetworkResult } from '@/api/generated/api.types';
import { formatId } from '@/lib/utils';
import { createTableDropdown } from '@/components/custom/dropdown-menu';

export const TableDropDown = ({ network }: { network: DockerNetworkResult }) => {
  return createTableDropdown({
    type: 'Network',
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
        onClick: () => openDialog(network),
        disabled: network.inUse ?? false,
        danger: true,
      },
    ],
  });
};
