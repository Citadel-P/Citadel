import { ResourceDetailHeader } from '@/components/custom/resource-detail-header';
import { InlineTitleLabel } from '@/components/custom/inline-title-label';
import type { KnownResourceName } from '@/api/types';
import { canShowCachedResource } from '@/lib/request-error';
import { ResourceReadError } from '@/components/custom/resource-read-error';
import { AppContent } from '@/components/custom/app-content';
import { BackToTop } from '@/components/custom/back-to-top';
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
import { MarkdownDescription, MarkdownDescriptionEditor } from '@/components/custom/markdown-description';

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
      {Components.Navigation && <Components.Navigation />}
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
    return <ResourceReadError page error={error} refetch={refetch} isFetching={isFetching} />;
  if (!item && isLoading) return <ResourceSkeleton variant="form" embedded />;
  if (!item)
    return <ResourceReadError page error={error ?? { status: 404 }} refetch={refetch} isFetching={isFetching} />;

  const Content = skipMetadataUpdate
    ? Components.supportsHeaderRename
      ? EditFormPageWithRename
      : EditFormContent
    : EditFormPageWithMetadata;

  return (
    <>
      {!!error && <ResourceReadError page error={error} refetch={refetch} isFetching={isFetching} stale />}
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
  const { mutateAsync: renameResource } = useMutate(`rename${type}` as Extract<KnownResourceName, `rename${string}`>);

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
  const { mutateAsync: renameResource } = useMutate(`rename${type}` as Extract<KnownResourceName, `rename${string}`>);
  const { mutateAsync: updateMetadata } = useMutate(
    `update${type}Metadata` as Extract<KnownResourceName, `update${string}Metadata`>,
  );

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
  renameResource?: (variables: { data: { id: string; name: string } }) => Promise<any>;
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
    await renameResource({ data: { id, name } });
    await invalidateRelatedQueries();
    setMetaDataChanged(true);
  };

  return (
    <>
      {Components.Navigation && <Components.Navigation />}
      <EditHeader
        type={type}
        canEditTitle={canWrite && Header.canEditTitle !== false}
        canEditDescription={canWrite && Header.canEditDescription !== false}
        showDescription={Header.showDescription}
        item={item}
        Indicator={Header.Indicator}
        Tags={Header.Tags}
        Actions={Header.ActionButtons}
        onRename={(name) => handleRenameResource(name)}
        onChangeDescription={updateMetadata ? (description) => handleUpdateMetadata({ description }) : undefined}
      />
      {Components.SubHeader && <Components.SubHeader resource={item} />}
      <ResourceTabs
        localKey={localKey}
        resource={item}
        tabs={tabs}
        metadataChanged={metadatChanged}
        actions={Components.TabActions && <Components.TabActions resource={item} />}
      />
      <TaskSheet type={type} />
    </>
  );
};

const PageShell = ({ children }: { mode: 'add' | 'edit'; children: React.ReactNode }) => (
  <AppContent className="flex flex-col gap-(--section-gap)">
    {children}
    <BackToTop />
  </AppContent>
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
  showDescription?: boolean;
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
  showDescription = true,
  item,
  Indicator,
  Tags,
  Actions,
  onRename,
  onChangeDescription,
}: EditHeaderProps<T>) => {
  const Icon = CitadelIcons[type] ?? Pencil;
  return (
    <ResourceDetailHeader icon={<Icon className="size-5" />} actions={<Actions resource={item} />}>
      <EditableTitle
        value={item.name}
        readOnly={!canEditTitle}
        onSave={onRename}
        status={<Indicator resource={item} />}
      />
      {showDescription && (
        <EditableDescription
          readOnly={!canEditDescription}
          value={item.description ?? ''}
          onSave={onChangeDescription}
        />
      )}
      {Tags && (
        <div className="mt-2 flex min-w-0 flex-wrap items-center gap-2">
          <Tags resource={item} />
        </div>
      )}
    </ResourceDetailHeader>
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
    return <InlineTitleLabel value={value} status={status} onEdit={readOnly ? undefined : edit.start} />;

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

export const EditableDescription = ({
  value = '',
  maxLength,
  readOnly,
  onSave,
}: {
  value?: string;
  maxLength?: number;
  readOnly: boolean;
  onSave?: (v: string) => void;
}) => {
  const edit = useInlineEdit(value);

  if (!edit.editing)
    return (
      <div className="group flex items-center gap-2 w-full min-w-0">
        <div
          className={`text-sm text-muted-foreground leading-relaxed [overflow-wrap:anywhere] focus:outline-none min-w-0 ${readOnly ? '' : 'cursor-text'}`}
          onClick={
            readOnly
              ? undefined
              : (e) => {
                  if (!(e.target instanceof Element && e.target.closest('a'))) edit.start();
                }
          }
          onKeyDown={(e) => {
            if (readOnly || e.target !== e.currentTarget) return;
            if (e.key === 'Enter' || e.key === ' ') {
              e.preventDefault();
              edit.start();
            }
          }}>
          {value ? (
            <MarkdownDescription>{value}</MarkdownDescription>
          ) : readOnly ? (
            'No description'
          ) : (
            'Add a description…'
          )}
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
      <MarkdownDescriptionEditor
        aria-label="Description"
        maxLength={maxLength}
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
