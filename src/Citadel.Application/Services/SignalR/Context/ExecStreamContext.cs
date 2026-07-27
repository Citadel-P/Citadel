using Domain.Contracts.Interfaces;

namespace Application.Services.SignalR.Context;

internal sealed class ExecStreamContext : StreamContext, IDisposable
{
    public IExecSession? Session { get; set; }
    public CancellationTokenSource Cancellation { get; } = new();
    public int LatestCols { get; set; }
    public int LatestRows { get; set; }

    public void Dispose()
    {
        try { Cancellation.Cancel(); } catch { }
        Cancellation.Dispose();
    }
}
