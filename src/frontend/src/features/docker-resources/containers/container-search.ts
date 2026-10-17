import type { ContainerView } from '@/api/generated/api.types';

export function filterContainers(items: ContainerView[], search: string): ContainerView[] {
  const term = search.trim().toLowerCase();
  if (!term) return items;
  return items.filter((container) =>
    [
      container.name,
      container.containerId,
      container.dockerImageId,
      container.imageView?.name,
      ...(container.imageView?.tags ?? []),
      // Only server-authorized owner names, never raw Docker labels or guessed names.
      ...(container.ownerNames ?? []),
    ].some((value) => value?.toLowerCase().includes(term)),
  );
}
