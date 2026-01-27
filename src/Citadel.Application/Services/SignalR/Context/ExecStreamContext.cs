using Domain.Contracts.Interfaces;

namespace Application.Services.SignalR.Context;

internal sealed class ExecStreamContext : StreamContext
{
    public IExecSession? Session { get; set; }
    public CancellationTokenSource Cancellation { get; private set; } = new();

    public override void RemoveSubscriber(string connectionId)
    {
        base.RemoveSubscriber(connectionId);

        if (IsEmpty)
        {
            try { Cancellation.Cancel(); } catch { }
            try { Cancellation.Dispose(); } catch { }
            Cancellation = new CancellationTokenSource();
        }
    }
}