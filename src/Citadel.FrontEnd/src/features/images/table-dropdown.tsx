import { SearchCode, Trash } from 'lucide-react';
import { ImageView } from '@/api/generated/api.types';
import { useImagesContext } from './ImagesContext';
import { useCallback } from 'react';
import { useNavigate } from 'react-router';
import { formatId } from '@/lib/utils';
import { DropdownAction, DropdownActions } from '@/components/custom/dropdown-menu';

export const TableDropDown = ({ image }: { image: ImageView }) => {
  const navigate = useNavigate();
  const { setDialogData } = useImagesContext();

  const openDialog = useCallback(() => {
    setDialogData({ open: true, currentSelection: [image] });
  }, [setDialogData, image]);
  const items: DropdownAction[] = [
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
  ];

  return <DropdownActions items={items} />;
};
