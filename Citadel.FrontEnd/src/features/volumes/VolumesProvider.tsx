import { useState, useMemo, useCallback, useEffect, createContext } from 'react';
import { DeleteVolumesInput, DockerVolumeResult } from '@/api/_generated';
import { IDeleteDialogData, useDialogState } from '@/hooks/useDialogState';
import { useDELETEVolumes } from './hooks/useDELETEVolumes';
import { useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';
import { useRequiredContext } from '@/hooks/useRequiredContext';

interface IContext {
  selectedRows: DockerVolumeResult[] | undefined;
  volumes: DockerVolumeResult[];
  setSelectedRows: (Volumes: DockerVolumeResult[] | undefined) => void;
  setVolumes: (Volumes: DockerVolumeResult[]) => void;
  dialogData: IDeleteDialogData<DockerVolumeResult>;
  setDialogData: (data: IDeleteDialogData<DockerVolumeResult>) => void;
  onSearch: (searchTerm: string) => void;
  requestDelete: (request: DeleteVolumesInput) => void;
  deleteIsPending: boolean;
  sheetOpen: boolean;
  setSheetOpen: (open: boolean) => void;
  currentVolume: DockerVolumeResult | undefined;
  setCurrentVolume: (volume: DockerVolumeResult | undefined) => void;
}

export const VolumesContext = createContext<IContext | undefined>(undefined);

const VolumesProvider: React.FC<{ children?: React.ReactNode }> = ({ children }) => {
  const client = useQueryClient();
  // State variables
  const [selectedRows, setSelectedRows] = useState<DockerVolumeResult[] | undefined>();
  const [volumes, setVolumes] = useState<DockerVolumeResult[]>([]);
  const [originalVolumes, setOriginalVolumes] = useState<DockerVolumeResult[]>([]);
  // Sheet state
  const [currentVolume, setCurrentVolume] = useState<DockerVolumeResult>();
  const [sheetOpen, setSheetOpen] = useState(false);

  const { dialogData, setDialogData } = useDialogState<DockerVolumeResult>();
  const [currentSearchTerm, setCurrentSearchTerm] = useState('');

  const { mutate: deleteVolumes, isSuccess: deleteIsSuccess, isPending: deleteIsPending } = useDELETEVolumes();

  // Wrapper function that handles both original and filtered Volumes
  const handleVolumesUpdate = useCallback((Volumes: DockerVolumeResult[]) => {
    setOriginalVolumes(Volumes);
    setVolumes(Volumes);
  }, []);

  // Search function to filter Volumes by name or ID
  const onSearch = useCallback((searchTerm: string) => {
    setCurrentSearchTerm(searchTerm);
  }, []);

  // Filter Volumes whenever search term changes
  useEffect(() => {
    if (!originalVolumes.length) return;

    if (currentSearchTerm.trim() === '') {
      // If no search term, show all volumes
      setVolumes(originalVolumes);
    } else {
      const searchLower = currentSearchTerm.toLowerCase();

      // Filter volumes by id containing the search term
      const filtered = originalVolumes.filter((volume) => {
        const nameMatches = volume.id?.toLowerCase().includes(searchLower) || false;
        const idMatches = (volume.id && volume.id.toLowerCase().includes(searchLower)) || false;

        return nameMatches || idMatches;
      });

      setVolumes(filtered);
    }
  }, [originalVolumes, currentSearchTerm]);

  // Handle successful volume deletion
  useEffect(() => {
    if (deleteIsSuccess) {
      client.invalidateQueries({ queryKey: ['useGETVolumes'] });
      setDialogData({ open: false });
      toast.success('The selected volume(s) has been successfully deleted');
    }
  }, [deleteIsSuccess, client, setDialogData]);

  // Handle volume deletion request
  const requestDelete = useCallback(
    (request: DeleteVolumesInput) => {
      deleteVolumes(request);
    },
    [deleteVolumes],
  );

  // Memoized context value
  const contextValue = useMemo(
    () => ({
      selectedRows,
      setSelectedRows,
      volumes,
      setVolumes: handleVolumesUpdate,
      setDialogData,
      dialogData,
      onSearch,
      requestDelete,
      deleteIsPending,
      sheetOpen,
      setSheetOpen,
      currentVolume,
      setCurrentVolume,
    }),
    [
      selectedRows,
      deleteIsPending,
      setSelectedRows,
      volumes,
      dialogData,
      setDialogData,
      onSearch,
      handleVolumesUpdate,
      requestDelete,
      sheetOpen,
      setSheetOpen,
      currentVolume,
      setCurrentVolume,
    ],
  );

  return <VolumesContext.Provider value={contextValue}>{children}</VolumesContext.Provider>;
};

export default VolumesProvider;
export const useVolumesContext = () => useRequiredContext(VolumesContext);
