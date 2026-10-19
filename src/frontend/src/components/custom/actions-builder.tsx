import { notifyRequestError } from '@/lib/request-error';
import { LucideIcon } from 'lucide-react';
import { ReactNode } from 'react';
import { toast } from 'sonner';
import { useQueryClient } from '@tanstack/react-query';
import { DropdownActionButton } from '@/components/custom/dropdown-with-dialog';
import { ActionButton, ActionWithDialog, GroupActionWithDialog } from '@/components/custom/action-with-dialog';
import { useMutate } from '@/lib/hooks';
import { capitalize } from '@/lib/utils';
import { CapabilityKey, hasCapabilities } from '@/lib/resource-capabilities';
import type { DropdownActionComponent, ButtonGroupComponent, ButtonActionComponent } from '@/pages/types';
import type { KnownResourceName, ResourceType, UseMutateVariables } from '@/api/types';

export type ActionKind = 'command' | 'toggle';
export type BaseResource = { name: string };

export interface CommandAction<R, K extends KnownResourceName> {
  key: string;
  title?: string;
  type: 'command';
  icon: LucideIcon;
  mutateKey?: K;
  onClick?: (resources: R[] | R) => void;
  isVisible?: (resources: R | R[]) => boolean;
  canExecute?: (r: R | R[]) => boolean;
  disabledReason?: (r: R | R[]) => string;
  confirm?: boolean;
  confirmationDescription?: ReactNode;
  destructive?: boolean;
  invalidate?: KnownResourceName;
  variant?: 'default' | 'destructive' | 'outline' | 'secondary' | 'ghost' | 'link';
  useVariables?: (resources: R[] | R) => UseMutateVariables<K>;
  resourceType?: ResourceType;
  separatorBefore?: boolean;
  requiredCapabilities?: CapabilityKey[];
  useHandler?: (ctx: { resources: R | R[] }) => {
    run: () => void | Promise<void>;
    canExecute?: boolean;
    isPending?: boolean;
    disabledReason?: string;
  };
  useSuccessHandler?: (ctx: { resources: R | R[] }) => (() => void) | void;
}

export interface ToggleAction<R, K extends KnownResourceName> {
  key: string;
  type: 'toggle';
  isVisible?: (resources: R | R[]) => boolean;
  separatorBefore?: boolean;
  predicate?: (r: R) => boolean;
  primary: ToggleConfig<R, K>;
  secondary: ToggleConfig<R, K>;
}

type ToggleConfig<R, K extends KnownResourceName> = {
  title: string;
  icon: LucideIcon;
  mutateKey?: K;
  confirm?: boolean;
  variant?: 'default' | 'destructive' | 'outline' | 'secondary' | 'ghost' | 'link';
  canExecute?: (r: R | R[]) => boolean;
  disabledReason?: (r: R | R[]) => string;
  useVariables?: (resources: R | R[]) => UseMutateVariables<K>;
  useHandler?: (ctx: { resources: R | R[] }) => {
    run: () => void | Promise<void>;
    canExecute?: boolean;
    isPending?: boolean;
    disabledReason?: string;
  };
  onSuccess?: (ctx: { resources: R | R[] }) => (() => void) | void;
  invalidate?: KnownResourceName;
  destructive?: boolean;
  resourceType?: ResourceType;
  requiredCapabilities?: CapabilityKey[];
};

export type ActionConfig<R, K extends KnownResourceName> = CommandAction<R, K> | ToggleAction<R, K>;

export function createActionsBuilder<R extends BaseResource>(options?: { showToast?: boolean }) {
  const actions: ActionConfig<R, any>[] = [];
  const showToast = options?.showToast ?? true;

  const builder = {
    addAction<K extends KnownResourceName>(action: ActionConfig<R, K>) {
      actions.push(action);
      return builder;
    },

    build() {
      const dropdown: Record<string, DropdownActionComponent<R>> = {};
      const group: Record<string, ButtonGroupComponent<R>> = {};
      const info: Record<string, ButtonActionComponent<R>> = {};

      for (const act of actions) {
        const components =
          act.type === 'toggle' ? createToggleComponents(act, showToast) : createCommandComponents(act, showToast);

        if (!act.isVisible) {
          dropdown[act.key] = components.Dropdown;
          group[act.key] = components.Group;
          info[act.key] = components.Info;
          continue;
        }

        const isVisible = act.isVisible;
        const Dropdown = components.Dropdown;
        const Group = components.Group;
        const Info = components.Info;

        dropdown[act.key] = (props) => (isVisible(props.resource) ? <Dropdown {...props} /> : null);
        group[act.key] = (props) => (isVisible(props.resources) ? <Group {...props} /> : null);
        info[act.key] = (props) => (isVisible(props.resource) ? <Info {...props} /> : null);
      }

      return { dropdown, group, info };
    },
  };

  return builder;
}

// --- Component Factories ---
function createCommandComponents<R extends BaseResource>(act: CommandAction<R, any>, showToast: boolean) {
  const title = act.title ?? capitalize(act.key);
  const useExecutor = createExecutor(act, title, showToast);

  const Dropdown: DropdownActionComponent<R> = ({ resource, onAction }) => {
    const { run, isPending, canExecute } = useExecutor(resource);
    const variant = act.variant || (act.destructive ? 'destructive' : 'outline');
    return (
      <DropdownActionButton
        title={title}
        icon={<act.icon className="h-4 w-4" />}
        separatorBefore={act.separatorBefore}
        variant={variant}
        disabled={!canExecute || isPending}
        loading={isPending}
        onClick={
          act.confirm || act.destructive
            ? () =>
                onAction?.('confirm', {
                  name: resource.name,
                  title,
                  icon: <act.icon className="h-4 w-4" />,
                  onClick: run,
                  disabled: !canExecute,
                  variant: variant,
                  description: act.confirmationDescription,
                })
            : () => {
                void run().catch(notifyRequestError);
              }
        }
      />
    );
  };

  const Group: ButtonGroupComponent<R> = ({ resources }) => {
    const { run, isPending, canExecute, disabledReason } = useExecutor(resources);
    const variant = act.variant || (act.destructive ? 'destructive' : 'outline');
    if (act.confirm || act.destructive) {
      return (
        <GroupActionWithDialog
          type={act.resourceType ?? 'Container'}
          name={title}
          title={title}
          iconPosition="left"
          variant={variant}
          icon={<act.icon className="h-4 w-4" />}
          onClick={run}
          description={act.confirmationDescription}
          disabled={!canExecute || isPending}
          disabledReason={!canExecute ? disabledReason : undefined}
        />
      );
    }

    return (
      <ActionButton
        title={title}
        iconPosition="left"
        variant={variant}
        icon={<act.icon className="h-4 w-4" />}
        onClick={() => {
          void run().catch(notifyRequestError);
        }}
        disabled={!canExecute || isPending}
        disabledReason={!canExecute ? disabledReason : undefined}
        loading={isPending}
      />
    );
  };

  const Info: ButtonActionComponent<R> = ({ resource }) => {
    const { run, isPending, canExecute, disabledReason } = useExecutor(resource);
    const variant = act.variant || (act.destructive ? 'destructive' : 'outline');
    if (act.confirm || act.destructive) {
      return (
        <ActionWithDialog
          name={resource.name}
          title={title}
          iconPosition="left"
          icon={<act.icon className="h-4 w-4" />}
          onClick={run}
          disabled={!canExecute || isPending}
          disabledReason={!canExecute ? disabledReason : undefined}
          variant={variant}
        />
      );
    }

    return (
      <ActionButton
        title={title}
        iconPosition="left"
        variant={variant}
        icon={<act.icon className="h-4 w-4" />}
        onClick={() => {
          void run().catch(notifyRequestError);
        }}
        disabled={!canExecute || isPending}
        disabledReason={!canExecute ? disabledReason : undefined}
        loading={isPending}
      />
    );
  };

  return { Dropdown, Group, Info };
}

function createToggleComponents<R extends BaseResource>(act: ToggleAction<R, any>, showToast: boolean) {
  // Each branch owns its hooks. Switching branches remounts the selected component.
  const primary = createCommandComponents(
    {
      ...act.primary,
      useSuccessHandler: act.primary.onSuccess,
      key: act.key,
      type: 'command',
      separatorBefore: act.separatorBefore,
    },
    showToast,
  );
  const secondary = createCommandComponents(
    {
      ...act.secondary,
      useSuccessHandler: act.secondary.onSuccess,
      key: act.key,
      type: 'command',
      separatorBefore: act.separatorBefore,
    },
    showToast,
  );
  const select = (resources: R | R[]) => {
    const list = Array.isArray(resources) ? resources : [resources];
    return (act.predicate ? list.some(act.predicate) : list.some((r) => act.secondary.canExecute?.(r)))
      ? secondary
      : primary;
  };
  const Dropdown: DropdownActionComponent<R> = (props) => {
    const Component = select(props.resource).Dropdown;
    return <Component {...props} />;
  };
  const Group: ButtonGroupComponent<R> = (props) => {
    const Component = select(props.resources).Group;
    return <Component {...props} />;
  };
  const Info: ButtonActionComponent<R> = (props) => {
    const Component = select(props.resource).Info;
    return <Component {...props} />;
  };
  return { Dropdown, Group, Info };
}

function createExecutor<R extends BaseResource>(act: CommandAction<R, any>, title: string, showToast: boolean) {
  const requiredCapabilities: CapabilityKey[] =
    act.requiredCapabilities ?? (act.destructive ? ['canExecute'] : act.mutateKey ? ['canWrite'] : []);
  const useHandler = act.useHandler;
  if (useHandler) {
    return function useHandlerExecutor(resources: R | R[]) {
      const capabilitiesAllow = hasCapabilities(resources, requiredCapabilities);
      const handler = useHandler({ resources });
      const canExecute = (handler.canExecute ?? true) && (act.canExecute?.(resources) ?? true) && capabilitiesAllow;
      return {
        run: async () => await handler.run(),
        canExecute,
        isPending: handler.isPending ?? false,
        disabledReason: canExecute
          ? undefined
          : capabilitiesAllow
            ? (handler.disabledReason ??
              act.disabledReason?.(resources) ??
              'This action is unavailable in the current resource state.')
            : 'You do not have permission to perform this action.',
      };
    };
  }
  return function useMutationExecutor(resources: R | R[]) {
    return useMutationLogic(
      act,
      resources,
      title,
      showToast,
      hasCapabilities(resources, requiredCapabilities),
      act.invalidate,
    );
  };
}

function useMutationLogic<R, K extends KnownResourceName>(
  act: {
    mutateKey?: K;
    canExecute?: (r: any) => boolean;
    disabledReason?: (r: R | R[]) => string;
    useVariables?: (r: any) => any;
    useSuccessHandler?: (ctx: any) => any;
    onSuccess?: (ctx: any) => any;
  },
  resources: R | R[],
  title: string,
  showToast: boolean,
  capabilitiesAllow: boolean,
  invalidate?: string,
) {
  const { mutateAsync, isPending } = useMutate(act.mutateKey || ('none' as any));
  const client = useQueryClient();

  const canExecute = act.mutateKey ? (act.canExecute ? act.canExecute(resources) : true) && capabilitiesAllow : false;

  const vars = act.useVariables ? act.useVariables(resources) : undefined;

  const successCallback = act.useSuccessHandler
    ? act.useSuccessHandler({ resources })
    : act.onSuccess
      ? act.onSuccess({ resources })
      : undefined;

  const run = async () => {
    if (!act.mutateKey) return;
    try {
      await mutateAsync(vars);
      if (invalidate) client.invalidateQueries({ queryKey: [invalidate] });
      if (showToast) {
        const name = Array.isArray(resources) ? `${resources.length} items` : (resources as any).name;
        toast.success(`${title} requested for ${name}`);
      }
      if (successCallback) await successCallback();
    } catch (err) {
      notifyRequestError(err);
      throw err;
    }
  };

  return {
    run,
    isPending,
    canExecute,
    disabledReason: canExecute
      ? undefined
      : !capabilitiesAllow
        ? 'You do not have permission to perform this action.'
        : (act.disabledReason?.(resources) ?? 'This action is unavailable in the current resource state.'),
  };
}
