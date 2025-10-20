import { FormField, FormItem, FormControl, FormMessage } from '@/components/ui/form';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';

interface PortMappingInputProps {
  control: any;
  name: string;
  label?: string | undefined;
  ports: string[];
}

const PortMappingInput = ({ control, name, ports, label }: PortMappingInputProps) => {
  return (
    <div className="col-span-2">
      <div className="flex items-center gap-1">
        <Label className="flex-none text-xs mb-1">{label ?? 'Ports'}</Label>
      </div>

      <div className="space-y-2">
        {ports.length === 0 && <span className="text-xs text-muted">No ports exposed in this image</span>}
        {ports.map((port, idx) => (
          <div key={idx} className="flex flex-col sm:flex-row gap-2 items-start sm:items-center">
            <FormField
              control={control}
              name={`${name}.${idx}`}
              render={({ field }) => {
                const isDefaultValue = field.value === port;
                const hostPort = isDefaultValue ? '' : field.value.substring(0, field.value.length - port.length - 1);

                return (
                  <FormItem className="flex-1 w-full">
                    <FormControl>
                      <div className="flex items-stretch">
                        <Input
                          placeholder="Host port"
                          className="rounded-r-none! focus-visible:ring-transparent"
                          type="number"
                          min="0"
                          max="65535"
                          value={hostPort}
                          onChange={(e) => {
                            const newHostPort = e.target.value;
                            if (newHostPort) {
                              field.onChange(`${newHostPort}-${port}`);
                            } else {
                              field.onChange(port);
                            }
                          }}
                        />
                        <span className="flex z-10 items-center justify-center w-[90px] flex-shrink-0 bg-accent/60 border-r rounded-r-sm border-y border-border text-xs h-full">
                          {':'}
                          {port}
                        </span>
                      </div>
                    </FormControl>
                    <FormMessage className="text-xs" />
                  </FormItem>
                );
              }}
            />
          </div>
        ))}
      </div>
    </div>
  );
};

export default PortMappingInput;
