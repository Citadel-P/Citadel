import { createContext, useContext } from 'react';

export interface FormFieldAccessibility {
  label: string;
  describedBy?: string;
  invalid: boolean;
}

export const FormFieldAccessibilityContext = createContext<FormFieldAccessibility | undefined>(undefined);

export const useFormFieldAccessibility = () => useContext(FormFieldAccessibilityContext);
