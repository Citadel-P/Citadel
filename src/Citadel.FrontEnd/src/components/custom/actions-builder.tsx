import { LucideIcon } from 'lucide-react';
import { toast } from 'sonner';
import { useQueryClient } from '@tanstack/react-query';
import { DropdownActionButton } from '@/components/custom/dropdown-with-dialog';
import { ActionButton, ActionWithDialog, GroupActionWithDialog } from '@/components/custom/action-with-dialog';
import { useMutate } from '@/lib/hooks';
import { capitalize } from '@/lib/utils';
import type { DropdownActionComponent, ButtonGroupComponent, ButtonActionComponent } from '@/pages/types';
import type { KnownResourceName, PrimaryArg, ResourceType } from '@/api/types';

export type ActionKind = 'command' | 'toggle';
export type BaseResource = { name: string };

export interface CommandAction<R, K extends KnownResourceName | undefined = undefined> {
  key: string;
  type: 'command';
  icon: LucideIcon;
  mutateKey?: K;
  onClick?: (resources: R[] | R) => void;
  canExecute?: (r: R | R[]) => boolean;
  confirm?: boolean;
  destructive?: boolean;
  invalidate?: K;
  useVariables?: (resources: R[] | R) => K extends KnownResourceName ? PrimaryArg<K> : any;
  argName?: 'params' | 'data';
  resourceType?: ResourceType;
  separatorBefore?: boolean;
  useHandler?: (ctx: { resources: R | R[] }) => {
    run: () => void | Promise<void>;
    canExecute?: boolean;
    isPending?: boolean;
  };
  useSuccessHandler?: (ctx: { resources: R | R[] }) => (() => void) | void;
}

export interface ToggleAction<R, K extends KnownResourceName = KnownResourceName> {
  key: string;
  type: 'toggle';
  separatorBefore?: boolean;
  primary: ToggleConfig<R, K>;
  secondary: ToggleConfig<R, K>;
}

type ToggleConfig<R, K extends KnownResourceName> = {
  title: string;
  icon: LucideIcon;
  mutateKey: K;
  canExecute: (r: R) => boolean;
  useVariables?: (resources: R | R[]) => PrimaryArg<K>;
  onSuccess?: (ctx: { resources: R | R[] }) => (() => void) | void;
};

export type ActionConfig<R, K extends KnownResourceName | undefined = undefined> =
  | CommandAction<R, K>
  | ToggleAction<R>;

export function createActionsBuilder<R extends BaseResource>(options?: { showToast?: boolean }) {
  const actions: ActionConfig<R, any>[] = [];
  const showToast = options?.showToast ?? true;

  const builder = {
    addAction<K extends KnownResourceName | undefined>(action: ActionConfig<R, K>) {
      actions.push(action);
      return builder;
    },

    build() {
      const dropdown: Record<string, DropdownActionComponent<R>> = {};
      const group: Record<string, ButtonGroupComponent<R>> = {};
      const info: Record<string, ButtonActionComponent<R>> = {};

      for (const act of actions) {
        // Generate components based on type
        const components =
          act.type === 'toggle' ? createToggleComponents(act, showToast) : createCommandComponents(act, showToast);

        dropdown[act.key] = components.Dropdown;
        group[act.key] = components.Group;
        info[act.key] = components.Info;
      }

      return { dropdown, group, info };
    },
  };

  return builder;
}

// --- Component Factories ---
function createCommandComponents<R extends BaseResource>(act: CommandAction<R, any>, showToast: boolean) {
  const title = capitalize(act.key);

  const Dropdown: DropdownActionComponent<R> = ({ resource, onAction }) => {
    const { run, isPending, canExecute } = useUnifiedExecutor(act, resource, title, showToast);

    return (
      <DropdownActionButton
        title={title}
        icon={<act.icon className="h-4 w-4" />}
        separatorBefore={act.separatorBefore}
        variant={act.destructive ? 'destructive' : 'default'}
        disabled={!canExecute || isPending}
        onClick={
          act.confirm || act.destructive
            ? () =>
                onAction?.('confirm', {
                  name: resource.name,
                  title,
                  icon: <act.icon className="h-4 w-4" />,
                  onClick: run,
                  disabled: !canExecute,
                  variant: act.destructive ? 'destructive' : 'default',
                })
            : run
        }
      />
    );
  };

  const Group: ButtonGroupComponent<R> = ({ resources }) => {
    const { run, isPending, canExecute } = useUnifiedExecutor(act, resources, title, showToast);

    // Dialog / Confirmation Mode
    if (act.confirm || act.destructive) {
      return (
        <GroupActionWithDialog
          type={act.resourceType ?? 'Container'}
          name={title}
          title={title}
          iconPosition="left"
          variant={act.destructive ? 'destructive' : 'outline'}
          icon={<act.icon className="h-4 w-4" />}
          onClick={run}
          disabled={!canExecute || isPending}
        />
      );
    }

    // Standard Button Mode
    return (
      <ActionButton
        title={title}
        iconPosition="left"
        variant={act.destructive ? 'destructive' : 'outline'}
        icon={<act.icon className="h-4 w-4" />}
        onClick={run}
        disabled={!canExecute || isPending}
      />
    );
  };

  const Info: ButtonActionComponent<R> = ({ resource }) => {
    const { run, isPending, canExecute } = useUnifiedExecutor(act, resource, title, showToast);

    if (act.confirm || act.destructive) {
      return (
        <ActionWithDialog
          name={resource.name}
          title={title}
          iconPosition="left"
          icon={<act.icon className="h-4 w-4" />}
          onClick={run}
          disabled={!canExecute || isPending}
          variant={act.destructive ? 'destructive' : 'default'}
        />
      );
    }

    return (
      <ActionButton
        title={title}
        iconPosition="left"
        variant={act.destructive ? 'destructive' : 'outline'}
        icon={<act.icon className="h-4 w-4" />}
        onClick={run}
        disabled={!canExecute || isPending}
      />
    );
  };

  return { Dropdown, Group, Info };
}

/**
 * Creates the React components for a 'toggle' type action
 */
function createToggleComponents<R extends BaseResource>(act: ToggleAction<R>, showToast: boolean) {
  const useActiveConfig = (resources: R | R[]) => {
    const rList = Array.isArray(resources) ? resources : [resources];
    const isSecondary = rList.some(act.secondary.canExecute);
    return isSecondary ? act.secondary : act.primary;
  };

  const Dropdown: DropdownActionComponent<R> = ({ resource }) => {
    const config = useActiveConfig(resource);
    // Toggles are treated as mutations in the executor
    const { run, isPending, canExecute } = useMutationLogic(
      config,
      resource,
      config.title,
      showToast,
      undefined, // no explicit invalidate key on toggle config root usually
      'data',
    );

    return (
      <DropdownActionButton
        title={config.title}
        icon={<config.icon className="h-4 w-4" />}
        onClick={run}
        disabled={!canExecute || isPending}
        separatorBefore={act.separatorBefore}
      />
    );
  };

  const Group: ButtonGroupComponent<R> = ({ resources }) => {
    const config = useActiveConfig(resources);
    const { run, isPending, canExecute } = useMutationLogic(
      config,
      resources,
      config.title,
      showToast,
      undefined,
      'data',
    );

    return (
      <ActionButton
        title={config.title}
        icon={<config.icon className="h-4 w-4" />}
        iconPosition="left"
        variant="outline"
        onClick={run}
        disabled={!canExecute || isPending}
      />
    );
  };

  const Info: ButtonActionComponent<R> = ({ resource }) => {
    const config = useActiveConfig(resource);
    const { run, isPending, canExecute } = useMutationLogic(
      config,
      resource,
      config.title,
      showToast,
      undefined,
      'data',
    );

    return (
      <ActionButton
        iconPosition="left"
        variant="outline"
        title={config.title}
        icon={<config.icon className="h-4 w-4" />}
        onClick={run}
        disabled={!canExecute || isPending}
      />
    );
  };

  return { Dropdown, Group, Info };
}

/**
 * Decides whether to use the Handler hook or the Mutation hook logic
 */
function useUnifiedExecutor<R>(act: CommandAction<R, any>, resources: R | R[], title: string, showToast: boolean) {
  // If useHandler is defined, we use the custom hook logic
  // Note: We use a conditional check inside, but hook rules require consistent calling.
  // Since 'act' is constant for a specific component instance created by the builder, this is safe.

  if (act.useHandler) {
    const handler = act.useHandler({ resources });
    return {
      run: async () => await handler.run(),
      canExecute: handler.canExecute ?? true,
      isPending: handler.isPending ?? false,
    };
  }

  // Otherwise default to Mutation logic
  return useMutationLogic(act, resources, title, showToast, act.invalidate, act.argName);
}

/**
 * Handles the React Query mutation setup and execution wrappers
 */
function useMutationLogic<R, K extends KnownResourceName>(
  act: {
    mutateKey?: K;
    canExecute?: (r: any) => boolean;
    useVariables?: (r: any) => any;
    useSuccessHandler?: (ctx: any) => any;
    onSuccess?: (ctx: any) => any; // Support Toggle structure
  },
  resources: R | R[],
  title: string,
  showToast: boolean,
  invalidate?: string,
  argName: 'params' | 'data' = 'data',
) {
  const { mutateAsync, isPending } = useMutate(act.mutateKey!);
  const client = useQueryClient();

  const canExecute = act.canExecute
    ? Array.isArray(resources)
      ? resources.some(act.canExecute)
      : act.canExecute(resources)
    : true;

  const vars = act.useVariables ? act.useVariables(resources) : undefined;

  // Normalize success handler (Command uses useSuccessHandler, Toggle uses onSuccess)
  const successCallback = act.useSuccessHandler
    ? act.useSuccessHandler({ resources })
    : act.onSuccess
      ? act.onSuccess({ resources })
      : undefined;

  const run = async () => {
    try {
      await mutateAsync({ [argName]: vars });

      if (invalidate) {
        client.invalidateQueries({ queryKey: [invalidate] });
      }

      if (showToast) {
        const resourceName = Array.isArray(resources) ? `${resources.length} items` : (resources as any).name;
        toast.success(`${title} executed for ${resourceName}`);
      }

      if (successCallback) successCallback();
    } catch (err: any) {
      // Error handling is centralized here if needed, or left to global error boundary
      // For now, rethrow to let UI handle it if needed
      console.error(err);
    }
  };

  return { run, isPending, canExecute };
}
