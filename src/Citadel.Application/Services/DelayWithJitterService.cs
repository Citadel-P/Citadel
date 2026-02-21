using Hosting.Common;

namespace Application.Services;

internal interface IDelayWithJitterService
{
    Task DelayWithJitterForAsync(Func<CancellationToken, Task> task, TimeSpan maxJitter = default, CancellationToken cancellationToken = default);
}

internal sealed class DelayWithJitterService : IDelayWithJitterService
{
    public Task DelayWithJitterForAsync(Func<CancellationToken, Task> task, TimeSpan maxJitter = default, CancellationToken cancellationToken = default)
        => Helpers.DelayWithJitterFor(task, maxJitter, cancellationToken);
}
