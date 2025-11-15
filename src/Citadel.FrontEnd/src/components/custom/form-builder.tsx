import React, { useMemo, useState } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { History, Save } from 'lucide-react';
import { Loader2 } from 'lucide-react';

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
  render: (value: any, set: (partial: Partial<T> | ((prev: Partial<T>) => Partial<T>)) => void) => React.ReactNode;
}

export interface FieldItemConfig<T> {
  kind: 'field';
  field: FieldConfig<T>;
}

export interface GroupFieldConfig<T> {
  kind: 'group';
  id: string; // anchor id for the group
  label: string; // label for sidebar + group header
  description?: React.ReactElement | string;
  fields: FieldConfig<T>[];
}

export type SectionItemConfig<T> = FieldItemConfig<T> | GroupFieldConfig<T>;

export interface SectionConfig<T> {
  title: string;
  items: SectionItemConfig<T>[];
}

export type FormSchema<T> = Record<string, SectionConfig<T>>;

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

  const merged = useMemo(() => deepMerge<T>(original, update), [original, update]);
  const sections = Object.keys(schema);

  const allKeys = useMemo(() => {
    const keys: string[] = [];
    for (const sectionKey of sections) {
      const section = schema[sectionKey];
      for (const item of section.items) {
        if (item.kind === 'field') {
          keys.push(item.field.key as string);
        } else {
          for (const f of item.fields) {
            keys.push(f.key as string);
          }
        }
      }
    }
    return keys;
  }, [sections, schema]);

  const hasChanges = useMemo(() => Object.keys(update).length > 0, [update]);
  const isDirty = Object.values(dirty).some(Boolean);
  const canSave = mode === 'edit' ? isDirty : hasChanges;
  const isValid = Object.values(errors).every((v) => !v);

  const findFieldByKey = (key: string): FieldConfig<T> | undefined => {
    for (const sectionKey of sections) {
      const section = schema[sectionKey];
      for (const item of section.items) {
        if (item.kind === 'field') {
          if (item.field.key === key) return item.field;
        } else {
          for (const f of item.fields) {
            if (f.key === key) return f;
          }
        }
      }
    }
    return undefined;
  };

  const validateField = (key: string, value: any) => {
    const field = findFieldByKey(key);
    if (!field) return;

    let error: string | null = null;
    const isEmpty = value === undefined || value === '';

    if (field.required && isEmpty) {
      error = 'Required';
    } else if (field.validate) {
      error = field.validate(value);
    }

    setErrors((prev) => ({
      ...prev,
      [key]: error,
    }));
  };

  const validateAll = (mergedValue: T) => {
    const newErrors: Record<string, string | null> = {};

    for (const sectionKey of sections) {
      const section = schema[sectionKey];

      for (const item of section.items) {
        if (item.kind === 'field') {
          const field = item.field;
          const key = field.key as string;
          const val = getValue(mergedValue, key);
          const isEmpty = val === undefined || val === '';

          if (field.required && isEmpty) {
            newErrors[key] = 'Required';
          } else if (field.validate) {
            newErrors[key] = field.validate(val);
          } else {
            newErrors[key] = null;
          }
        } else {
          for (const field of item.fields) {
            const key = field.key as string;
            const val = getValue(mergedValue, key);
            const isEmpty = val === undefined || val === '';

            if (field.required && isEmpty) {
              newErrors[key] = 'Required';
            } else if (field.validate) {
              newErrors[key] = field.validate(val);
            } else {
              newErrors[key] = null;
            }
          }
        }
      }
    }

    setErrors(newErrors);
    setTouched((prev) => {
      const next: Record<string, boolean> = { ...prev };
      for (const key of allKeys) {
        next[key] = true;
      }
      return next;
    });

    return Object.values(newErrors).every((v) => !v);
  };

  const handleChange = (key: string, partialOrUpdater: Partial<T> | ((prev: Partial<T>) => Partial<T>)) => {
    if (disabled) return;

    setUpdate((prev) => {
      const resolved = typeof partialOrUpdater === 'function' ? partialOrUpdater(prev) : partialOrUpdater;

      const newUpdate = deepMerge<Partial<T>>(prev, resolved);
      const newMerged = deepMerge<T>(original, newUpdate);

      const newVal = getValue(newMerged, key);
      const oldVal = getValue(original, key);

      if (mode === 'edit') {
        setDirty((d) => ({ ...d, [key]: newVal !== oldVal }));
      }

      setTouched((t) => ({ ...t, [key]: true }));
      validateField(key, newVal);

      return newUpdate;
    });
  };

  const reset = () => {
    setUpdate({});
    setDirty({});
    setErrors({});
    setTouched({});
  };

  const confirm = async () => {
    const valid = validateAll(merged);
    if (!valid) return;
    await onSave(merged as T);
    reset();
  };

  const getGroupDirtyAndError = (group: GroupFieldConfig<T>) => {
    const fieldKeys = group.fields.map((f) => f.key as string);
    const groupDirty = fieldKeys.some((k) => dirty[k]);
    const groupHasError = fieldKeys.some((k) => touched[k] && errors[k]);
    return { groupDirty, groupHasError };
  };

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

                    const { groupDirty } = getGroupDirtyAndError(item);

                    return (
                      <a href={`#${item.id}`} key={item.id}>
                        <Button variant="secondary" size="sm" className="justify-end w-full text-xs bg-accent/60">
                          {mode === 'edit' && groupDirty && (
                            <span className="mr-1 text-[10px] text-destructive">*</span>
                          )}
                          {item.label}
                        </Button>
                      </a>
                    );
                  })}
                </div>
              );
            })}

            {canSave && (
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
                    const showError = touched[key] && error;
                    const edited = mode === 'edit' && dirty[key];
                    const fieldDisabled = !!disabled || !!f.disabled;

                    return (
                      <fieldset id={key} key={key} disabled={fieldDisabled} className="relative border rounded-md p-6">
                        <div className="flex flex-col gap-4">
                          <div>
                            <label className="block font-medium text-sm">
                              {f.label}
                              {f.required && <span className="text-destructive ml-1">*</span>}
                            </label>
                            {typeof f.description === 'string' ? (
                              <p className="text-sm text-muted-foreground">{f.description}</p>
                            ) : (
                              f.description
                            )}
                          </div>

                          <div className="relative">
                            {edited && (
                              <span className="absolute -top-0 right-1 text-[10px] text-primary bg-background px-1">
                                Edited
                              </span>
                            )}
                            {f.render(value, (p) => handleChange(key, p))}
                          </div>

                          {showError && <p className="text-xs text-destructive mt-1">{error}</p>}
                        </div>
                      </fieldset>
                    );
                  }

                  // Group rendering
                  const group = item;
                  return (
                    <section
                      id={group.id}
                      key={group.id}
                      className="relative border rounded-md p-6 flex flex-col gap-4">
                      {/* <div>
                        <h3 className="font-medium text-sm">{group.label}</h3>
                        {typeof group.description === 'string' ? (
                          <p className="text-sm text-muted-foreground">{group.description}</p>
                        ) : (
                          group.description
                        )}
                      </div> */}

                      <div className="flex flex-col gap-4">
                        {group.fields.map((f) => {
                          const key = f.key as string;
                          const value = getValue(merged, key);
                          const error = errors[key];
                          const showError = touched[key] && error;
                          const edited = mode === 'edit' && dirty[key];
                          const fieldDisabled = !!disabled || !!f.disabled;

                          return (
                            <fieldset
                              key={key}
                              disabled={fieldDisabled}
                              className="relative pb-6 last:pb-0 border-b last:border-b-0">
                              <div className="flex flex-col gap-4">
                                <div>
                                  <label className="block font-medium text-sm">
                                    {f.label}
                                    {f.required && <span className="text-destructive ml-1">*</span>}
                                  </label>
                                  {typeof f.description === 'string' ? (
                                    <p className="text-sm text-muted-foreground">{f.description}</p>
                                  ) : (
                                    f.description
                                  )}
                                </div>
                                <div className="relative">
                                  {edited && (
                                    <span className="absolute -top-0 right-1 text-[10px] text-primary bg-background px-1">
                                      Edited
                                    </span>
                                  )}
                                  {f.render(value, (p) => handleChange(key, p))}
                                </div>

                                {showError && <p className="text-xs text-destructive mt-1">{error}</p>}
                              </div>
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
          <Button size="sm" onClick={confirm} disabled={disabled || !isValid || !canSave || pending}>
            {pending ? <Loader2 className="w-4 h-4 animate-spin mr-1" /> : <Save className="w-3 h-3 mr-1" />} Save
          </Button>
        </div>
      </div>
    </div>
  );
}

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

function deepMerge<T>(target: any, source: any): T {
  const output: any = { ...target };
  if (isObject(target) && isObject(source)) {
    Object.keys(source).forEach((key) => {
      if (isObject(source[key])) {
        if (!(key in target)) {
          Object.assign(output, { [key]: source[key] });
        } else {
          output[key] = deepMerge(target[key], source[key]);
        }
      } else {
        Object.assign(output, { [key]: source[key] });
      }
    });
  }
  return output;
}

function isObject(item: any): boolean {
  return item && typeof item === 'object' && !Array.isArray(item);
}
