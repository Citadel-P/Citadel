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
  primary: {
    title: string;
    icon: LucideIcon;
    mutateKey: K;
    canExecute: (r: R) => boolean;
    useVariables?: (resources: R | R[]) => PrimaryArg<K>;
    onSuccess?: (ctx: { resources: R | R[] }) => (() => void) | void;
  };
  secondary: {
    title: string;
    icon: LucideIcon;
    mutateKey: K;
    canExecute: (r: R) => boolean;
    useVariables?: (resources: R | R[]) => PrimaryArg<K>;
    onSuccess?: (ctx: { resources: R | R[] }) => (() => void) | void;
  };
}

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
        const title = capitalize(act.key);

        const useRunActionMutation = (mutateAsync: any) => {
          const { executeMutationWithToast } = useMutationExecutor(showToast);

          return async (
            vars: any,
            successMessage: string,
            onSuccess: void | (() => void) | undefined,
            invalidate?: string,
            argName: 'params' | 'data' = 'data',
          ) => {
            await executeMutationWithToast({
              title,
              mutateAsync,
              variables: { [argName]: vars },
              invalidate,
              successMessage,
            });
            onSuccess?.();
          };
        };

        if (act.type === 'command') {
          let Dropdown: DropdownActionComponent<R>;
          let Group: ButtonGroupComponent<R>;
          let Info: ButtonActionComponent<R>;

          // HOOK-BASED BEHAVIOR (useHandler)
          if (act.useHandler) {
            Dropdown = ({ resource, onAction }) => {
              const { run, canExecute = true, isPending = false } = act.useHandler!({ resources: resource });
              const execute = async () => await run();

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
                            onClick: execute,
                            disabled: !canExecute,
                            variant: act.destructive ? 'destructive' : 'default',
                          })
                      : execute
                  }
                />
              );
            };

            Group = ({ resources }) => {
              const { run, canExecute = true, isPending = false } = act.useHandler!({ resources });
              const execute = async () => await run();

              if (act.confirm || act.destructive) {
                return (
                  <GroupActionWithDialog
                    type={act.resourceType ?? 'Container'}
                    name="Delete"
                    title="Delete"
                    iconPosition="left"
                    variant={act.destructive ? 'destructive' : 'outline'}
                    icon={<act.icon className="h-4 w-4" />}
                    onClick={execute}
                    disabled={!canExecute || isPending}
                  />
                );
              }

              return (
                <ActionButton
                  title={title}
                  iconPosition="left"
                  variant={act.destructive ? 'destructive' : 'outline'}
                  icon={<act.icon className="h-4 w-4" />}
                  onClick={execute}
                  disabled={!canExecute || isPending}
                />
              );
            };

            Info = ({ resource }) => {
              const { run, canExecute = true, isPending = false } = act.useHandler!({ resources: resource });
              const execute = async () => await run();

              if (act.confirm || act.destructive) {
                return (
                  <ActionWithDialog
                    name={resource.name}
                    title={title}
                    iconPosition="left"
                    icon={<act.icon className="h-4 w-4" />}
                    onClick={execute}
                    disabled={!canExecute || isPending}
                    variant={act.destructive ? 'destructive' : 'default'}
                  />
                );
              }

              return (
                <ActionWithDialog
                  name={resource.name}
                  title={capitalize(act.key)}
                  iconPosition="left"
                  icon={<act.icon className="h-4 w-4" />}
                  onClick={execute}
                  disabled={!canExecute || isPending}
                  variant={act.destructive ? 'destructive' : 'default'}
                />
              );
            };
          }
          // DEFAULT MUTATION-BASED BEHAVIOR
          else {
            Dropdown = ({ resource, onAction }) => {
              const { mutateAsync, isPending } = useMutate(act.mutateKey);
              const runActionMutation = useRunActionMutation(mutateAsync);
              const canRun = act.canExecute?.(resource) ?? true;
              const vars = act.useVariables?.(resource);
              const successCallback = act.useSuccessHandler?.({ resources: resource });

              const execute = async () => {
                await runActionMutation(
                  vars,
                  `${title} executed for ${resource.name}`,
                  successCallback,
                  act.invalidate,
                  act.argName,
                );
              };

              return (
                <DropdownActionButton
                  title={title}
                  icon={<act.icon className="h-4 w-4" />}
                  separatorBefore={act.separatorBefore}
                  variant={act.destructive ? 'destructive' : 'default'}
                  disabled={!canRun || isPending}
                  onClick={
                    act.confirm || act.destructive
                      ? () =>
                          onAction?.('confirm', {
                            name: resource.name,
                            title,
                            icon: <act.icon className="h-4 w-4" />,
                            onClick: execute,
                            disabled: !canRun,
                            variant: 'destructive',
                          })
                      : execute
                  }
                />
              );
            };

            Group = ({ resources }) => {
              const { mutateAsync, isPending } = useMutate(act.mutateKey);
              const runActionMutation = useRunActionMutation(mutateAsync);
              const canRun = act.canExecute?.(resources) ?? true;
              const vars = act.useVariables?.(resources);
              const successCallback = act.useSuccessHandler?.({ resources });

              const execute = async () => {
                await runActionMutation(
                  vars,
                  `${resources.length} ${title.toLowerCase()} completed.`,
                  successCallback,
                  act.invalidate,
                  act.argName,
                );
              };

              if (act.confirm || act.destructive) {
                return (
                  <GroupActionWithDialog
                    type={act.resourceType ?? 'Container'}
                    name="Delete"
                    title="Delete"
                    iconPosition="left"
                    variant={act.destructive ? 'destructive' : 'outline'}
                    icon={<act.icon className="h-4 w-4" />}
                    onClick={execute}
                    disabled={!canRun || isPending}
                  />
                );
              }

              return (
                <ActionButton
                  title={title}
                  iconPosition="left"
                  variant={act.destructive ? 'destructive' : 'outline'}
                  icon={<act.icon className="h-4 w-4" />}
                  onClick={execute}
                  disabled={!canRun || isPending}
                />
              );
            };

            Info = ({ resource }) => {
              const { mutateAsync, isPending } = useMutate(act.mutateKey);
              const runActionMutation = useRunActionMutation(mutateAsync);
              const canRun = act.canExecute?.(resource) ?? true;
              const vars = act.useVariables?.(resource);
              const successCallback = act.useSuccessHandler?.({ resources: resource });

              const execute = async () => {
                await runActionMutation(
                  vars,
                  `${capitalize(act.key)} executed for ${resource.name}`,
                  successCallback,
                  act.invalidate,
                );
              };

              if (act.confirm || act.destructive) {
                return (
                  <ActionWithDialog
                    name={resource.name}
                    title={capitalize(act.key)}
                    iconPosition="left"
                    icon={<act.icon className="h-4 w-4" />}
                    onClick={execute}
                    disabled={!canRun || isPending}
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
                  onClick={execute}
                  disabled={!canRun || isPending}
                />
              );
            };
          }

          dropdown[act.key] = Dropdown;
          group[act.key] = Group;
          info[act.key] = Info;
        }

        if (act.type === 'toggle') {
          const Dropdown: DropdownActionComponent<R> = ({ resource }) => {
            const useSecondary = act.secondary.canExecute(resource);
            const cfg = useSecondary ? act.secondary : act.primary;
            const { mutateAsync, isPending } = useMutate(cfg.mutateKey);
            const runActionMutation = useRunActionMutation(mutateAsync);
            const canRun = cfg.canExecute(resource);
            const vars = cfg.useVariables ? cfg.useVariables(resource) : undefined;
            const successCallback = cfg.onSuccess?.({ resources: resource });

            const handleRun = async () => {
              await runActionMutation(vars, `${cfg.title} executed for ${resource.name}`, successCallback);
            };

            return (
              <DropdownActionButton
                title={cfg.title}
                icon={<cfg.icon className="h-4 w-4" />}
                onClick={handleRun}
                disabled={!canRun || isPending}
                separatorBefore={act.separatorBefore}
              />
            );
          };

          const Group: ButtonGroupComponent<R> = ({ resources }) => {
            const useSecondary = resources.some(act.secondary.canExecute);
            const cfg = useSecondary ? act.secondary : act.primary;
            const { mutateAsync, isPending } = useMutate(cfg.mutateKey);
            const runActionMutation = useRunActionMutation(mutateAsync);
            const canRun = resources.some(cfg.canExecute);
            const vars = cfg.useVariables ? cfg.useVariables(resources) : undefined;
            const successCallback = cfg.onSuccess?.({ resources });

            const handleRun = async () => {
              await runActionMutation(vars, `${cfg.title} executed for ${resources.length} items.`, successCallback);
            };

            return (
              <ActionButton
                title={cfg.title}
                icon={<cfg.icon className="h-4 w-4" />}
                iconPosition="left"
                variant="outline"
                onClick={handleRun}
                disabled={!canRun || isPending}
              />
            );
          };

          const Info: ButtonActionComponent<R> = ({ resource }) => {
            const useSecondary = act.secondary.canExecute(resource);
            const cfg = useSecondary ? act.secondary : act.primary;
            const { mutateAsync, isPending } = useMutate(cfg.mutateKey);
            const runActionMutation = useRunActionMutation(mutateAsync);
            const canRun = cfg.canExecute(resource);
            const vars = cfg.useVariables ? cfg.useVariables(resource) : undefined;
            const successCallback = cfg.onSuccess?.({ resources: resource });

            const handleRun = async () => {
              await runActionMutation(vars, `${cfg.title} executed for ${resource.name}`, successCallback);
            };

            return (
              <ActionButton
                iconPosition="left"
                variant="outline"
                title={cfg.title}
                icon={<cfg.icon className="h-4 w-4" />}
                onClick={handleRun}
                disabled={!canRun || isPending}
              />
            );
          };

          dropdown[act.key] = Dropdown;
          group[act.key] = Group;
          info[act.key] = Info;
        }
      }

      return { dropdown, group, info };
    },
  };

  return builder;
}

function useMutationExecutor(showToast: boolean) {
  const client = useQueryClient();

  async function executeMutationWithToast({
    mutateAsync,
    variables,
    invalidate,
    successMessage,
  }: {
    title: string;
    mutateAsync: (vars: any) => Promise<any>;
    variables: any;
    invalidate?: string;
    successMessage: string;
  }) {
    try {
      await mutateAsync(variables);
      if (invalidate) client.invalidateQueries({ queryKey: [invalidate] });
      if (showToast) toast.success(successMessage);
      return true;
    } catch (err: any) {
      // if (showToast) toast.error(`${title} failed: ${err.message ?? 'Unknown error'}`);
      // else console.error(`[${title}] mutation failed:`, err);
      throw err;
    }
  }

  return { executeMutationWithToast };
}
