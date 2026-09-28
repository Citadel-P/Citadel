import { Component, type ReactNode } from 'react';
import { Button } from '@/components/ui/button';

export class ErrorBoundary extends Component<{ children: ReactNode; root?: boolean }, { failed: boolean }> {
  state = { failed: false };

  static getDerivedStateFromError() {
    return { failed: true };
  }

  render() {
    if (!this.state.failed) return this.props.children;
    return (
      <section role="alert" className="m-6 space-y-3 rounded-lg border bg-card p-6 text-sm">
        <h1 className="font-semibold">
          {this.props.root ? 'Citadel could not be displayed' : 'This page could not be displayed'}
        </h1>
        <p className="text-muted-foreground">
          {this.props.root ? 'Reload the app to try again.' : 'Try again, or choose another page from the sidebar.'}
        </p>
        <Button
          variant="outline"
          onClick={() => (this.props.root ? window.location.reload() : this.setState({ failed: false }))}>
          {this.props.root ? 'Reload app' : 'Try again'}
        </Button>
      </section>
    );
  }
}
