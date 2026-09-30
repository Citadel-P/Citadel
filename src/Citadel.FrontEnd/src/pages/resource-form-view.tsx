import { canShowCachedResource } from '@/lib/request-error';
import { ResourceReadError } from '@/components/custom/resource-read-error';
import { AppContent } from '@/components/custom/app-content';
import { useEffect, useRef, useState } from 'react';
import { useParams } from 'react-router';
import { Pencil, Plus, Save, X } from 'lucide-react';
import { useQueryClient } from '@tanstack/react-query';

import { useMutate } from '@/lib/hooks';
import { capitalize } from '@/lib/utils';
import { CitadelIcons } from '@/lib/icons';
import { PageHeader } from '@/components/custom/page-header';

import { FieldInput } from '@/components/custom/form-builder';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';

import NotFound from './not-found';
import { RequiredFormFields, RequiredFormComponents } from './types';
import { ResourceType } from '@/api/types';
import { ResourceSkeleton } from './resource-skeleton';
import { ResourceTabs } from '@/components/custom/resource-tabs';
import TaskSheet from '@/components/custom/task-sheet';
import { PatchResourceMetadataInput } from '@/api/generated/api.types';
import { hasCapability } from '@/lib/resource-capabilities';

export type ResourceFormViewProps = { type: ResourceType; mode: 'add' | 'edit'; Components: RequiredFormComponents };

export const ResourceFormView = ({ type, mode, Components }: ResourceFormViewProps) => (
  <PageShell mode={mode}>
    {mode === 'add' ? (
      <AddFormPage type={type} Components={Components} />
    ) : (
      <EditFormPage type={type} Components={Components} skipMetadataUpdate={Components.EditForm?.skipMetadataUpdate} />
    )}
  </PageShell>
);

const AddFormPage = ({ type, Components: form }: Omit<ResourceFormViewProps, 'mode'>) => {
  const Components = form.AddForm;

  if (!Components?.Content) return <NotFound />;

  return (
    <>
      <AddHeader type={type} title={Components.Header?.title} />
      <Components.Content />
    </>
  );
};

const EditFormPage = ({
  type,
  Components: form,
  skipMetadataUpdate = false,
}: Omit<ResourceFormViewProps, 'mode'> & { skipMetadataUpdate?: boolean }) => {
  const { id } = useParams();

  const Components = form.EditForm;
  if (!Components?.useData) return <NotFound />;

  // Each resource type supplies a different hook tree; never reuse it across resources.
  return (
    <EditFormData
      key={`${type}:${id}`}
      id={id!}
      type={type}
      Components={Components}
      skipMetadataUpdate={skipMetadataUpdate}
    />
  );
};

const EditFormData = ({
  id,
  type,
  Components,
  skipMetadataUpdate,
}: {
  id: string;
  type: ResourceType;
  Components: any;
  skipMetadataUpdate: boolean;
}) => {
  const formData = Components.useData(id!);

  const { item, isLoading, error, refetch, isFetching } = formData ?? {};

  if (error && !canShowCachedResource(error))
    return <ResourceReadError error={error} refetch={refetch} isFetching={isFetching} />;
  if (!item && isLoading) return <ResourceSkeleton variant="form" embedded />;
  if (!item) return <ResourceReadError error={error ?? { status: 404 }} refetch={refetch} isFetching={isFetching} />;

  const Content = skipMetadataUpdate
    ? Components.supportsHeaderRename
      ? EditFormPageWithRename
      : EditFormContent
    : EditFormPageWithMetadata;

  return (
    <>
      {!!error && <ResourceReadError error={error} refetch={refetch} isFetching={isFetching} stale />}
      <Content key={id} id={id!} item={item} Components={Components} type={type} />
    </>
  );
};

const EditFormPageWithRename = ({
  id,
  item,
  Components,
  type,
}: {
  id: string;
  item: RequiredFormFields;
  Components: any;
  type: ResourceType;
}) => {
  const { mutateAsync: renameResource } = useMutate(`rename${type}` as any);

  return <EditFormContent id={id} item={item} renameResource={renameResource} Components={Components} type={type} />;
};

const EditFormPageWithMetadata = ({
  id,
  item,
  Components,
  type,
}: {
  id: string;
  item: RequiredFormFields;
  Components: any;
  type: ResourceType;
}) => {
  const { mutateAsync: renameResource } = useMutate(`rename${type}` as any);
  const { mutateAsync: updateMetadata } = useMutate(`update${type}Metadata` as any);

  return (
    <EditFormContent
      id={id!}
      item={item}
      updateMetadata={updateMetadata}
      renameResource={renameResource}
      Components={Components}
      type={type}
    />
  );
};

const EditFormContent = ({
  id,
  item,
  updateMetadata,
  renameResource,
  Components,
  type,
}: {
  id: string;
  item: RequiredFormFields;
  updateMetadata?: (variables: { id: string; data: Partial<PatchResourceMetadataInput> }) => Promise<any>;
  renameResource?: (variables: { id: string; name: string }) => Promise<any>;
  Components: any;
  type: ResourceType;
}) => {
  const queryClient = useQueryClient();
  const [metadatChanged, setMetaDataChanged] = useState(false);

  const tabs = Components?.Tabs ?? [];
  const Header = Components?.Header;
  const localKey = `${type}-workload-${id}.active-tab`;
  const canWrite = hasCapability(item, 'canWrite');

  const invalidateRelatedQueries = () => queryClient.invalidateQueries();

  const handleUpdateMetadata = async (patch: Partial<PatchResourceMetadataInput>) => {
    if (!updateMetadata) return;
    await updateMetadata({ id, data: patch });
    await invalidateRelatedQueries();
    setMetaDataChanged(true);
  };

  const handleRenameResource = async (name: string) => {
    if (!renameResource) return;
    await renameResource({ id, name });
    await invalidateRelatedQueries();
    setMetaDataChanged(true);
  };

  return (
    <>
      <EditHeader
        type={type}
        canEditTitle={canWrite && Header.canEditTitle !== false}
        canEditDescription={canWrite && Header.canEditDescription !== false}
        item={item}
        Indicator={Header.Indicator}
        Tags={Header.Tags}
        Actions={Header.ActionButtons}
        onRename={(name) => handleRenameResource(name)}
        onChangeDescription={updateMetadata ? (description) => handleUpdateMetadata({ description }) : undefined}
      />
      {Components.SubHeader && <Components.SubHeader resource={item} />}
      <ResourceTabs localKey={localKey} resource={item} tabs={tabs} metadataChanged={metadatChanged} />
      <TaskSheet type={type} />
    </>
  );
};

const PageShell = ({ children }: { mode: 'add' | 'edit'; children: React.ReactNode }) => (
  <AppContent className="flex flex-col gap-(--section-gap)">{children}</AppContent>
);

const resourceLabel = (type: string) => capitalize(type.replace(/([a-z])([A-Z])/g, '$1 $2'));

const AddHeader = ({ type, title }: { type: ResourceType; title?: string }) => {
  const Icon = CitadelIcons[type] ?? Plus;
  const label = title ?? resourceLabel(type);
  return (
    <div className="rounded-lg border bg-card p-(--surface-padding) shadow-xs">
      <PageHeader
        title={`Add ${label}`}
        description={`Configure your ${label.toLowerCase()} using the sections below.`}
        icon={<Icon className="size-5" />}
      />
    </div>
  );
};

type EditHeaderProps<T> = {
  type: ResourceType;
  canEditTitle?: boolean;
  canEditDescription?: boolean;
  item: T;
  Indicator: React.ComponentType<{ resource: T }>;
  Tags?: React.ComponentType<{ resource: T }>;
  Actions: React.ComponentType<{ resource: T }>;
  onRename?: (name: string) => void;
  onChangeDescription?: (description: string) => void;
};

const EditHeader = <T extends RequiredFormFields>({
  type,
  canEditTitle = true,
  canEditDescription = true,
  item,
  Indicator,
  Tags,
  Actions,
  onRename,
  onChangeDescription,
}: EditHeaderProps<T>) => {
  const Icon = CitadelIcons[type] ?? Pencil;
  return (
    <header className="flex min-w-0 flex-col overflow-hidden rounded-lg border bg-card shadow-xs lg:flex-row lg:items-center">
      <div className="flex min-w-0 flex-1 items-start gap-4 p-(--surface-padding)">
        <span
          aria-hidden="true"
          className="flex size-11 shrink-0 items-center justify-center rounded-lg border border-primary/15 bg-primary/5 text-primary dark:text-foreground">
          <Icon className="size-5" />
        </span>
        <div className="flex min-w-0 flex-1 flex-col gap-1">
          <EditableTitle
            value={item.name}
            readOnly={!canEditTitle}
            onSave={onRename}
            status={<Indicator resource={item} />}
          />
          <EditableDescription
            readOnly={!canEditDescription}
            value={item.description ?? ''}
            onSave={onChangeDescription}
          />
          {Tags && (
            <div className="mt-2 flex min-w-0 flex-wrap items-center gap-2">
              <Tags resource={item} />
            </div>
          )}
        </div>
      </div>
      <div className="flex min-w-0 flex-wrap items-center gap-3 border-t bg-muted/10 px-(--surface-padding) py-3 lg:max-w-[45%] lg:shrink-0 lg:justify-end lg:border-t-0 lg:bg-transparent lg:py-(--surface-padding)">
        <Actions resource={item} />
      </div>
    </header>
  );
};

const useInlineEdit = (initial: string) => {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(initial);

  const value = editing ? draft : initial;

  const isDirty = draft.trim() !== initial.trim();

  const start = () => {
    setDraft(initial);
    setEditing(true);
  };

  const cancel = () => {
    setEditing(false);
  };

  const commit = async (onSave?: (v: string) => void) => {
    if (isDirty) {
      onSave?.(draft);
    }
    setEditing(false);
  };

  return { value, setValue: setDraft, editing, isDirty, start, cancel, commit };
};

const EditableTitle = ({
  value,
  readOnly,
  onSave,
  status,
}: {
  value: string;
  status: React.ReactNode;
  readOnly?: boolean;
  onSave?: (v: string) => void;
}) => {
  const edit = useInlineEdit(value);
  const inputRef = useRef<HTMLInputElement | null>(null);

  useEffect(() => {
    if (edit.editing) {
      inputRef.current?.focus();
    }
  }, [edit.editing]);

  if (!edit.editing)
    return (
      <div className="group/title flex w-fit min-w-0 max-w-full flex-wrap items-center gap-x-2 gap-y-2">
        <h1 className="min-w-0 max-w-full text-2xl font-semibold tracking-tight [overflow-wrap:anywhere]">
          {readOnly ? (
            value
          ) : (
            <button
              type="button"
              onClick={edit.start}
              className="max-w-full cursor-text rounded-sm text-left focus-visible:outline-2 focus-visible:outline-ring">
              {value}
            </button>
          )}
        </h1>

        <div
          className={
            readOnly
              ? 'flex shrink-0 items-center'
              : 'relative flex shrink-0 items-center pl-0 transition-[padding] duration-150 ease-out group-hover/title:pl-8 group-focus-within/title:pl-8 motion-reduce:transition-none'
          }>
          {!readOnly && (
            <Button
              type="button"
              size="icon-sm"
              variant="ghost"
              aria-label="Edit name"
              onClick={edit.start}
              className="pointer-events-none absolute left-0 size-7 -translate-x-1 opacity-0 transition-[opacity,transform] duration-150 ease-out group-hover/title:pointer-events-auto group-hover/title:translate-x-0 group-hover/title:opacity-100 group-focus-within/title:pointer-events-auto group-focus-within/title:translate-x-0 group-focus-within/title:opacity-100 motion-reduce:transition-none">
              <Pencil className="size-3.5" />
            </Button>
          )}
          {status}
        </div>
      </div>
    );

  return (
    <div className="flex min-w-0 flex-wrap items-center gap-2">
      <InlineEditActions
        dirty={edit.isDirty && edit.value.trim().length > 0}
        onSave={() => edit.commit(onSave)}
        onCancel={edit.cancel}>
        <FieldInput
          ref={inputRef}
          value={edit.value}
          onChange={edit.setValue}
          placeholder="Name"
          onKeyDown={(e) => {
            if (e.key === 'Enter') edit.commit(onSave);
            if (e.key === 'Escape') edit.cancel();
          }}
          className="h-7.5 text-md font-bold bg-transparent"
        />
      </InlineEditActions>
      {status}
    </div>
  );
};

const EditableDescription = ({
  value = '',
  readOnly,
  onSave,
}: {
  value?: string;
  readOnly: boolean;
  onSave?: (v: string) => void;
}) => {
  const edit = useInlineEdit(value);

  if (!edit.editing)
    return (
      <div className="group flex items-center gap-2 w-full min-w-0">
        <div
          className={`text-sm text-muted-foreground leading-relaxed [overflow-wrap:anywhere] focus:outline-none min-w-0 ${readOnly ? '' : 'cursor-text'}`}
          onClick={readOnly ? undefined : edit.start}
          onKeyDown={(e) => {
            if (readOnly) return;
            if (e.key === 'Enter' || e.key === ' ') {
              e.preventDefault();
              edit.start();
            }
          }}>
          {value || (readOnly ? 'No description' : 'Add a description…')}
        </div>
        {!readOnly && (
          <GhostIconButton aria-label="Edit description" onClick={edit.start}>
            <Pencil className="h-3.5 w-3.5" />
          </GhostIconButton>
        )}
      </div>
    );

  return (
    <InlineEditActions dirty={edit.isDirty} onSave={() => edit.commit(onSave)} onCancel={edit.cancel}>
      <Textarea
        value={edit.value}
        onChange={(e) => edit.setValue(e.target.value)}
        placeholder="Add a description…"
        className="resize-none focus-visible:ring-0"
        onKeyDown={(e) => {
          if (e.key === 'Escape') edit.cancel();
          if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) edit.commit(onSave);
        }}
      />
    </InlineEditActions>
  );
};

const GhostIconButton = ({ children, ...props }: React.ComponentProps<typeof Button>) => (
  <Button
    size="icon-sm"
    variant="ghost"
    className="opacity-0 group-hover:opacity-100 focus-visible:opacity-100 inline-flex hover:bg-transparent"
    {...props}>
    {children}
  </Button>
);

const InlineEditActions = ({
  dirty,
  onSave,
  onCancel,
  children,
}: {
  dirty: boolean;
  onSave: () => void;
  onCancel: () => void;
  children: React.ReactNode;
}) => (
  <div className="flex gap-2 w-full">
    {children}
    <Button
      size="icon-sm"
      className="h-7.25"
      variant="outline"
      onClick={dirty ? onSave : onCancel}
      title={`${dirty ? 'Save' : 'Cancel'}`}>
      {dirty ? <Save className="h-3.5 w-3.5" xlinkTitle="Save" /> : <X className="h-3.5 w-3.5" />}
    </Button>
  </div>
);
