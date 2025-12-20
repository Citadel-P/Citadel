import React, { useMemo, useState, useCallback, useEffect, useRef, memo } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Loader2, Eye, History, Save, X } from 'lucide-react';
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { MonacoDiff } from '@/lib/monaco';
import { fromNow } from '@/lib/dayjs.helper';
import { Switch } from '../ui/switch';
import { Label } from '../ui/label';
import { cn } from '@/lib/utils';
import { Textarea } from '../ui/textarea';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '../ui/select';

type Primitive = string | number | boolean | bigint | symbol | null | undefined | Date;

// Produces: "name" | "type" | "configuration" | "configuration.pat" | ...
export type Path<T> = T extends Primitive
  ? never
  : {
      [K in keyof T & string]: T[K] extends Primitive ? K : T[K] extends Array<any> ? K : K | `${K}.${Path<T[K]>}`;
    }[keyof T & string];

export interface FieldConfig<T> {
  key: Path<T>;
  label: string;
  required?: boolean;
  placeholder?: string;
  description?: React.ReactElement | string;
  disabled?: boolean;
  validate?: (value: any) => string | null;
  render: (value: any, set: FieldChange<T>) => React.ReactNode;
}

export interface FieldItemConfig<T> {
  kind: 'field';
  field: FieldConfig<T>;
}

export interface RowFieldConfig<T> {
  kind: 'row';
  id: string;
  fields: FieldConfig<T>[];
  gap?: string;
  className?: string;
}

export interface GroupFieldConfig<T> {
  kind: 'group';
  id: string;
  label: string;
  description?: React.ReactElement | string;
  items: Array<FieldItemConfig<T> | RowFieldConfig<T>>;
  direction?: 'vertical' | 'horizontal';
}

export interface SectionConfig<T> {
  title?: string;
  items: SectionItemConfig<T>[];
}

export type FormSchema<T> = Record<string, SectionConfig<T>>;
export type SectionItemConfig<T> = FieldItemConfig<T> | GroupFieldConfig<T> | RowFieldConfig<T>;
export type FieldChange<T> = (partial: Partial<T> | ((prev: Partial<T>) => Partial<T>)) => void;

interface FieldShellProps {
  label: string;
  required?: boolean;
  description?: React.ReactElement | string;
  edited: boolean;
  error?: string | null;
  touched: boolean;
  children: React.ReactNode;
}

type DraftInfo = {
  hasDraft: boolean;
  savedAt?: string;
};

interface StoredDraft<T> {
  version?: string | number;
  update: Partial<T>;
  savedAt: string;
}

/* -------------------------------------------------------------------------- */
/*                               Schema helpers                               */
/* -------------------------------------------------------------------------- */

export function defineField<T, K extends Path<T>>(
  config: Omit<FieldConfig<T>, 'key'> & { key: K },
): FieldItemConfig<T> {
  return {
    kind: 'field',
    field: config,
  };
}

export function defineRowField<T>(config: {
  id: string;
  fields: Array<FieldItemConfig<T> | FieldConfig<T>>;
  gap?: string;
  className?: string;
}): RowFieldConfig<T> {
  const normalizedFields = config.fields.map((f) =>
    'kind' in f && f.kind === 'field' ? f.field : (f as FieldConfig<T>),
  );
  return {
    kind: 'row',
    id: config.id,
    fields: normalizedFields,
    gap: config.gap,
    className: config.className,
  };
}

export function defineGroupField<T>(config: {
  id: string;
  label: string;
  description?: React.ReactElement | string;
  items: Array<FieldItemConfig<T> | RowFieldConfig<T>>;
  direction?: 'vertical' | 'horizontal';
}): GroupFieldConfig<T> {
  return {
    kind: 'group',
    id: config.id,
    label: config.label,
    description: config.description,
    items: config.items,
    direction: config.direction ?? 'vertical',
  };
}

export function defineSection<T>(config: SectionConfig<T>): SectionConfig<T> {
  return config;
}

/* -------------------------------------------------------------------------- */
/*                               Shell components                             */
/* -------------------------------------------------------------------------- */

function FieldShell({ label, required, description, edited, error, touched, children }: FieldShellProps) {
  const showError = touched && error;

  return (
    <div className="flex flex-col gap-4">
      <div>
        <label className="block font-medium text-sm">
          {label}
          {required && <span className="text-destructive ml-1">*</span>}
        </label>
        {typeof description === 'string' ? <p className="text-sm text-muted-foreground">{description}</p> : description}
      </div>

      <div className="relative">
        {edited && <span className="absolute -top-0 right-1 text-[10px] text-primary bg-background px-1">Edited</span>}
        {children}
      </div>

      {showError && <p className="text-xs text-destructive mt-1">{error}</p>}
    </div>
  );
}

// Memoized wrapper to prevent re-rendering fields when unrelated fields change.
interface SmartFieldProps<T> extends Omit<FieldShellProps, 'children'> {
  render: (value: any, set: FieldChange<T>) => React.ReactNode;
  value: any;
  onChange: FieldChange<T>;
}

function SmartFieldImpl<T>({ render, value, onChange, ...props }: SmartFieldProps<T>) {
  return <FieldShell {...props}>{render(value, onChange)}</FieldShell>;
}

const SmartField = memo(SmartFieldImpl) as typeof SmartFieldImpl;

export const FieldInput = ({
  value,
  onChange,
  placeholder,
  type,
  disabled,
  className,
}: {
  value?: string;
  onChange: (v: string) => void;
  placeholder?: string;
  type?: string;
  disabled?: boolean;
  className?: string;
}) => (
  <Input
    disabled={disabled}
    type={type}
    value={value ?? ''}
    onChange={(e) => onChange(e.target.value)}
    placeholder={placeholder}
    className={cn('max-w-[400px] max-h-[36px]', className)}
  />
);

export const FieldTextArea = ({
  value,
  onChange,
  placeholder,
  disabled,
}: {
  value?: string;
  onChange: (v: string) => void;
  placeholder?: string;
  type?: string;
  disabled?: boolean;
}) => (
  <Textarea
    disabled={disabled}
    value={value ?? ''}
    onChange={(e) => onChange(e.target.value)}
    placeholder={placeholder}
    className="max-w-full focus-visible:ring-0"
  />
);

export const FieldSwitch = ({
  checked,
  id,
  onChange,
}: {
  checked: boolean;
  id: string;
  onChange: (v: boolean) => void;
}) => (
  <div className="flex items-center gap-2">
    <Switch checked={checked ?? false} id={id} onCheckedChange={onChange} />

    <Label
      htmlFor={id}
      className={cn('text-sm transition-colors font-normal', checked ? 'text-green-600/80' : 'text-muted-foreground')}>
      {checked ? 'On' : 'Off'}
    </Label>
  </div>
);

export function PortMappingField({
  set,
  ports,
  disabled,
}: {
  set: (v: string[]) => void;
  ports: string[];
  disabled?: boolean;
}) {
  return (
    <div className="col-span-2 ">
      <div className="space-y-2 flex flex-col gap-2">
        {ports.length === 0 && <span className="text-xs text-muted">No ports exposed in this image</span>}

        {ports.map((value, idx) => {
          const hasMapping = value.includes('-');
          const [hostPort, containerPort] = hasMapping ? value.split('-') : ['', value];

          return (
            <div
              key={idx}
              className="flex flex-col sm:flex-row gap-2 items-start sm:items-center max-w-[400px] max-h-[36px]">
              <div className="flex flex-1 w-full">
                <Input
                  placeholder="Host port"
                  className="rounded-r-none! focus-visible:ring-transparent"
                  type="number"
                  min="0"
                  max="65535"
                  value={hostPort}
                  disabled={disabled}
                  onChange={(e) => {
                    const newHost = e.target.value;
                    const updated = [...ports];

                    updated[idx] = newHost ? `${newHost}-${containerPort}` : containerPort;

                    set(updated);
                  }}
                />

                <span className="flex z-10 items-center justify-center w-[80px] shadow-xs flex-shrink-0 bg-accent/60 border-r rounded-r-sm border-y border-border text-xs">
                  :{containerPort}
                </span>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}

export function InputGroupField({
  value,
  onChange,
  prefixPlaceholder,
  suffixPlaceholder,
  type,
  disabled,
  className,
}: {
  value?: string;
  onChange: (v: string) => void;
  prefixPlaceholder?: string;
  suffixPlaceholder?: string;
  type?: string;
  disabled?: boolean;
  className?: string;
}) {
  return (
    <div className="col-span-2 ">
      <div className="space-y-2 flex flex-col gap-2">
        <div
          className={cn(
            'flex flex-col sm:flex-row gap-2 items-start sm:items-center max-w-[400px] max-h-[36px]',
            className,
          )}>
          <div className="flex flex-1 w-full">
            <span className="flex z-10 items-center justify-center w-[80px] shadow-xs flex-shrink-0 bg-accent/60 border-l rounded-l-sm border-y border-border text-xs">
              {prefixPlaceholder}
            </span>
            <Input
              placeholder={suffixPlaceholder}
              className="rounded-l-none! focus-visible:ring-transparent"
              type={type}
              value={value}
              disabled={disabled}
              onChange={(e) => onChange(e.target.value)}
            />
          </div>
        </div>
      </div>
    </div>
  );
}

export function ItemSelector({
  value,
  onChange,
  disabled,
  collection,
}: {
  value?: string;
  onChange: (v: any) => void;
  disabled?: boolean;
  collection: Record<string, { label: string; description?: string }>;
}) {
  const finalValue = value ?? Object.values(collection)[0]?.label;
  const selected = collection[finalValue as keyof typeof collection];
  return (
    <Select value={finalValue} onValueChange={onChange} disabled={disabled}>
      <SelectTrigger className="w-full max-w-[400px]">
        <SelectValue>
          {selected ? (
            <div className="flex items-center gap-2">
              <span>{selected.label}</span>
            </div>
          ) : (
            'Select a value...'
          )}
        </SelectValue>
      </SelectTrigger>

      <SelectContent className="bg-background">
        {Object.entries(collection).map(([key, info]) => (
          <SelectItem key={key} value={key}>
            <div className="flex items-center gap-2">
              <div className="flex flex-col">
                <span className="font-medium">{info.label}</span>
                <span className="text-xs text-muted-foreground">{info.description}</span>
              </div>
            </div>
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}

/* -------------------------------------------------------------------------- */
/*                                 Form logic                                 */
/* -------------------------------------------------------------------------- */

function getValue(obj: any, path: string): any {
  return path.split('.').reduce((acc, key) => acc?.[key], obj);
}

function isObject(item: any): boolean {
  return item && typeof item === 'object' && !Array.isArray(item);
}

function deepMerge<T>(target: any, source: any): T {
  if (!isObject(target) || !isObject(source)) return (source ?? target) as T;

  const output: any = Array.isArray(target) ? [...target] : { ...target };

  for (const key of Object.keys(source)) {
    const prev = target[key];
    const next = source[key];

    if (isObject(prev) && isObject(next)) {
      output[key] = deepMerge(prev, next);
    } else {
      output[key] = next;
    }
  }

  return output as T;
}

function extractFieldMap<T>(schema: FormSchema<T>): Record<string, FieldConfig<T>> {
  const map: Record<string, FieldConfig<T>> = {};

  for (const sectionKey of Object.keys(schema)) {
    const section = schema[sectionKey];
    for (const item of section.items) {
      if (item.kind === 'field') {
        map[item.field.key as string] = item.field;
      } else if (item.kind === 'row') {
        for (const f of item.fields) {
          map[f.key as string] = f;
        }
      } else if (item.kind === 'group') {
        for (const sub of item.items) {
          if (sub.kind === 'field') {
            map[sub.field.key as string] = sub.field;
          } else if (sub.kind === 'row') {
            for (const f of sub.fields) {
              map[f.key as string] = f;
            }
          }
        }
      }
    }
  }

  return map;
}

function computeValidationState<T>(
  original: T,
  merged: T,
  fieldMap: Record<string, FieldConfig<T>>,
): {
  errors: Record<string, string | null>;
  dirty: Record<string, boolean>;
} {
  const errors: Record<string, string | null> = {};
  const dirty: Record<string, boolean> = {};

  for (const [key, field] of Object.entries(fieldMap)) {
    const val = getValue(merged, key);
    const originalVal = getValue(original, key);

    dirty[key] = val !== originalVal;

    const isEmpty = val === undefined || val === '';
    let err: string | null = null;

    if (field.required && isEmpty) {
      err = 'Required';
    } else if (field.validate) {
      err = field.validate(val);
    }

    errors[key] = err;
  }

  return { errors, dirty };
}

/* ------------------------------ Draft helpers ----------------------------- */

function loadDraft<T>(key: string, expectedVersion?: string | number): StoredDraft<T> | null {
  try {
    const raw = localStorage.getItem(key);
    if (!raw) return null;

    const parsed = JSON.parse(raw) as StoredDraft<T>;
    if (!parsed || typeof parsed !== 'object') return null;

    if (expectedVersion !== undefined && parsed.version !== expectedVersion) {
      return null;
    }

    if (!parsed.update || typeof parsed.update !== 'object') return null;

    return parsed;
  } catch {
    return null;
  }
}

function persistDraft<T>(key: string, version: string | number | undefined, update: Partial<T>): string | undefined {
  try {
    const savedAt = new Date().toISOString();
    const payload: StoredDraft<T> = {
      version,
      update,
      savedAt,
    };
    localStorage.setItem(key, JSON.stringify(payload));
    return savedAt;
  } catch {
    return undefined;
  }
}

function clearDraft(key: string) {
  try {
    localStorage.removeItem(key);
  } catch {
    // ignore
  }
}

/* -------------------------------------------------------------------------- */
/*                                 FormShell                                  */
/* -------------------------------------------------------------------------- */

export function FormShell<T>({
  title,
  schema,
  original,
  update,
  setUpdate,
  onSave,
  disabled,
  mode = 'edit',
  pending = false,
  draftKey,
  draftVersion,
  onReset,
}: {
  title?: string;
  schema: FormSchema<T>;
  original: T;
  update: Partial<T>;
  setUpdate: React.Dispatch<React.SetStateAction<Partial<T>>>;
  onSave: (payload: T) => Promise<void>;
  disabled?: boolean;
  mode?: 'add' | 'edit';
  pending?: boolean;
  draftKey?: string;
  draftVersion?: string | number;
  onReset?: () => void;
}) {
  const updateRef = React.useRef(update);
  updateRef.current = update;

  const [touched, setTouched] = useState<Record<string, boolean>>({});
  const [previewOpen, setPreviewOpen] = useState(false);

  const [draftInfo, setDraftInfo] = useState<DraftInfo>({ hasDraft: false });
  const [draftLoadedBanner, setDraftLoadedBanner] = useState(false);

  const sections = Object.keys(schema);

  const fieldMap = useMemo(() => extractFieldMap(schema), [schema]);
  const merged = useMemo(() => deepMerge<T>(original, update), [original, update]);

  const { errors, dirty } = useMemo(
    () => computeValidationState(original, merged, fieldMap),
    [original, merged, fieldMap],
  );

  const hasChanges = useMemo(() => Object.values(dirty).some(Boolean), [dirty]);
  const isValid = useMemo(() => Object.values(errors).every((v) => !v), [errors]);
  const canSave = hasChanges;

  const lastSavedAtLabel = draftInfo.savedAt ? fromNow(draftInfo.savedAt as any) : undefined;

  /* ------------------------- Load draft on first mount ------------------------- */
  useEffect(() => {
    if (!draftKey) return;
    if (typeof window === 'undefined') return;

    const stored = loadDraft<Partial<T>>(draftKey, draftVersion);
    if (!stored || !stored.update) return;

    setUpdate((prev) => deepMerge<Partial<T>>(prev ?? {}, stored.update));
    setDraftInfo({ hasDraft: true, savedAt: stored.savedAt });
    setDraftLoadedBanner(true);
  }, [draftKey, draftVersion, setUpdate]);

  /* ----------------------------- Field change API ----------------------------- */
  const handleChange = useCallback(
    (key: string, partialOrUpdater: Partial<T> | ((prev: Partial<T>) => Partial<T>)) => {
      if (disabled) return;

      setTouched((prev) => {
        if (prev[key]) return prev;
        return { ...prev, [key]: true };
      });

      const current = updateRef.current ?? {};
      const resolved = typeof partialOrUpdater === 'function' ? partialOrUpdater(current) : partialOrUpdater;
      const next = deepMerge<Partial<T>>(current, resolved);

      setUpdate(next);

      if (draftKey && typeof window !== 'undefined') {
        const savedAt = persistDraft<Partial<T>>(draftKey, draftVersion, next);
        if (savedAt) {
          setDraftInfo({ hasDraft: true, savedAt });
        }
      }
    },
    [disabled, setUpdate, draftKey, draftVersion],
  );

  // Use a Ref for handleChange to ensure the cached handlers always call the latest version
  // without needing to be re-created themselves.
  const handleChangeRef = useRef(handleChange);
  handleChangeRef.current = handleChange;

  const fieldHandlersRef = useRef<Map<string, FieldChange<T>>>(new Map());

  const getFieldHandler = useCallback((key: string) => {
    if (!fieldHandlersRef.current.has(key)) {
      fieldHandlersRef.current.set(key, (partialOrUpdater) => {
        handleChangeRef.current(key, partialOrUpdater);
      });
    }
    return fieldHandlersRef.current.get(key)!;
  }, []);

  const reset = useCallback(() => {
    if (onReset) {
      onReset();
    } else {
      setUpdate({});
    }
    setTouched({});
    if (draftKey && typeof window !== 'undefined') {
      clearDraft(draftKey);
      setDraftInfo({ hasDraft: false, savedAt: undefined });
      setDraftLoadedBanner(false);
    }
  }, [onReset, setUpdate, draftKey]);

  const validateAll = useCallback(
    (value: T) => {
      const { errors: allErrors } = computeValidationState(original, value, fieldMap);
      const valid = Object.values(allErrors).every((v) => !v);

      // mark all fields as touched so errors become visible
      setTouched((prev) => {
        const next: Record<string, boolean> = { ...prev };
        for (const key of Object.keys(fieldMap)) {
          next[key] = true;
        }
        return next;
      });

      return valid;
    },
    [original, fieldMap],
  );

  const confirm = useCallback(async () => {
    const valid = validateAll(merged);
    if (!valid) return;

    await onSave(merged as T);

    if (draftKey && typeof window !== 'undefined') {
      clearDraft(draftKey);
      setDraftInfo({ hasDraft: false, savedAt: undefined });
      setDraftLoadedBanner(false);
    }
  }, [validateAll, merged, onSave, draftKey]);

  /* -------------------------------------------------------------------------- */
  /*                                    UI                                     */
  /* -------------------------------------------------------------------------- */

  return (
    <div className="flex flex-col gap-6">
      {/* Draft banner (if we restored a draft) */}
      {draftLoadedBanner && (
        <div className="rounded-md border border-dashed border-primary/40 bg-primary/5 px-3 py-2 text-xs text-muted-foreground flex items-center justify-between gap-3">
          <div className="flex flex-row gap-0.5">
            <span>Restored an unsaved draft from this browser</span>
            {lastSavedAtLabel && <span>(updated {lastSavedAtLabel}).</span>}
          </div>
          <div className="flex items-center gap-1">
            <Button type="button" variant="ghost" className="h-6 px-2 text-xs" onClick={reset} disabled={disabled}>
              Discard draft
            </Button>
            <Button
              type="button"
              variant="ghost"
              size="icon"
              aria-label="Dismiss"
              className="h-6 w-6 text-muted-foreground"
              onClick={() => setDraftLoadedBanner(false)}>
              <X className="w-3 h-3" />
            </Button>
          </div>
        </div>
      )}

      <div className="flex gap-6">
        {/* Sidebar (xl and up) */}
        <aside className="hidden xl:block relative pr-6 border-r">
          <div className="sticky top-16 hidden xl:flex flex-col gap-8 w-[140px] h-fit pb-24">
            {title && <p className="text-sm font-semibold text-muted-foreground mb-2">{title}</p>}

            {sections.map((sectionKey) => {
              const section = schema[sectionKey];

              return (
                <div key={sectionKey} className="flex flex-col gap-2">
                  {section.title && (
                    <p className="uppercase text-xs mb-1 text-muted-foreground text-right">{section.title}</p>
                  )}

                  {section.items.map((item) => {
                    if (item.kind === 'field') {
                      const f = item.field;
                      const key = f.key as string;
                      const isDirtyField = !!dirty[key];
                      const hasError = touched[key] && errors[key];

                      return (
                        <a href={`#${key}`} key={key}>
                          <Button
                            variant="secondary"
                            size="sm"
                            className={`justify-end w-full text-xs ${hasError ? '' : 'bg-accent/60'}`}>
                            {mode === 'edit' && isDirtyField && (
                              <span className="mr-1 text-[10px] text-destructive">*</span>
                            )}{' '}
                            {f.label}
                          </Button>
                        </a>
                      );
                    }

                    if (item.kind === 'row') {
                      return (
                        <a href={`#${item.id}`} key={item.id}>
                          <Button variant="secondary" size="sm" className="justify-end w-full text-xs bg-accent/60">
                            {item.id}
                          </Button>
                        </a>
                      );
                    }

                    const group = item;
                    // compute if any child field is dirty
                    let groupDirty = false;
                    for (const sub of group.items) {
                      if (sub.kind === 'field') {
                        const k = sub.field.key as string;
                        if (dirty[k]) {
                          groupDirty = true;
                          break;
                        }
                      } else if (sub.kind === 'row') {
                        for (const f of sub.fields) {
                          const k = f.key as string;
                          if (dirty[k]) {
                            groupDirty = true;
                            break;
                          }
                        }
                        if (groupDirty) break;
                      }
                    }

                    return (
                      <a href={`#${group.id}`} key={group.id}>
                        <Button variant="secondary" size="sm" className="justify-end w-full text-xs bg-accent/60">
                          {mode === 'edit' && groupDirty && (
                            <span className="mr-1 text-[10px] text-destructive">*</span>
                          )}
                          {group.label}
                        </Button>
                      </a>
                    );
                  })}
                </div>
              );
            })}

            {hasChanges && (
              <div className="mt-2 flex flex-col items-center gap-2">
                {mode === 'edit' && (
                  <>
                    <Button
                      className="w-full text-xs"
                      variant="outline"
                      onClick={reset}
                      disabled={disabled || !hasChanges}>
                      <History className="w-3 h-3" /> Reset
                    </Button>

                    <Button
                      className="w-full text-xs"
                      variant="outline"
                      disabled={!hasChanges}
                      onClick={() => setPreviewOpen(true)}>
                      <Eye className="w-3 h-3" /> Preview Changes
                    </Button>
                  </>
                )}
                <Button
                  className="w-full text-xs"
                  disabled={disabled || !isValid || !canSave || pending}
                  onClick={confirm}>
                  {pending ? <Loader2 className="w-4 h-4 animate-spin" /> : <Save className="w-3 h-3" />} Save
                </Button>
              </div>
            )}
          </div>
        </aside>

        {/* Main */}
        <main className="flex-1 flex flex-col gap-4">
          {sections.map((sectionKey) => {
            const section = schema[sectionKey];

            return (
              <div key={sectionKey} className="flex flex-col gap-5 border-b last:border-b-0 pb-10 last:pb-4">
                {section.title && <div className="text-md font-bold uppercase">{section.title}</div>}

                {section.items.map((item) => {
                  if (item.kind === 'field') {
                    const f = item.field;
                    const key = f.key as string;
                    const value = getValue(merged, key);
                    const error = errors[key];
                    const edited = mode === 'edit' && dirty[key];
                    const fieldDisabled = !!disabled || !!f.disabled;

                    return (
                      <fieldset
                        id={key}
                        key={key}
                        disabled={fieldDisabled}
                        className="relative border rounded-md p-6 scroll-mt-20 xl:scroll-mt-16">
                        <SmartField
                          render={f.render}
                          value={value}
                          onChange={getFieldHandler(key)}
                          label={f.label}
                          required={f.required}
                          description={f.description}
                          edited={!!edited}
                          error={error}
                          touched={!!touched[key]}
                        />
                      </fieldset>
                    );
                  }

                  if (item.kind === 'row') {
                    // Row at section level
                    return (
                      <section
                        id={item.id}
                        key={item.id}
                        className={`relative border rounded-md p-6 scroll-mt-20 xl:scroll-mt-16`}>
                        <div className={cn('flex flex-row w-full', item.gap ?? 'gap-4', item.className)}>
                          {item.fields.map((f) => {
                            const key = f.key as string;
                            const value = getValue(merged, key);
                            const error = errors[key];
                            const edited = mode === 'edit' && dirty[key];
                            const fieldDisabled = !!disabled || !!f.disabled;

                            return (
                              <fieldset key={key} disabled={fieldDisabled} className={`relative pb-0 last:pb-0 flex-1`}>
                                <SmartField
                                  render={f.render}
                                  value={value}
                                  onChange={getFieldHandler(key)}
                                  label={f.label}
                                  required={f.required}
                                  description={f.description}
                                  edited={!!edited}
                                  error={error}
                                  touched={!!touched[key]}
                                />
                              </fieldset>
                            );
                          })}
                        </div>
                      </section>
                    );
                  }

                  // Group
                  const group = item;
                  return (
                    <section
                      id={group.id}
                      key={group.id}
                      className={`relative border rounded-md p-6 flex gap-4 ${group.direction === 'horizontal' ? 'flex-row' : 'flex-col'} scroll-mt-20 xl:scroll-mt-16`}>
                      <div className="flex flex-col gap-4 w-full">
                        {group.items.map((sub) => {
                          if (sub.kind === 'field') {
                            const f = sub.field;
                            const key = f.key as string;
                            const value = getValue(merged, key);
                            const error = errors[key];
                            const edited = mode === 'edit' && dirty[key];
                            const fieldDisabled = !!disabled || !!f.disabled;

                            return (
                              <fieldset
                                key={key}
                                disabled={fieldDisabled}
                                className={`relative pb-6 last:pb-0 ${group.direction === 'horizontal' ? 'flex-1' : 'block border-b last:border-b-0'}`}>
                                <SmartField
                                  render={f.render}
                                  value={value}
                                  onChange={getFieldHandler(key)}
                                  label={f.label}
                                  required={f.required}
                                  description={f.description}
                                  edited={!!edited}
                                  error={error}
                                  touched={!!touched[key]}
                                />
                              </fieldset>
                            );
                          }

                          const row = sub;
                          return (
                            <div
                              key={row.id}
                              id={row.id}
                              className={`w-full ${group.direction === 'horizontal' ? 'flex-1' : ''} rounded-md scroll-mt-20 xl:scroll-mt-16`}>
                              <div
                                className={cn('flex flex-col sm:flex-row w-full', row.gap ?? 'gap-4', row.className)}>
                                {row.fields.map((f) => {
                                  const key = f.key as string;
                                  const value = getValue(merged, key);
                                  const error = errors[key];
                                  const edited = mode === 'edit' && dirty[key];
                                  const fieldDisabled = !!disabled || !!f.disabled;
                                  return (
                                    <fieldset
                                      key={key}
                                      disabled={fieldDisabled}
                                      className={` pb-1 last:pb-1  last:flex-1 scroll-mt-20 xl:scroll-mt-16`}>
                                      <SmartField
                                        render={f.render}
                                        value={value}
                                        onChange={getFieldHandler(key)}
                                        label={f.label}
                                        required={f.required}
                                        description={f.description}
                                        edited={!!edited}
                                        error={error}
                                        touched={!!touched[key]}
                                      />
                                    </fieldset>
                                  );
                                })}
                              </div>
                            </div>
                          );
                        })}
                      </div>
                    </section>
                  );
                })}
              </div>
            );
          })}
        </main>
      </div>

      {/* Bottom action bar for small screens */}
      <div className="xl:hidden sticky bottom-0 bg-background border-t pt-3 pb-3 mt-2">
        <div className="flex justify-end gap-2">
          {mode === 'edit' && (
            <>
              <Button variant="outline" size="sm" onClick={reset} disabled={disabled || !hasChanges}>
                <History className="w-4 h-4 mr-1" />
                Reset
              </Button>

              <Button variant="outline" size="sm" disabled={!hasChanges} onClick={() => setPreviewOpen(true)}>
                <Eye className="w-3 h-3" /> Preview Changes
              </Button>
            </>
          )}
          <Button size="sm" onClick={confirm} disabled={disabled || !isValid || !canSave || pending}>
            {pending ? <Loader2 className="w-4 h-4 animate-spin mr-1" /> : <Save className="w-3 h-3 mr-1" />} Save
          </Button>
        </div>
      </div>

      {/* Preview Modal */}
      <Dialog open={previewOpen} onOpenChange={setPreviewOpen}>
        <DialogContent aria-describedby={undefined} className="w-full max-w-[1100px] sm:max-w-[1100px]">
          <DialogHeader>
            <DialogTitle>Diff Preview</DialogTitle>
          </DialogHeader>

          <div className="pt-4">
            <MonacoDiff original={original} modified={merged} format="yaml" />
          </div>
        </DialogContent>
      </Dialog>
    </div>
  );
}
