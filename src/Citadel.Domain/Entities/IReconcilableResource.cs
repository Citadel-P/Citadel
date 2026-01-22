namespace Domain.Entities;

/// <summary>
/// Marks a resource that can enter a transient control state and may require
/// reconciliation if the control operation does not complete as expected.
/// </summary>
internal interface IReconcilableResource
{
    /// <summary>
    /// Gets the current control state of the resource.
    /// </summary>
    ResourceControlState ControlState { get; }

    /// <summary>
    /// Gets the Unix timestamp (in seconds) at which the current control operation started.
    /// A null value indicates that no control operation is in progress.
    /// </summary>
    long? ControlStartedAt { get; }

    /// <summary>
    /// Gets the current row version used for optimistic concurrency control.
    /// </summary>
    long RowVersion { get; }
}
