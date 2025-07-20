import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuTrigger,
  DropdownMenuSeparator,
} from '@/components/ui/dropdown-menu';
import { Button } from '@/components/ui/button';
import { MoreHorizontal, SearchCode, Trash } from 'lucide-react';
import { ImageView } from '@/api/_generated';
import { useImagesContext } from './ImagesProvider';
import { useCallback } from 'react';
import { ActionMenuItem } from '@/components/ui/ActionMenuItem';

const DropdownTableMenu = ({ image }: { image: ImageView }) => {
  const { setDialogData, setSheetOpen, setCurrentImage } = useImagesContext();

  // Memoized function to open the delete dialog
  const openDialog = useCallback(() => {
    setDialogData({ open: true, currentSelection: [image] });
  }, [setDialogData, image]);

  const openSheet = useCallback(() => {
    setSheetOpen(true);
    setCurrentImage(image);
  }, [setSheetOpen, setCurrentImage, image]);

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" className="h-8 w-8 p-0">
          <span className="sr-only">Open menu</span>
          <MoreHorizontal className="h-2 w-2" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-38 drop-shadow-md shadow-custom bg-background pt-2 pb-2">
        <ActionMenuItem onClick={openSheet} icon={<SearchCode className="mr-2 h-3 w-3" />} label="Inspect" />
        <DropdownMenuSeparator />
        <ActionMenuItem
          onClick={openDialog}
          icon={<Trash className="mr-2 h-3 w-3 text-danger" />}
          label="Delete"
          className="text-danger"
        />
      </DropdownMenuContent>
    </DropdownMenu>
  );
};

export default DropdownTableMenu;
