import { Component, type ErrorInfo, type ReactNode } from "react";
import { Button } from "../../meridian-ui";
export class SettingsPageBoundary extends Component<{ children: ReactNode; page: string; retry: () => void }, { failed: boolean }> {
  override state = { failed: false };
  static getDerivedStateFromError() { return { failed: true }; }
  override componentDidCatch(error: Error, info: ErrorInfo) { console.error(`Settings page ${this.props.page} failed`, error, info); }
  override render() {
    if (this.state.failed) return <section className="settings-card settings-fallback" role="alert"><h3>This page could not be displayed</h3><p>Your other settings are still available. Retry this page or select another category.</p><Button onClick={this.props.retry}>Retry page</Button></section>;
    return this.props.children;
  }
}
