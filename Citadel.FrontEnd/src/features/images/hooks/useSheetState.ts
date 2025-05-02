import { DockerHubTagView, GhcrPackageVersion } from '@/api/_generated';
import { useState } from 'react';

type AllowedTypes = GhcrPackageVersion | DockerHubTagView;

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
