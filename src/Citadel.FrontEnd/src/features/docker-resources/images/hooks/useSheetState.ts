import { DockerHubImageResult, DockerHubTagView, GitHubCrPackageVersion } from '@/api/generated/api.types';
import { useState } from 'react';

type AllowedTypes = GitHubCrPackageVersion | DockerHubTagView | DockerHubImageResult;

// Hook for managing sheet state
export function useSheetState<T extends AllowedTypes>() {
  const [sheetState, setSheetState] = useState<{
    isOpen: boolean;
    image: T | null;
  }>({ isOpen: false, image: null });

  const openSheet = (image: T) => {
    setSheetState({ isOpen: true, image });
  };

  const closeSheet = () => {
    setSheetState({ isOpen: false, image: null });
  };

  return { sheetState, openSheet, closeSheet };
}
