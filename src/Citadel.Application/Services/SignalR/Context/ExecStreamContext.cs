using Domain.Contracts.Interfaces;

namespace Application.Services.SignalR.Context;

internal sealed class ExecStreamContext : StreamContext
{
    public IExecSession? Session { get; set; }
    public CancellationTokenSource Cancellation { get; private set; } = new();
    public int LatestCols { get; set; }
    public int LatestRows { get; set; }
    public string Shell { get; set; }
    public override void RemoveSubscriber(string connectionId)
    {
        base.RemoveSubscriber(connectionId);

        if (IsEmpty)
        {
            var old = Cancellation;
            Cancellation = new CancellationTokenSource();

            try { old.Cancel(); } catch { }
            try { old.Dispose(); } catch { }
        }
    }
}
