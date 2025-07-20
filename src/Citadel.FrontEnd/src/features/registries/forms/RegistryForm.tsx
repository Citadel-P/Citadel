import { Label } from '@/components/ui/label';
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group';
import { useRegistryFormContext } from './RegistryFormProvider';
import Loader from '@/components/ui/loader';

const RegistryForm = () => {
  const { providers, formTitle, setCurrentProvider, currentProvider, isLoading } = useRegistryFormContext();

  if (isLoading) return <Loader />;
  return (
    <div className="mx-auto px-4 py-3 lg:container sm:px-6">
      <div className="w-full rounded-lg border-border bg-background p-4">
        <div className="mb-4 flex items-center justify-between">
          <div>
            <h5 className="text-md font-bold text-foreground">{formTitle}</h5>
          </div>
        </div>
        <ol className="relative border-s border-border ml-1">
          <li className="mb-10 ms-6">
            <span className="absolute flex items-center justify-center w-5 h-5 bg-primary/80 rounded-full -start-2.5 ring-4 ring-background text-xs text-primary-foreground">
              1
            </span>
            <h2 className="text-sm mb-2 font-semibold text-foreground">Choose a registry provider</h2>
            <RadioGroup
              value={currentProvider}
              defaultValue={currentProvider}
              className="flex flex-wrap justify-between"
              onValueChange={setCurrentProvider}>
              {providers.map((provider) => (
                <Label
                  key={provider.id}
                  className={`${provider.disabled ? 'cursor-not-allowed bg-foreground/2' : 'hover:bg-accent/50 hover:cursor-pointer'} flex-grow flex-1 flex items-start gap-3 rounded-lg border p-4 has-[[data-state=checked]]:border-primary/50 has-[[data-state=checked]]:bg-primary/10 `}>
                  <RadioGroupItem
                    value={provider.id}
                    id={provider.name}
                    disabled={provider.disabled}
                    className="shadow-none data-[state=checked]:border-primary/50 data-[state=checked]:bg-primary *:data-[slot=radio-group-indicator]:[&>svg]:fill-white *:data-[slot=radio-group-indicator]:[&>svg]:stroke-white"
                  />
                  <div className="grid gap-1 font-normal">
                    <div className="font-medium">{provider.name}</div>
                    <div className="text-muted-foreground leading-snug">{provider.description}</div>
                  </div>
                </Label>
              ))}
            </RadioGroup>
          </li>
          <li className="mb-10 ms-6">
            <span className="absolute flex items-center justify-center w-5 h-5 bg-primary/80 rounded-full -start-2.5 ring-4 ring-background text-xs text-primary-foreground">
              2
            </span>
            <h2 className="text-sm font-semibold text-foreground">Configure</h2>
            {providers.find((s) => s.id === currentProvider)?.configuration}
          </li>
        </ol>
      </div>
    </div>
  );
};

export default RegistryForm;
