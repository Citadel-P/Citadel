import { createContext } from 'react';
import { useRequiredContext } from '@/hooks/useRequiredContext';
import { IDeleteDialogData } from '@/hooks/useDialogState';
import { DeleteImagesRequest, ImageView, RegistryView } from '@/api/_generated';

interface IContext {
  isLoading: boolean;
  registries: RegistryView[];
  setSelectionChange: (name: string) => void;
  selectedRegistry: RegistryView | undefined;
  selectedRows: ImageView[] | undefined;
  localImages: ImageView[];
  requestDelete: (request: DeleteImagesRequest) => void;
  setSelectedRows: (images: ImageView[] | undefined) => void;
  setLocalImages: (images: ImageView[]) => void;
  deleteIsPending: boolean;
  dialogData: IDeleteDialogData<ImageView>;
  setDialogData: (data: IDeleteDialogData<ImageView>) => void;
  onSearch: (searchTerm: string) => void;
  currentImage: ImageView | undefined;
  setCurrentImage: (image: ImageView | undefined) => void;
  sheetOpen: boolean;
  setSheetOpen: (open: boolean) => void;
}

export const ImagesContext = createContext<IContext | undefined>(undefined);
ImagesContext.displayName = 'ImagesContext';

export const useImagesContext = () => useRequiredContext(ImagesContext);
