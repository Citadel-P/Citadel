import { AutomationActionView, ResourceCapabilities } from '@/api/generated/api.types';
import { useRead } from '@/lib/hooks';
import { useEffect, useRef, useState } from 'react';
import { useResourceTagFilter } from '@/features/tags/components';

const AUTOMATION_CAPABILITIES: ResourceCapabilities = { canRead: true, canWrite: true, canExecute: true };

export const useAutomationActionsGroup = () => {
  const { selectedTagNames } = useResourceTagFilter();
  const { data, isLoading } = useRead(
    'listAutomationActions',
    selectedTagNames.length > 0 ? { query: { tags: selectedTagNames } } : undefined,
  );
  const [actions, setActions] = useState<AutomationActionView[] | undefined>();
  const lastFetchedRef = useRef<AutomationActionView[]>([]);

  useEffect(() => {
    if (!data) return;

    const newBase = data.data.actions;
    if (newBase !== lastFetchedRef.current) {
      lastFetchedRef.current = newBase;
      setActions(newBase);
    }
  }, [data]);

  return { actions, isLoading, capabilities: AUTOMATION_CAPABILITIES, selectedTagNames };
};
