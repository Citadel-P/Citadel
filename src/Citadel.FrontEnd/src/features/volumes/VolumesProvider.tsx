import { useState, useMemo, useCallback, useEffect } from 'react';
import { DockerVolumeResult } from '@/api/generated/api.types';
import { VolumesContext } from './VolumesContext';
import { useDeleteDialog } from '@/lib/hooks';

export const VolumesProvider: React.FC<{ children?: React.ReactNode }> = ({ children }) => {
  const [selectedRows, setSelectedRows] = useState<DockerVolumeResult[] | undefined>();
  const [volumes, setVolumes] = useState<DockerVolumeResult[] | undefined>([]);
  const [originalVolumes, setOriginalVolumes] = useState<DockerVolumeResult[] | undefined>([]);
  const { setDialogData, dialogData, requestDelete, deleteIsPending } = useDeleteDialog<DockerVolumeResult>({
    type: 'Volume',
  });
  const [currentVolume, setCurrentVolume] = useState<DockerVolumeResult>();
  const [currentSearchTerm, setCurrentSearchTerm] = useState('');

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
    if (!originalVolumes?.length) return;

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
      currentVolume,
      setCurrentVolume,
    ],
  );

  return <VolumesContext.Provider value={contextValue}>{children}</VolumesContext.Provider>;
};
