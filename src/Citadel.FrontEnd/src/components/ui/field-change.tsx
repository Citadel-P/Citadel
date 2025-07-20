import { UseFormReturn } from 'react-hook-form';
import { Badge } from './badge';
interface IProps {
  fieldName: string;
  form: UseFormReturn<any, any, any>;
}

export function FieldChange({ form, fieldName }: IProps) {
  return form.getFieldState(fieldName).isDirty ? (
    <Badge variant="secondary" className="rounded-sm text-[11px] font-normal absolute right-8">
      Edited
    </Badge>
  ) : null;
}
