import { DockerResourceType } from '@/api/types';
import { ContainerComponents } from '@/features/docker-resources/containers';
import { ImageComponents } from '@/features/docker-resources/images';
import { NetworkComponents } from '@/features/docker-resources/networks';
import { VolumeComponents } from '@/features/docker-resources/volumes';

/**
 * Defines the components needed to render a Docker resource page.
 * This single interface can represent either a **single-table resource** or a **tabbed resource**.
 */
export interface RequiredDockerComponents<T = any> {
  /** Dialog component for deletion confirmation */
  DeleteDialog: React.FC;

  /** Icon displayed in the page header */
  Icon: React.ReactElement;

  /** Action bar component shown below the header */
  ActionBar: React.FC<{ items: any[] }>;

  /** Table component for single-table resources */
  Table?: React.FC<{ items: any[]; isLoading: boolean }>;

  /** Data hook for single-table resources */
  useData?: (platformId: string) => ResourceDataHookResult<T>;

  /** Optional helper to filter items by search term */
  filterItems?: (items: T[], search: string) => T[];

  /** Optional configuration for header  */
  header?: HeaderOptions;

  /** Tabs configuration for tabbed resources */
  tabs?: {
    /** Tab label shown in the UI */
    label: string;

    /** Table component for this tab */
    Content: React.FC<{ items: any[]; isLoading: boolean }>;

    /** Data hook for this tab */
    useData?: (platformId: string) => ResourceDataHookResult<T>;

    /** Optional configuration for header specific to this tab. */
    header?: HeaderOptions;
  }[];
}

export const DockerResourceComponents: {
  [key in DockerResourceType]: RequiredDockerComponents;
} = {
  Image: ImageComponents,
  Volume: VolumeComponents,
  Network: NetworkComponents,
  Container: ContainerComponents,
};

export interface ResourceDataHookResult<T> {
  items: T[];
  isLoading: boolean;
}

export interface HeaderOptions {
  /** Whether to show the search field. Defaults to true. */
  showSearch?: boolean;
  /** Whether to show the "Add" button. Defaults to true. */
  showAdd?: boolean;
  /** Additional custom header items (buttons, dropdowns, etc.). */
  Extra?: React.FC;
}
