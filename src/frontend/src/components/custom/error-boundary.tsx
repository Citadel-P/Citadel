import { Component, type ReactNode } from 'react';
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
      <section role="alert" className="m-6 space-y-3 rounded-lg border bg-card p-6 text-sm">
        <h1 className="font-semibold">
          {this.props.root ? 'Citadel could not be displayed' : 'This page could not be displayed'}
        </h1>
        <p className="text-muted-foreground">
          {reload ? 'Reload the app to try again.' : 'Try again, or choose another page from the sidebar.'}
        </p>
        <Button
          variant="outline"
          onClick={() => (reload ? window.location.reload() : this.setState({ failure: null }))}>
          {reload ? 'Reload app' : 'Try again'}
        </Button>
      </section>
    );
  }
}
