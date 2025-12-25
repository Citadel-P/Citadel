import { useEffect, useState } from 'react';
import { useParams } from 'react-router';
import { Pencil, Plus, Save, X } from 'lucide-react';

import { ResourceFormComponents } from '@/features';
import { useMutate, useResourceParamType } from '@/lib/hooks';
import { capitalize } from '@/lib/utils';

import { FieldInput } from '@/components/custom/form-builder';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';

import NotFound from './not-found';
import { RequiredFormFields } from './types';

export const ResourceFormPage = ({ mode }: { mode: 'add' | 'edit' }) => {
  const type = useResourceParamType();
  const { id } = useParams();

  const { mutateAsync: updateResource } = useMutate(`update${type}` as any);

  const Components = ResourceFormComponents[type];
  const formData = Components?.useFormData?.(id);

  const item = formData?.item;
  const isLoading = formData?.isLoading;

  const [resource, setResource] = useState<RequiredFormFields | null>(null);

  useEffect(() => {
    if (item) {
      setResource(item);
    }
  }, [item]);

  if (!type) return <NotFound />;
  if (!Components?.Form) return <NotFound />;

  if (mode === 'edit' && (isLoading || !resource)) {
    return null;
  }

  const updateField = async (patch: Partial<RequiredFormFields>) => {
    if (!resource) return;
    const previous = resource;
    setResource({ ...resource, ...patch });
    try {
      await updateResource({ id, data: patch });
    } catch {
      setResource(previous);
    }
  };

  return (
    <PageShell>
      {mode === 'add' ? (
        <AddHeader type={type} />
      ) : (
        <EditHeader
          item={resource}
          Indicator={Components.Header.Indicator}
          Actions={Components.Header.ActionButtons}
          onRename={(name) => updateField({ name })}
          onChangeDescription={(description) => updateField({ description })}
        />
      )}

      <Components.Form mode={mode} resource={resource} />
    </PageShell>
  );
};

const PageShell = ({ children }: { children: React.ReactNode }) => (
  <div className="px-4 py-4 lg:container sm:px-6 mx-auto">
    <div className="w-full rounded-lg border bg-background p-4 flex flex-col gap-6">{children}</div>
  </div>
);

const AddHeader = ({ type }: { type: string }) => (
  <div className="flex items-center gap-2">
    <div className="inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">
      <Plus className="h-4 w-4" />
    </div>
    <h1 className="text-md font-bold">Add {capitalize(type)}</h1>
  </div>
);

type EditHeaderProps<T> = {
  item: T;
  Indicator: React.ComponentType<{ resource: T }>;
  Actions: React.ComponentType<{ resource: T }>;
  onRename: (name: string) => void;
  onChangeDescription: (description: string) => void;
};

const EditHeader = <T extends RequiredFormFields>({
  item,
  Indicator,
  Actions,
  onRename,
  onChangeDescription,
}: EditHeaderProps<T>) => (
  <div className="flex flex-col sm:flex-row gap-4 items-start">
    <div className="flex items-center gap-2 flex-1 min-w-0">
      <Indicator resource={item} />
      <div className="flex flex-col flex-1 min-w-0">
        <EditableTitle value={item.name} onSave={onRename} />
        <EditableDescription value={item.description} onSave={onChangeDescription} />
      </div>
    </div>
    <div className="flex gap-4 items-center flex-wrap shrink-0">
      <Actions resource={item} />
    </div>
  </div>
);

const useInlineEdit = (initial: string) => {
  const [editing, setEditing] = useState(false);
  const [value, setValue] = useState(initial);

  useEffect(() => {
    if (!editing) setValue(initial);
  }, [initial, editing]);

  const isDirty = value.trim() !== initial.trim();

  const start = () => setEditing(true);
  const cancel = () => {
    setValue(initial);
    setEditing(false);
  };
  const commit = async (onSave?: (v: string) => void) => {
    if (isDirty) onSave?.(value);
    setEditing(false);
  };

  return { value, setValue, editing, isDirty, start, cancel, commit };
};

const EditableTitle = ({ value, onSave }: { value: string; onSave: (v: string) => void }) => {
  const edit = useInlineEdit(value);

  if (!edit.editing)
    return (
      <div className="group flex items-center gap-1 min-w-0">
        <div
          className="text-md font-bold truncate cursor-text focus:outline-none"
          onClick={edit.start}
          onKeyDown={(e) => {
            if (e.key === 'Enter' || e.key === ' ') {
              e.preventDefault();
              edit.start();
            }
          }}>
          {value}
        </div>

        <GhostIconButton onClick={edit.start}>
          <Pencil className="h-3.5 w-3.5" />
        </GhostIconButton>
      </div>
    );

  return (
    <InlineEditActions
      dirty={edit.isDirty && edit.value.trim().length > 0}
      onSave={() => edit.commit(onSave)}
      onCancel={edit.cancel}>
      <FieldInput
        autoFocus
        value={edit.value}
        onChange={edit.setValue}
        placeholder="Name"
        onKeyDown={(e) => {
          if (e.key === 'Enter') edit.commit(onSave);
          if (e.key === 'Escape') edit.cancel();
        }}
        className="h-[30px] text-md font-bold bg-transparent"
      />
    </InlineEditActions>
  );
};

const EditableDescription = ({ value = '', onSave }: { value?: string; onSave?: (v: string) => void }) => {
  const edit = useInlineEdit(value);

  if (!edit.editing)
    return (
      <div className="group flex items-center gap-2 w-full">
        <div
          className="text-sm text-muted-foreground leading-relaxed truncate cursor-text focus:outline-none"
          onClick={edit.start}
          onKeyDown={(e) => {
            if (e.key === 'Enter' || e.key === ' ') {
              e.preventDefault();
              edit.start();
            }
          }}>
          {value || '--'}
        </div>
        <GhostIconButton onClick={edit.start}>
          <Pencil className="h-3.5 w-3.5" />
        </GhostIconButton>
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
    variant="ghost"
    className="opacity-0 group-hover:opacity-100 p-0 h-auto w-auto min-h-0 min-w-0 inline-flex hover:bg-transparent"
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
    <Button size="icon-sm" variant="ghost" onClick={dirty ? onSave : onCancel}>
      {dirty ? <Save className="h-4 w-4" /> : <X className="h-4 w-4" />}
    </Button>
  </div>
);
