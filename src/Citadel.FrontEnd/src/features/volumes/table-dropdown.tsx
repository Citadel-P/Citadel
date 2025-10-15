import { Trash, SearchCode } from 'lucide-react';
import { useVolumesContext } from './VolumesContext';
import { DockerVolumeResult } from '@/api/generated/api.types';
import { createTableDropdown } from '@/components/custom/dropdown-menu';

export const TableDropDown = ({ volume }: { volume: DockerVolumeResult }) => {
  const context = useVolumesContext();

  return createTableDropdown({
    resource: volume,
    context,
    actions: ({ navigate, openDialog }) => [
      {
        id: 'inspect',
        label: 'Inspect',
        icon: <SearchCode className="mr-2 h-3 w-3" />,
        onClick: () => navigate(volume.id),
      },
      {
        id: 'delete',
        label: 'Delete',
        icon: <Trash className="mr-2 h-3 w-3 text-danger" />,
        onClick: openDialog,
        disabled: volume.inUse ?? false,
        danger: true,
      },
    ],
  });
};
