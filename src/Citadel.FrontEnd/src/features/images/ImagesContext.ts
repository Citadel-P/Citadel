import { createContext } from 'react';
import { useRequiredContext } from '@/hooks/useRequiredContext';
import { DeleteImagesRequest, ImageView, RegistryView } from '@/api/_generated';
import { IDialogData } from '@/hooks/useDialogState';

interface IContext {
  isLoading: boolean;
  registries: RegistryView[];
  setSelectionChange: (name: string) => void;
  selectedRegistry: RegistryView | undefined;
  selectedRows: ImageView[] | undefined;
  localImages: ImageView[];
  requestDelete: (request: DeleteImagesRequest) => void;
  setSelectedRows: (images: ImageView[] | undefined) => void;
  deleteIsPending: boolean;
  dialogData: IDialogData<ImageView>;
  setDialogData: (data: IDialogData<ImageView>) => void;
  onSearch: (searchTerm: string) => void;
  currentImage: ImageView | undefined;
  setCurrentImage: (image: ImageView | undefined) => void;
  runDialogData: IDialogData<ImageView>;
  setRunDialogData: (data: IDialogData<ImageView>) => void;
}

export const ImagesContext = createContext<IContext | undefined>(undefined);
ImagesContext.displayName = 'ImagesContext';

export const useImagesContext = () => useRequiredContext(ImagesContext);
