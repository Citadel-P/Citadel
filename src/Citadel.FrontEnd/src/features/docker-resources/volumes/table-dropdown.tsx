import { Trash, SearchCode } from 'lucide-react';
import { DockerVolumeResult } from '@/api/generated/api.types';
import { createTableDropdown } from '@/components/custom/dropdown-with-dialog';

export const TableDropDown = ({ volume }: { volume: DockerVolumeResult }) => {
  return createTableDropdown<DockerVolumeResult>({
    type: 'Volume',
    actions: ({ navigate, openDialog }) => [
      {
        id: 'inspect',
        label: 'Inspect',
        icon: <SearchCode className="mr-2 h-3 w-3" />,
        onClick: () => navigate(volume?.id ?? ''),
      },
      {
        id: 'delete',
        label: 'Delete',
        icon: <Trash className="mr-2 h-3 w-3 text-danger" />,
        onClick: () => openDialog(volume),
        disabled: volume?.inUse ?? false,
        danger: true,
      },
    ],
  });
};
