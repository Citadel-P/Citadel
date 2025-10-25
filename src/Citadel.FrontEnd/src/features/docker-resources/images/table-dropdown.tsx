import { SearchCode, Trash } from 'lucide-react';
import { ImageView } from '@/api/generated/api.types';
import { formatId } from '@/lib/utils';
import { createTableDropdown } from '@/components/custom/dropdown-with-dialog';

export const TableDropDown = ({ image }: { image: ImageView }) => {
  return createTableDropdown({
    type: 'Image',
    actions: ({ navigate, openDialog }) => [
      {
        id: 'inspect',
        label: 'Inspect',
        icon: <SearchCode className="mr-2 h-3 w-3" />,
        onClick: () => navigate(`${formatId(image.dockerImageId)}`),
      },
      {
        id: 'delete',
        label: 'Delete',
        icon: <Trash className="mr-2 h-3 w-3 text-danger" />,
        onClick: () => openDialog(image),
        danger: true,
      },
    ],
  });
};
