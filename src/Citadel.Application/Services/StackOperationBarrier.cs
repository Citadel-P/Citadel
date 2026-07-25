namespace Application.Services;

internal enum StackOperationCheckpoint
{
    ProcessingClaimed,
    ExternalStateCaptured
}

internal interface IStackOperationBarrier
{
    ValueTask WaitAsync(
        StackOperationCheckpoint checkpoint,
        Guid stackId,
        long operationRowVersion,
        CancellationToken cancellationToken);
}

internal sealed class NoOpStackOperationBarrier : IStackOperationBarrier
{
    public ValueTask WaitAsync(
        StackOperationCheckpoint checkpoint,
        Guid stackId,
        long operationRowVersion,
        CancellationToken cancellationToken)
        => ValueTask.CompletedTask;
}
