import { useRequiredContext } from '@/hooks/useRequiredContext';
import { createContext } from 'react';
import { DeleteVolumesInput, DockerVolumeResult } from '@/api/generated/api.types';
import { IDialogData } from '@/lib/hooks';

interface IContext {
  selectedRows: DockerVolumeResult[] | undefined;
  volumes: DockerVolumeResult[] | undefined;
  setSelectedRows: (Volumes: DockerVolumeResult[] | undefined) => void;
  setVolumes: (Volumes: DockerVolumeResult[]) => void;
  dialogData: IDialogData<DockerVolumeResult>;
  setDialogData: (data: IDialogData<DockerVolumeResult>) => void;
  onSearch: (searchTerm: string) => void;
  requestDelete: (request: DeleteVolumesInput) => void;
  deleteIsPending: boolean;
  currentVolume: DockerVolumeResult | undefined;
  setCurrentVolume: (volume: DockerVolumeResult | undefined) => void;
}

export const VolumesContext = createContext<IContext | undefined>(undefined);
VolumesContext.displayName = 'VolumesContext';

export const useVolumesContext = () => useRequiredContext(VolumesContext);
