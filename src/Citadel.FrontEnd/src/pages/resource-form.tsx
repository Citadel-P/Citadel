import { useEffect, useRef, useState } from 'react';
import { useParams } from 'react-router';
import { Pencil, Plus, Save, X } from 'lucide-react';
import { useQueryClient } from '@tanstack/react-query';

import { ResourceFormComponents } from '@/features';
import { useMutate, useResourceParamType } from '@/lib/hooks';
import { capitalize } from '@/lib/utils';

import { FieldInput } from '@/components/custom/form-builder';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';

import NotFound from './not-found';
import { RequiredFormFields } from './types';
import { ResourceType } from '@/api/types';
import Loader from '@/components/ui/loader';
import { ResourceTabs } from '@/components/custom/resource-tabs';
import TaskSheet from '@/components/custom/task-sheet';
import { PatchResourceMetadata } from '@/api/generated/api.types';
import { hasCapability } from '@/lib/resource-capabilities';

export const ResourceForm = ({ mode }: { mode: 'add' | 'edit' }) => {
  const { type, tab } = useResourceParamType();
  if (!type) return <NotFound />;

  const resolvedType = tab ?? type;
  const formComponents = ResourceFormComponents[resolvedType];

  return (
    <PageShell mode={mode}>
      {mode === 'add' ? (
        <AddFormPage type={resolvedType} />
      ) : (
        <EditFormPage type={resolvedType} skipMetadataUpdate={formComponents?.EditForm?.skipMetadataUpdate} />
      )}
    </PageShell>
  );
};

const AddFormPage = ({ type }: { type: ResourceType }) => {
  const Components = ResourceFormComponents[type]?.AddForm;

  if (!Components?.Content) return <NotFound />;

  return (
    <>
      <AddHeader type={type} title={Components.Header?.title} />
      <Components.Content />
    </>
  );
};

const EditFormPage = ({ type, skipMetadataUpdate = false }: { type: ResourceType; skipMetadataUpdate?: boolean }) => {
  const { id } = useParams();
  const { mutateAsync: renameResource } = useMutate(`rename${type}` as any);

  const Components = ResourceFormComponents[type]?.EditForm;
  const formData = Components?.useData?.(id!);

  const { item, isLoading } = formData ?? {};

  if (isLoading || !item) return <Loader />;

  const Content = skipMetadataUpdate ? EditFormContent : EditFormPageWithMetadata;

  return <Content key={id} id={id!} item={item} renameResource={renameResource} Components={Components} type={type} />;
};

const EditFormPageWithMetadata = ({
  id,
  item,
  renameResource,
  Components,
  type,
}: {
  id: string;
  item: RequiredFormFields;
  renameResource: (variables: { id: string; name: string }) => Promise<any>;
  Components: any;
  type: ResourceType;
}) => {
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
  updateMetadata?: (variables: { id: string; data: Partial<PatchResourceMetadata> }) => Promise<any>;
  renameResource: (variables: { id: string; name: string }) => Promise<any>;
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

  const handleUpdateMetadata = async (patch: Partial<PatchResourceMetadata>) => {
    if (!updateMetadata) return;
    await updateMetadata({ id, data: patch });
    await invalidateRelatedQueries();
    setMetaDataChanged(true);
  };

  const handleRenameResource = async (name: string) => {
    await renameResource({ id, name });
    await invalidateRelatedQueries();
    setMetaDataChanged(true);
  };

  return (
    <>
      <EditHeader
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

const PageShell = ({ mode, children }: { mode: 'add' | 'edit'; children: React.ReactNode }) => (
  <div className="px-4 py-4 lg:container sm:px-6 mx-auto">
    <div className={`w-full rounded-lg border bg-background p-4 flex flex-col ${mode === 'add' ? 'gap-6' : 'gap-0.5'}`}>
      {children}
    </div>
  </div>
);

const AddHeader = ({ type, title }: { type: string; title?: string }) => (
  <div className="flex items-center gap-2">
    <div className="inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">
      <Plus className="h-4 w-4" />
    </div>
    <h1 className="text-md font-bold">Add {title ?? capitalize(type)}</h1>
  </div>
);

type EditHeaderProps<T> = {
  canEditTitle?: boolean;
  canEditDescription?: boolean;
  item: T;
  Indicator: React.ComponentType<{ resource: T }>;
  Tags?: React.ComponentType<{ resource: T }>;
  Actions: React.ComponentType<{ resource: T }>;
  onRename: (name: string) => void;
  onChangeDescription?: (description: string) => void;
};

const EditHeader = <T extends RequiredFormFields>({
  canEditTitle = true,
  canEditDescription = true,
  item,
  Indicator,
  Tags,
  Actions,
  onRename,
  onChangeDescription,
}: EditHeaderProps<T>) => (
  <div className="flex flex-col sm:flex-row gap-4 items-start">
    <div className="flex items-center gap-2 flex-1 min-w-0 w-full">
      <Indicator resource={item} />
      <div className="flex flex-col flex-1 min-w-0">
        <EditableTitle value={item.name} readOnly={!canEditTitle} onSave={onRename} />
        <EditableDescription
          readOnly={!canEditDescription}
          value={item.description ?? ''}
          onSave={onChangeDescription}
        />
      </div>
    </div>
    <div className="flex gap-4 items-center flex-wrap shrink-0">
      {Tags && <Tags resource={item} />}
      <Actions resource={item} />
    </div>
  </div>
);

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
}: {
  value: string;
  readOnly?: boolean;
  onSave: (v: string) => void;
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
      <div className="group flex items-center gap-1 min-w-0">
        <div
          className={`text-md font-bold truncate focus:outline-none ${readOnly ? '' : 'cursor-text'}`}
          onClick={readOnly ? undefined : edit.start}
          onKeyDown={(e) => {
            if (readOnly) return;
            if (e.key === 'Enter' || e.key === ' ') {
              e.preventDefault();
              edit.start();
            }
          }}>
          {value}
        </div>

        {!readOnly && (
          <GhostIconButton onClick={edit.start}>
            <Pencil className="h-3.5 w-3.5" />
          </GhostIconButton>
        )}
      </div>
    );

  return (
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
          className={`text-sm text-muted-foreground leading-relaxed truncate focus:outline-none min-w-0 ${readOnly ? '' : 'cursor-text'}`}
          onClick={readOnly ? undefined : edit.start}
          onKeyDown={(e) => {
            if (readOnly) return;
            if (e.key === 'Enter' || e.key === ' ') {
              e.preventDefault();
              edit.start();
            }
          }}>
          {value || '--'}
        </div>
        {!readOnly && (
          <GhostIconButton onClick={edit.start}>
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
    className="opacity-0 group-hover:opacity-100 inline-flex hover:bg-transparent"
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
