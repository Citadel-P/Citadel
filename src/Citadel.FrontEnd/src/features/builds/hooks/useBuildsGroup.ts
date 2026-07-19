import { BuildProjectView, ResourceCapabilities } from '@/api/generated/api.types';
import { useResourceTagFilter } from '@/features/tags/components';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';
import { useRead } from '@/lib/hooks';
import { HubConnection } from '@microsoft/signalr';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { selectBuildProjectLatestRun } from '../build-run-state';

export const useBuildsGroup = () => {
  const { selectedTagNames } = useResourceTagFilter();
  const readArgs = useMemo(
    () => (selectedTagNames.length > 0 ? { query: { tags: selectedTagNames } } : undefined),
    [selectedTagNames],
  );
  const { data, isLoading } = useRead('listBuildProjects', readArgs);
  const [projects, setProjects] = useState<BuildProjectView[] | undefined>();
  const [capabilities, setCapabilities] = useState<ResourceCapabilities | undefined>();
  const lastFetchedRef = useRef<BuildProjectView[]>([]);

  useEffect(() => {
    if (!data) return;

    const next = data.data.projects;
    if (next !== lastFetchedRef.current) {
      lastFetchedRef.current = next;
      setProjects(next);
      setCapabilities(data.data.capabilities);
    }
  }, [data]);

  const matchesActiveFilters = useCallback(
    (project: BuildProjectView) => {
      if (selectedTagNames.length === 0) return true;

      const tagNames = new Set((project.tags ?? []).map((tag) => tag.name.trim().toLowerCase()));
      return selectedTagNames.every((tagName) => tagNames.has(tagName.trim().toLowerCase()));
    },
    [selectedTagNames],
  );

  const handleBuildProjectInfoUpdated = useCallback(
    (project: BuildProjectView, action: string) => {
      setProjects((prev) => {
        if (!prev) return prev;
        if (action === 'create') {
          return matchesActiveFilters(project) ? [...prev, project] : prev;
        }
        if (action === 'delete') {
          return prev.filter((item) => item.id !== project.id);
        }

        const index = prev.findIndex((item) => item.id === project.id);
        if (!matchesActiveFilters(project)) {
          return index === -1 ? prev : prev.filter((item) => item.id !== project.id);
        }

        if (index === -1) return [...prev, project];

        const updated = [...prev];
        updated[index] = {
          ...project,
          latestRun: selectBuildProjectLatestRun(project, updated[index].latestRun),
        };
        return updated;
      });
    },
    [matchesActiveFilters],
  );

  const setupEventListeners = useCallback(
    (hubConnection: HubConnection) => {
      hubConnection.on('BuildProjectInfoUpdated', handleBuildProjectInfoUpdated);
    },
    [handleBuildProjectInfoUpdated],
  );

  const removeEventListeners = useCallback(
    (hubConnection: HubConnection) => {
      hubConnection.off('BuildProjectInfoUpdated', handleBuildProjectInfoUpdated);
    },
    [handleBuildProjectInfoUpdated],
  );

  useSignalRGroup({
    groupName: 'build-projects',
    setupEventListeners,
    removeEventListeners,
  });

  return {
    projects: projects ?? [],
    isLoading,
    capabilities,
    selectedTagNames,
  };
};
