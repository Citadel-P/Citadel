import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import {
  ActivityEventType,
  ActivityResourceType,
  ActivityStatus,
  DeploymentStatus,
  DeploymentView,
  LatestActivityView,
  UpdateBehavior,
} from '@/api/generated/api.types';
import { DeploymentFormComponents } from './index';

const SubHeader = DeploymentFormComponents.EditForm!.SubHeader!;
const message = 'Deployment runtime is unavailable: Docker returned invalid JSON';
const failedApply: LatestActivityView = {
  id: 'failed-apply',
  resourceType: ActivityResourceType.Deployment,
  eventType: ActivityEventType.DeploymentApplied,
  status: ActivityStatus.Failure,
  createdAt: '2026-09-19T22:40:23Z',
  info: {
    $type: 'DeploymentApplied',
    deployment: null,
    result: { message, code: 500 },
  },
};

function deployment(status: DeploymentStatus, activity = failedApply) {
  return {
    status,
    latestActivityView: activity,
    spec: { updateBehavior: UpdateBehavior.Disabled },
  } as DeploymentView;
}

describe('deployment activity header', () => {
  it('distinguishes a healthy runtime from a previous failed apply', () => {
    render(<SubHeader resource={deployment(DeploymentStatus.Healthy)} />);
    expect(screen.getByText('Previous deployment attempt failed')).toBeInTheDocument();
    expect(screen.getByText(/container is currently healthy/)).toBeInTheDocument();
    expect(screen.queryByText(message)).not.toBeInTheDocument();
  });

  it('keeps the operation failure detail visible for a degraded deployment', () => {
    render(<SubHeader resource={deployment(DeploymentStatus.Degraded)} />);
    expect(screen.getByText('Last operation failed')).toBeInTheDocument();
    expect(screen.getByText(message)).toBeInTheDocument();
  });

  it('removes a stale degradation warning after runtime recovery', () => {
    const activity: LatestActivityView = {
      ...failedApply,
      eventType: ActivityEventType.DeploymentDegraded,
      status: ActivityStatus.Warning,
      info: { $type: 'DeploymentDegraded', reason: 'Container stopped' },
    };
    const { rerender } = render(<SubHeader resource={deployment(DeploymentStatus.Degraded, activity)} />);
    expect(screen.getByText('Container stopped')).toBeInTheDocument();
    rerender(<SubHeader resource={deployment(DeploymentStatus.Healthy, activity)} />);
    expect(screen.queryByText('Container stopped')).not.toBeInTheDocument();
  });
});
