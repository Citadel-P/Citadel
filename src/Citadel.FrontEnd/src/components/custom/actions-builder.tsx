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

export interface CommandAction<R, K extends KnownResourceName> {
  key: string;
  type: 'command';
  icon: LucideIcon;
  mutateKey?: K;
  onClick?: (resources: R[] | R) => void;
  canExecute?: (r: R | R[]) => boolean;
  confirm?: boolean;
  destructive?: boolean;
  invalidate?: K;
  variant?: 'default' | 'destructive' | 'outline' | 'secondary' | 'ghost' | 'link';
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
  useVariables?: (resources: R | R[]) => K extends KnownResourceName ? PrimaryArg<K> : any;
  useHandler?: (ctx: { resources: R | R[] }) => {
    run: () => void | Promise<void>;
    canExecute?: boolean;
    isPending?: boolean;
  };
  onSuccess?: (ctx: { resources: R | R[] }) => (() => void) | void;
  invalidate?: K;
  argName?: 'params' | 'data';
  destructive?: boolean;
  resourceType?: ResourceType;
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
  const title = capitalize(act.key);

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
    const { run, isPending, canExecute } = useUnifiedExecutor(act, resources, title, showToast);
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
      />
    );
  };

  const Info: ButtonActionComponent<R> = ({ resource }) => {
    const { run, isPending, canExecute } = useUnifiedExecutor(act, resource, title, showToast);
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
    argName?: any;
    canExecute?: any;
    useVariables?: any;
    useSuccessHandler?: any;
    onSuccess?: any;
  },
  resources: R | R[],
  title: string,
  showToast: boolean,
) {
  if (act.useHandler) {
    const handler = act.useHandler({ resources });
    return {
      run: async () => await handler.run(),
      canExecute: handler.canExecute ?? true,
      isPending: handler.isPending ?? false,
    };
  }

  return useMutationLogic(act, resources, title, showToast, act.invalidate, act.argName);
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
  invalidate?: string,
  argName: 'params' | 'data' = 'data',
) {
  const { mutateAsync, isPending } = useMutate(act.mutateKey || ('none' as any));
  const client = useQueryClient();

  const canExecute = act.mutateKey
    ? act.canExecute
      ? Array.isArray(resources)
        ? resources.some(act.canExecute)
        : act.canExecute(resources)
      : true
    : false;

  const vars = act.useVariables ? act.useVariables(resources) : undefined;

  const successCallback = act.useSuccessHandler
    ? act.useSuccessHandler({ resources })
    : act.onSuccess
      ? act.onSuccess({ resources })
      : undefined;

  const run = async () => {
    if (!act.mutateKey) return;
    try {
      await mutateAsync({ [argName]: vars });
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

  return { run, isPending, canExecute };
}
