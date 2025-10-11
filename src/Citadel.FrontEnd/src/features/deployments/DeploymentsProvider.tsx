import { useCallback, useEffect, useMemo, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { DeploymentsContext } from './DeploymentContext';

export const DeploymentsProvider: React.FC<{ children?: React.ReactNode }> = ({ children }) => {
  const client = useQueryClient();
  const isLoading = false;

  const contextValue = useMemo(
    () => ({
      isLoading,
    }),
    [isLoading],
  );

  return <DeploymentsContext.Provider value={contextValue}>{children}</DeploymentsContext.Provider>;
};
