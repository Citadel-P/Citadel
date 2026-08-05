import { LucideIcon } from 'lucide-react';
import { toast } from 'sonner';
import { useQueryClient } from '@tanstack/react-query';
import { DropdownActionButton } from '@/components/custom/dropdown-with-dialog';
import { ActionButton, ActionWithDialog, GroupActionWithDialog } from '@/components/custom/action-with-dialog';
import { useMutate } from '@/lib/hooks';
import { capitalize } from '@/lib/utils';
import { CapabilityKey, hasCapabilities } from '@/lib/resource-capabilities';
import type { DropdownActionComponent, ButtonGroupComponent, ButtonActionComponent } from '@/pages/types';
import type { KnownResourceName, PrimaryArg, ResourceType, UseMutateVariables } from '@/api/types';

export type ActionKind = 'command' | 'toggle';
export type BaseResource = { name: string };

export interface CommandAction<R, K extends KnownResourceName> {
  key: string;
  title?: string;
  type: 'command';
  icon: LucideIcon;
  mutateKey?: K;
  onClick?: (resources: R[] | R) => void;
  canExecute?: (r: R | R[]) => boolean;
  confirm?: boolean;
  destructive?: boolean;
  invalidate?: K;
  variant?: 'default' | 'destructive' | 'outline' | 'secondary' | 'ghost' | 'link';
  useVariables?: (resources: R[] | R) => K extends KnownResourceName ? PrimaryArg<K> | UseMutateVariables<K> : any;
  argName?: 'params' | 'data' | 'variables';
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
  canExecute?: (r: R) => boolean;
  useVariables?: (resources: R | R[]) => K extends KnownResourceName ? PrimaryArg<K> | UseMutateVariables<K> : any;
  useHandler?: (ctx: { resources: R | R[] }) => {
    run: () => void | Promise<void>;
    canExecute?: boolean;
    isPending?: boolean;
  };
  onSuccess?: (ctx: { resources: R | R[] }) => (() => void) | void;
  invalidate?: K;
  argName?: 'params' | 'data' | 'variables';
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
  const title = act.title ?? capitalize(act.key);

  const Dropdown: DropdownActionComponent<R> = ({ resource, onAction }) => {
    const { run, isPending, canExecute } = useUnifiedExecutor(act, resource, title, showToast);
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
                })
            : run
        }
      />
    );
  };

  const Group: ButtonGroupComponent<R> = ({ resources }) => {
    const { run, isPending, canExecute, disabledReason } = useUnifiedExecutor(act, resources, title, showToast);
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
          disabled={!canExecute || isPending}
        />
      );
    }

    return (
      <ActionButton
        title={title}
        iconPosition="left"
        variant={variant}
        icon={<act.icon className="h-4 w-4" />}
        onClick={run}
        disabled={!canExecute || isPending}
        disabledReason={!canExecute ? disabledReason : undefined}
        loading={isPending}
      />
    );
  };

  const Info: ButtonActionComponent<R> = ({ resource }) => {
    const { run, isPending, canExecute, disabledReason } = useUnifiedExecutor(act, resource, title, showToast);
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
        onClick={run}
        disabled={!canExecute || isPending}
        disabledReason={!canExecute ? disabledReason : undefined}
        loading={isPending}
      />
    );
  };

  return { Dropdown, Group, Info };
}

function createToggleComponents<R extends BaseResource>(act: ToggleAction<R, any>, showToast: boolean) {
  const useActiveConfig = (resources: R | R[]) => {
    const rList = Array.isArray(resources) ? resources : [resources];
    const isSecondary = act.predicate ? rList.some(act.predicate) : rList.some((r) => act.secondary.canExecute?.(r));

    return isSecondary ? act.secondary : act.primary;
  };

  const Dropdown: DropdownActionComponent<R> = ({ resource, onAction }) => {
    const config = useActiveConfig(resource);
    const variant = config.variant || (config.destructive ? 'destructive' : 'outline');
    const { run, isPending, canExecute } = useUnifiedExecutor(config, resource, config.title, showToast);

    return (
      <DropdownActionButton
        title={config.title}
        icon={<config.icon className="h-4 w-4" />}
        variant={variant}
        onClick={
          config.confirm || config.destructive
            ? () =>
                onAction?.('confirm', {
                  name: resource.name,
                  title: config.title,
                  icon: <config.icon className="h-4 w-4" />,
                  onClick: run,
                  disabled: !canExecute,
                  variant: variant,
                })
            : run
        }
        disabled={!canExecute || isPending}
        loading={isPending}
        separatorBefore={act.separatorBefore}
      />
    );
  };

  const Group: ButtonGroupComponent<R> = ({ resources }) => {
    const config = useActiveConfig(resources);
    const variant = config.variant || (config.destructive ? 'destructive' : 'outline');
    const { run, isPending, canExecute } = useUnifiedExecutor(config, resources, config.title, showToast);

    if (config.confirm || config.destructive) {
      return (
        <GroupActionWithDialog
          type={config.resourceType ?? 'Container'}
          name={config.title}
          title={config.title}
          iconPosition="left"
          variant={variant}
          icon={<config.icon className="h-4 w-4" />}
          onClick={run}
          disabled={!canExecute || isPending}
        />
      );
    }

    return (
      <ActionButton
        title={config.title}
        icon={<config.icon className="h-4 w-4" />}
        iconPosition="left"
        variant={variant}
        onClick={run}
        disabled={!canExecute || isPending}
        loading={isPending}
      />
    );
  };

  const Info: ButtonActionComponent<R> = ({ resource }) => {
    const config = useActiveConfig(resource);
    const variant = config.variant || (config.destructive ? 'destructive' : 'outline');
    const { run, isPending, canExecute } = useUnifiedExecutor(config, resource, config.title, showToast);

    if (config.confirm || config.destructive) {
      return (
        <ActionWithDialog
          name={resource.name}
          title={config.title}
          iconPosition="left"
          variant={variant}
          icon={<config.icon className="h-4 w-4" />}
          onClick={run}
          disabled={!canExecute || isPending}
        />
      );
    }

    return (
      <ActionButton
        iconPosition="left"
        variant={variant}
        title={config.title}
        icon={<config.icon className="h-4 w-4" />}
        onClick={run}
        disabled={!canExecute || isPending}
        loading={isPending}
      />
    );
  };

  return { Dropdown, Group, Info };
}

function useUnifiedExecutor<R>(
  act: {
    useHandler?: any;
    mutateKey?: KnownResourceName;
    invalidate?: any;
    argName?: 'params' | 'data' | 'variables';
    canExecute?: any;
    useVariables?: any;
    useSuccessHandler?: any;
    onSuccess?: any;
    destructive?: boolean;
    requiredCapabilities?: CapabilityKey[];
  },
  resources: R | R[],
  title: string,
  showToast: boolean,
) {
  const requiredCapabilities: CapabilityKey[] =
    act.requiredCapabilities ?? (act.destructive ? ['canExecute'] : act.mutateKey ? ['canWrite'] : []);
  const capabilitiesAllow = hasCapabilities(resources, requiredCapabilities);

  if (act.useHandler) {
    const handler = act.useHandler({ resources });
    const canExecute = (handler.canExecute ?? true) && capabilitiesAllow;
    return {
      run: async () => await handler.run(),
      canExecute,
      isPending: handler.isPending ?? false,
      disabledReason: canExecute
        ? undefined
        : capabilitiesAllow
          ? handler.disabledReason
          : 'You do not have permission to perform this action.',
    };
  }

  return useMutationLogic(act, resources, title, showToast, capabilitiesAllow, act.invalidate, act.argName);
}

function useMutationLogic<R, K extends KnownResourceName>(
  act: {
    mutateKey?: K;
    canExecute?: (r: any) => boolean;
    useVariables?: (r: any) => any;
    useSuccessHandler?: (ctx: any) => any;
    onSuccess?: (ctx: any) => any;
  },
  resources: R | R[],
  title: string,
  showToast: boolean,
  capabilitiesAllow: boolean,
  invalidate?: string,
  argName: 'params' | 'data' | 'variables' = 'data',
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
      await mutateAsync(argName === 'variables' ? vars : { [argName]: vars });
      if (invalidate) client.invalidateQueries({ queryKey: [invalidate] });
      if (showToast) {
        const name = Array.isArray(resources) ? `${resources.length} items` : (resources as any).name;
        toast.success(`${title} executed for ${name}`);
      }
      if (successCallback) successCallback();
    } catch (err) {
      console.error(err);
    }
  };

  return { run, isPending, canExecute, disabledReason: undefined };
}
