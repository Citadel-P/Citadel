import { Component, type ReactNode } from 'react';
import { FileWarning } from 'lucide-react';
import { ErrorPanel } from './page-error';
import { Button } from '@/components/ui/button';

type Failure = 'render' | 'module-load' | null;

export class ErrorBoundary extends Component<{ children: ReactNode; root?: boolean }, { failure: Failure }> {
  state: { failure: Failure } = { failure: null };

  static getDerivedStateFromError(error: unknown): { failure: Failure } {
    // React.lazy retains rejected imports for the lifetime of the document.
    const moduleLoadFailed =
      error instanceof TypeError &&
      /failed to fetch dynamically imported module|error loading dynamically imported module|importing a module script failed/i.test(
        error.message,
      );
    return { failure: moduleLoadFailed ? 'module-load' : 'render' };
  }

  render() {
    if (!this.state.failure) return this.props.children;
    const reload = this.props.root || this.state.failure === 'module-load';
    return (
      <ErrorPanel
        title={this.props.root ? 'Citadel could not be displayed' : 'This page could not be displayed'}
        message={reload ? 'Reload the app to try again.' : 'Try again, or choose another page from the sidebar.'}
        illustration={
          <FileWarning aria-hidden="true" className="my-6 size-24 text-muted-foreground" strokeWidth={1.25} />
        }>
        <Button
          variant="outline"
          onClick={() => (reload ? window.location.reload() : this.setState({ failure: null }))}>
          {reload ? 'Reload app' : 'Try again'}
        </Button>
      </ErrorPanel>
    );
  }
}
