import { ResourceCapabilities } from '@/api/generated/api.types';

type ResourceContentProps<T = any> = {
  items: any;
  actions: Record<string, DropdownActionComponent<T>>;
  isLoading: boolean;
  isFiltered?: boolean;
};

interface BaseResourceComponents<T = any> {
  /** Optional configuration for header */
  header?: HeaderOptions;

  /** Optional subheader */
  SubHeader?: React.FC;

  /** Icon displayed in the page header */
  Icon?: React.ComponentType<{ className?: string }>;

  /** Dropdown actions for table rows */
  DropdownActions?: {
    [action: string]: DropdownActionComponent<T>;
  };

  /** Group actions for the action bar */
  GroupActions?: React.FC<{ items: any[] }>;

  /** Data hook for the resource */
  useData?: (platformId: string) => ResourceDataHookResult<T>;

  /** Optional helper to filter items by search term */
  filterItems?: (items: T[], search: string) => T[];
}

/**
 * Defines the components needed to render a regular resource page.
 */
export interface RegularResourceComponents<T = any> extends BaseResourceComponents<T> {
  /** The main content component */
  Content: React.FC<ResourceContentProps<T>>;
  Tabs?: never;
}

/**
 * Defines the components needed to render a tabbed resource page.
 */
export interface TabbedResourceComponents<T = any> extends BaseResourceComponents<T> {
  /** Tabs configuration */
  Tabs: TabElement<T>[];
  Content?: never;
}

export type RequiredComponents<T = any> = RegularResourceComponents<T> | TabbedResourceComponents<T>;

/**
 * Encapsulates the logic and UI for the resource mutation page (Create/Edit)
 */
export interface RequiredFormComponents<T = any> {
  AddForm: {
    Header?: {
      title?: string;
    };
    /** The component responsible for rendering the main Add form. */
    Content?: React.FC;
  };

  EditForm?: {
    Header: {
      canEditTitle?: boolean;
      canEditDescription?: boolean;
      Indicator: React.FC<{ resource: T }>;
      Tags?: React.FC<{ resource: T }>;
      ActionButtons: React.FC<{ resource: T }>;
    };
    skipMetadataUpdate?: boolean;
    /** Optional subheader */
    SubHeader?: React.FC<{ resource: T }>;
    /** Tabs configuration */
    Tabs: ResourceTabElement<T>[];
    /** Main Data hook for this workload */
    useData: (id: string) => { item?: RequiredFormFields; isLoading: boolean };
  };
}

/**
 * Defines the components needed to render a Docker info resource page.
 */
export interface RequiredDockerInfoComponents<T = any> {
  /** Configuration for header  */
  Header: {
    Indicator: React.FC<{ resource: T }>;
    NameSuffix?: React.FC<{ resource: T }>;
    ActionButtons: React.FC<{ resource: T }>;
  };
  /** Optional subheader */
  SubHeader?: React.FC<{ resource: T }>;
  /** Tabs configuration for tabbed resources */
  Tabs: ResourceTabElement<T>[];
  /** Data hook for this resource */
  useData: (platformId: string, resourceId: string) => ResourceInfoHookResult<T>;
}

export interface ResourceDataHookResult<T> {
  items: T[];
  isLoading: boolean;
  capabilities: ResourceCapabilities | undefined;
}

export interface TabHeaderOptions {
  /** Optional left-side action buttons that appear next to the tab header */
  showSearch?: boolean;
  /** Placeholder text for the per-tab search field */
  searchPlaceholder?: string;
  /** Optional per-tab search callback */
  onSearch?: (query: string) => void;
  /** Whether to show an Add button in the tab header (right area) */
  showAdd?: boolean;
  /** Override text for the per-tab Add button */
  addButtonTitle?: string;
  /** Override URL for the per-tab Add button */
  addButtonUrl?: string;
  /** Additional custom header items (buttons, dropdowns, etc.). */
  Extra?: React.FC;
}

export interface TabElement<T> {
  /** Tab label shown in the UI */
  label: string;
  /** Optional route segment used when a tab is represented in the URL */
  slug?: string;
  /** Wether this tab is disabled */
  disabled?(resource: T): boolean;
  /** Comtent for this tab */
  Content: React.FC<ResourceContentProps<T>>;
  /** Per-tab header configuration for the tab header right area */
  Header?: TabHeaderOptions;
  /** Dropdown actions for table rows */
  DropdownActions?: {
    [action: string]: DropdownActionComponent<T>;
  };
  /** Group actions for the action bar */
  GroupActions?: React.FC<{ items: any[] }>;
  /** Data hook for the resource */
  useData?: () => ResourceDataHookResult<T>;
}

export type ResourceTabContentProps<T> = {
  resource: T;
  metadataChanged?: boolean;
};

export interface ResourceTabElement<T> {
  /** Tab label shown in the UI */
  label: string;
  /** Optional route segment used when a tab is represented in the URL */
  slug?: string;
  /** Wether this tab is disabled */
  disabled?(resource: T): boolean;
  /** Content for tabs rendered from a single resource context */
  Content: React.FC<ResourceTabContentProps<T>>;
}

interface ResourceInfoHookResult<T> {
  resource: T | undefined;
  isLoading: boolean;
  error: Error | null;
}

interface HeaderOptions {
  /** Override title */
  title?: string;
  /** Override subtitle */
  subtitle?: string;
  /** Whether to show the search field. Defaults to true. */
  showSearch?: boolean;
  /** Whether to show the "Add" button. Defaults to true. */
  showAdd?: boolean;
  /** Whether to show the tag filter control in the page header. */
  showTagFilter?: boolean;
  /** Whether to show the platform filter control in the page header. */
  showPlatformFilter?: boolean;
  /** Override add button title */
  addButtonTitle?: string;
  /** Override URL for the standard Add button. */
  addButtonUrl?: string;
  /** Additional custom header items (buttons, dropdowns, etc.). */
  Extra?: React.FC;
  /** Query parameters that contribute to the page's filtered state. */
  activeFilterParams?: string[];
  /** Optional add dialog opened by the standard Add button instead of navigating to an add route. */
  AddDialog?: React.FC<{ open: boolean; onOpenChange: (open: boolean) => void }>;
}

export type ActionData = {
  name: string;
  title: string;
  icon: React.ReactNode;
  disabled?: boolean;
  loading?: boolean;
  onClick?: () => void | Promise<unknown>;
  variant?: 'link' | 'default' | 'destructive' | 'outline' | 'secondary' | 'ghost' | null | undefined;
};

export interface ResourceFormProps<T> {
  mode: 'add' | 'edit';
  resource?: T;
}

export type ButtonActionComponent<T = any> = React.FC<{ resource: T }>;
export type ButtonGroupComponent<T = any> = React.FC<{ resources: T[] }>;
export type DropdownActionComponent<T = any> = React.FC<{
  resource: T;
  onAction?: (actionKey: string, actionData?: ActionData) => void;
}>;

export type RequiredFormFields = { name: string; description: string | null; status: unknown };
