import React, { useMemo, useState, useCallback, useEffect } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Loader2, Eye, History, Save } from 'lucide-react';
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { MonacoDiff } from '@/lib/monaco';

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
export interface GroupFieldConfig<T> {
  kind: 'group';
  id: string;
  label: string;
  description?: React.ReactElement | string;
  fields: FieldConfig<T>[];
}
export interface SectionConfig<T> {
  title: string;
  items: SectionItemConfig<T>[];
}

interface FieldShellProps {
  label: string;
  required?: boolean;
  description?: React.ReactElement | string;
  edited: boolean;
  error?: string | null;
  touched: boolean;
  children: React.ReactNode;
}

export type FormSchema<T> = Record<string, SectionConfig<T>>;
export type SectionItemConfig<T> = FieldItemConfig<T> | GroupFieldConfig<T>;
type FieldChange<T> = (partial: Partial<T> | ((prev: Partial<T>) => Partial<T>)) => void;

export function defineField<T, K extends Path<T>>(
  config: Omit<FieldConfig<T>, 'key'> & { key: K },
): FieldItemConfig<T> {
  return {
    kind: 'field',
    field: config,
  };
}

export function defineGroupField<T>(config: {
  id: string;
  label: string;
  description?: React.ReactElement | string;
  fields: Array<FieldItemConfig<T>>;
}): GroupFieldConfig<T> {
  return {
    kind: 'group',
    id: config.id,
    label: config.label,
    description: config.description,
    fields: config.fields.map((f) => f.field),
  };
}

export function defineSection<T>(config: SectionConfig<T>): SectionConfig<T> {
  return config;
}

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
}) {
  const [errors, setErrors] = useState<Record<string, string | null>>({});
  const [dirty, setDirty] = useState<Record<string, boolean>>({});
  const [touched, setTouched] = useState<Record<string, boolean>>({});
  const [previewOpen, setPreviewOpen] = useState(false);

  const merged = useMemo(() => smartMerge<T>(original, update), [original, update]);
  const sections = Object.keys(schema);

  // Flatten schema → key → FieldConfig
  const fieldMap = useMemo(() => {
    const map: Record<string, FieldConfig<T>> = {};
    for (const sectionKey of Object.keys(schema)) {
      const section = schema[sectionKey];
      for (const item of section.items) {
        if (item.kind === 'field') {
          map[item.field.key as string] = item.field;
        } else {
          for (const f of item.fields) {
            map[f.key as string] = f;
          }
        }
      }
    }
    return map;
  }, [schema]);

  // Derived flags
  const hasChanges = useMemo(() => Object.values(dirty).some(Boolean), [dirty]);
  const isValid = useMemo(() => Object.values(errors).every((v) => !v), [errors]);
  const canSave = hasChanges;

  // Recompute dirty + errors whenever merged/original/schema change
  useEffect(() => {
    const newErrors: Record<string, string | null> = {};
    const newDirty: Record<string, boolean> = {};

    for (const [key, field] of Object.entries(fieldMap)) {
      const val = getValue(merged, key);
      const originalVal = getValue(original, key);

      newDirty[key] = val !== originalVal;

      const isEmpty = val === undefined || val === '';
      let err: string | null = null;

      if (field.required && isEmpty) {
        err = 'Required';
      } else if (field.validate) {
        err = field.validate(val);
      }

      newErrors[key] = err;
    }

    if (!shallowEqual(newDirty, dirty)) {
      setDirty(newDirty);
    }
    if (!shallowEqual(newErrors, errors)) {
      setErrors(newErrors);
    }
  }, [merged, original, fieldMap]);

  const handleChange = useCallback(
    (key: string, partialOrUpdater: Partial<T> | ((prev: Partial<T>) => Partial<T>)) => {
      if (disabled) return;

      // mark field as touched
      setTouched((prev) => {
        if (prev[key]) return prev;
        return { ...prev, [key]: true };
      });

      // apply external update
      setUpdate((prev) => {
        const current = prev ?? {};
        const resolved = typeof partialOrUpdater === 'function' ? partialOrUpdater(current) : partialOrUpdater;
        return smartMerge<Partial<T>>(current, resolved);
      });
    },
    [disabled, setUpdate],
  );

  const createFieldChangeHandler = useCallback(
    (key: string): FieldChange<T> =>
      (partialOrUpdater) =>
        handleChange(key, partialOrUpdater),
    [handleChange],
  );

  const reset = useCallback(() => {
    setUpdate({});
    setDirty({});
    setErrors({});
    setTouched({});
  }, [setUpdate]);

  const validateAll = useCallback(
    (mergedValue: T) => {
      const newErrors: Record<string, string | null> = {};

      for (const [key, field] of Object.entries(fieldMap)) {
        const val = getValue(mergedValue, key);
        const isEmpty = val === undefined || val === '';

        let error: string | null = null;
        if (field.required && isEmpty) {
          error = 'Required';
        } else if (field.validate) {
          error = field.validate(val);
        }
        newErrors[key] = error;
      }

      setErrors(newErrors);
      setTouched((prev) => {
        const next: Record<string, boolean> = { ...prev };
        for (const key of Object.keys(fieldMap)) {
          next[key] = true;
        }
        return next;
      });

      return Object.values(newErrors).every((v) => !v);
    },
    [fieldMap],
  );

  const confirm = useCallback(async () => {
    const valid = validateAll(merged);
    if (!valid) return;
    await onSave(merged as T);
  }, [validateAll, merged, onSave, reset]);

  return (
    <div className="flex flex-col gap-6">
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
                            )}
                            {f.label}
                          </Button>
                        </a>
                      );
                    }

                    const group = item;
                    const fieldKeys = group.fields.map((f) => f.key as string);
                    const groupDirty = fieldKeys.some((k) => dirty[k]);

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
                  <Button
                    className="w-full text-xs"
                    variant="outline"
                    onClick={reset}
                    disabled={disabled || !hasChanges}>
                    <History className="w-3 h-3" /> Reset
                  </Button>
                )}
                <Button
                  className="w-full text-xs"
                  variant="outline"
                  disabled={!hasChanges}
                  onClick={() => setPreviewOpen(true)}>
                  <Eye className="w-3 h-3" /> Preview Changes
                </Button>
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
                      <fieldset id={key} key={key} disabled={fieldDisabled} className="relative border rounded-md p-6">
                        <FieldShell
                          label={f.label}
                          required={f.required}
                          description={f.description}
                          edited={!!edited}
                          error={error}
                          touched={!!touched[key]}>
                          {f.render(value, createFieldChangeHandler(key))}
                        </FieldShell>
                      </fieldset>
                    );
                  }

                  const group = item;
                  return (
                    <section
                      id={group.id}
                      key={group.id}
                      className="relative border rounded-md p-6 flex flex-col gap-4">
                      <div className="flex flex-col gap-4">
                        {group.fields.map((f) => {
                          const key = f.key as string;
                          const value = getValue(merged, key);
                          const error = errors[key];
                          const edited = mode === 'edit' && dirty[key];
                          const fieldDisabled = !!disabled || !!f.disabled;

                          return (
                            <fieldset
                              key={key}
                              disabled={fieldDisabled}
                              className="relative pb-6 last:pb-0 border-b last:border-b-0">
                              <FieldShell
                                label={f.label}
                                required={f.required}
                                description={f.description}
                                edited={!!edited}
                                error={error}
                                touched={!!touched[key]}>
                                {f.render(value, createFieldChangeHandler(key))}
                              </FieldShell>
                            </fieldset>
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
            <Button variant="outline" size="sm" onClick={reset} disabled={disabled || !hasChanges}>
              <History className="w-4 h-4 mr-1" />
              Reset
            </Button>
          )}
          <Button variant="outline" size="sm" disabled={!hasChanges} onClick={() => setPreviewOpen(true)}>
            <Eye className="w-3 h-3" /> Preview Changes
          </Button>
          <Button size="sm" onClick={confirm} disabled={disabled || !isValid || !canSave || pending}>
            {pending ? <Loader2 className="w-4 h-4 animate-spin mr-1" /> : <Save className="w-3 h-3 mr-1" />} Save
          </Button>
        </div>
      </div>

      {/* --- Preview Modal --- */}
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

/* ----------------------------- Utilities ----------------------------- */

export const FieldInput = ({
  value,
  onChange,
  placeholder,
  type,
  disabled,
}: {
  value?: string;
  onChange: (v: string) => void;
  placeholder?: string;
  type?: string;
  disabled?: boolean;
}) => (
  <Input
    disabled={disabled}
    type={type}
    value={value ?? ''}
    onChange={(e) => onChange(e.target.value)}
    placeholder={placeholder}
    className="max-w-[400px]"
  />
);

function getValue(obj: any, path: string): any {
  return path.split('.').reduce((acc, key) => acc?.[key], obj);
}

function smartMerge<T>(target: any, source: any): T {
  if (!isObject(target) || !isObject(source)) return (source ?? target) as T;

  let mutated = false;
  const output: any = Array.isArray(target) ? [...target] : { ...target };

  for (const key of Object.keys(source)) {
    const prev = target[key];
    const next = source[key];

    if (isObject(prev) && isObject(next)) {
      const nested = smartMerge(prev, next);
      if (nested !== prev) {
        mutated = true;
        output[key] = nested;
      }
    } else if (next !== prev) {
      mutated = true;
      output[key] = next;
    }
  }

  return mutated ? output : target;
}

function isObject(item: any): boolean {
  return item && typeof item === 'object' && !Array.isArray(item);
}

function shallowEqual(a: Record<string, any>, b: Record<string, any>) {
  const aKeys = Object.keys(a);
  const bKeys = Object.keys(b);
  if (aKeys.length !== bKeys.length) return false;
  for (const k of aKeys) {
    if (a[k] !== b[k]) return false;
  }
  return true;
}
