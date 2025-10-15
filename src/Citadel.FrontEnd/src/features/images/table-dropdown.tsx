import { SearchCode, Trash } from 'lucide-react';
import { ImageView } from '@/api/generated/api.types';
import { useImagesContext } from './ImagesContext';
import { formatId } from '@/lib/utils';
import { createTableDropdown } from '@/components/custom/dropdown-menu';

export const TableDropDown = ({ image }: { image: ImageView }) => {
  const context = useImagesContext();

  return createTableDropdown({
    resource: image,
    context,
    actions: ({ navigate, openDialog }) => [
      {
        id: 'inspect',
        label: 'Inspect',
        icon: <SearchCode className="mr-2 h-3 w-3" />,
        onClick: () => navigate(`${formatId(image.imageId)}`),
      },
      {
        id: 'delete',
        label: 'Delete',
        icon: <Trash className="mr-2 h-3 w-3 text-danger" />,
        onClick: openDialog,
        danger: true,
      },
    ],
  });
};
