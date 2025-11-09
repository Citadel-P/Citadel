/**
 * Defines the components needed to render a Docker resource page.
 * This single interface can represent either a **single-table resource** or a **tabbed resource**.
 */
export interface RequiredDockerComponents<T = any> {
  /** Optional configuration for header */
  header?: HeaderOptions;

  /** Optional subheader */
  SubHeader?: React.FC;

  /** Icon displayed in the page header */
  Icon: React.ReactElement;

  /** Dropdown actions for table rows */
  DropdownActions?: {
    [action: string]: DropdownActionComponent<T>;
  };

  /** Group actions for the action bar */
  GroupActions?: React.FC<{ items: any[] }>;

  /** Table component for single-table resources */
  Table?: React.FC<{
    items: any[];
    actions: Record<string, DropdownActionComponent<T>>;
    isLoading: boolean;
  }>;

  /** Data hook for single-table resources */
  useData?: (platformId: string) => ResourceDataHookResult<T>;

  /** Optional helper to filter items by search term */
  filterItems?: (items: T[], search: string) => T[];
}

/**
 * Defines the components needed to render a Docker info resource page.
 */
export interface RequiredDockerInfoComponents<T = any> {
  /** Configuration for header  */
  Header: {
    Indicator: React.FC<{ resource: T }>;
    ActionButtons: React.FC<{ resource: T }>;
  };
  /** Optional subheader */
  SubHeader?: React.FC<{ resource: T }>;
  /** Tabs configuration for tabbed resources */
  Tabs: {
    /** Tab label shown in the UI */
    label: string;

    /** Component(s) for this tab */
    Content: React.FC<{ resource: T }>;

    /** Data hook for this resource */
    useData?: (platformId: string, resourceId: string) => ResourceInfoHookResult<T>;
  }[];

  /** Data hook for this resource */
  useData: (platformId: string, resourceId: string) => ResourceInfoHookResult<T>;
}

export interface ResourceDataHookResult<T> {
  items: T[];
  isLoading: boolean;
}

interface ResourceInfoHookResult<T> {
  resource: T | undefined;
  isLoading: boolean;
  error: Error | null;
}

interface HeaderOptions {
  /** Whether to show the search field. Defaults to true. */
  showSearch?: boolean;
  /** Whether to show the "Add" button. Defaults to true. */
  showAdd?: boolean;
  /** Additional custom header items (buttons, dropdowns, etc.). */
  Extra?: React.FC;
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

export type ButtonActionComponent<T = any> = React.FC<{ resource: T }>;
export type ButtonGroupComponent<T = any> = React.FC<{ resources: T[] }>;
export type DropdownActionComponent<T = any> = React.FC<{
  resource: T;
  onAction?: (actionKey: string, actionData?: ActionData) => void;
}>;
