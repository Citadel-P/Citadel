import { useEffect, useState, useCallback } from 'react';
import { HubConnection } from '@microsoft/signalr';
import { GitRepositoryView } from '@/api/generated/api.types';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';
import { useRead } from '@/lib/hooks';

export const useGitRepoGroup = (id: string | undefined) => {
  const { data, isLoading } = useRead('getGitRepository', { id });
  const [gitRepo, setGitRepo] = useState<GitRepositoryView | undefined>();

  useEffect(() => {
    if (!data) return;
    setGitRepo(data.data);
  }, [data]);
  const handleDeploymentInfoUpdated = useCallback((deployment: GitRepositoryView) => {
    setGitRepo((prev) => {
      if (!prev) return prev;
      return {
        ...prev,
        name: deployment.name,
        status: deployment.status,
        description: deployment.description,
        url: deployment.url,
        defaultBranch: deployment.defaultBranch,
        webHookEnabled: deployment.webHookEnabled,
        webHookSecret: deployment.webHookSecret,
      };
    });
  }, []);

  const setupEventListeners = useCallback(
    (hubConnection: HubConnection) => {
      hubConnection.on('GitRepositoryInfoUpdated', handleDeploymentInfoUpdated);
    },
    [handleDeploymentInfoUpdated],
  );

  const removeEventListeners = useCallback(
    (hubConnection: HubConnection) => {
      hubConnection.off('GitRepositoryInfoUpdated', handleDeploymentInfoUpdated);
    },
    [handleDeploymentInfoUpdated],
  );

  useSignalRGroup({
    groupName: 'git-repo:' + id,
    skip: !id,
    setupEventListeners,
    removeEventListeners,
  });

  return { gitRepo, isLoading };
};
