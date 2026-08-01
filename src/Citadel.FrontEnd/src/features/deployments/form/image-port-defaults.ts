export const shouldApplyLocalImagePortDefaults = (
  requestedImageId: string | null,
  currentImageId: string | undefined,
  currentPorts: string[] | null | undefined,
): boolean => !!requestedImageId && requestedImageId === currentImageId && (currentPorts?.length ?? 0) === 0;

export const portsAfterImageSourceChange = (
  isAdoption: boolean,
  currentPorts: string[] | null | undefined,
): string[] => (isAdoption ? (currentPorts ?? []) : []);
