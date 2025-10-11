import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuTrigger,
  DropdownMenuSeparator,
} from '@/components/ui/dropdown-menu';
import { Button } from '@/components/ui/button';
import { MoreHorizontal, SearchCode, Trash } from 'lucide-react';
import { ImageView } from '@/api/generated/api.types';
import { useImagesContext } from './ImagesContext';
import { useCallback } from 'react';
import { ActionMenuItem } from '@/components/ui/ActionMenuItem';
import { useNavigate } from 'react-router';
import { formatId } from '@/lib/utils';

const DropdownTableMenu = ({ image }: { image: ImageView }) => {
  const navigate = useNavigate();
  const { setDialogData } = useImagesContext();

  // Memoized function to open the delete dialog
  const openDialog = useCallback(() => {
    setDialogData({ open: true, currentSelection: [image] });
  }, [setDialogData, image]);

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" className="h-8 w-8 p-0">
          <span className="sr-only">Open menu</span>
          <MoreHorizontal className="h-2 w-2" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-38 drop-shadow-md shadow-custom bg-background pt-2 pb-2">
        <ActionMenuItem
          onClick={() => navigate(`${formatId(image.imageId)}`)}
          icon={<SearchCode className="mr-2 h-3 w-3" />}
          label="Inspect"
        />
        <DropdownMenuSeparator />
        <ActionMenuItem
          onClick={openDialog}
          icon={<Trash className="mr-2 h-3 w-3 text-danger" />}
          label="Delete"
          className="text-danger hover:text-danger!"
        />
      </DropdownMenuContent>
    </DropdownMenu>
  );
};

export default DropdownTableMenu;
