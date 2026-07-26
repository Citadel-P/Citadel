using Hosting.Common;
using System.Collections.Concurrent;

namespace Application.Services;

internal interface IUpdateCheckLeaseManager
{
    bool TryAcquire(ResourceType resourceType, Guid resourceId, out IDisposable? lease);
}

internal sealed class UpdateCheckLeaseManager : IUpdateCheckLeaseManager
{
    private readonly ConcurrentDictionary<UpdateCheckKey, byte> activeChecks = new();

    public bool TryAcquire(ResourceType resourceType, Guid resourceId, out IDisposable? lease)
    {
        var key = new UpdateCheckKey(resourceType, resourceId);
        if (!activeChecks.TryAdd(key, 0))
        {
            lease = null;
            return false;
        }

        lease = new Lease(activeChecks, key);
        return true;
    }

    private readonly record struct UpdateCheckKey(ResourceType ResourceType, Guid ResourceId);

    private sealed class Lease(
        ConcurrentDictionary<UpdateCheckKey, byte> activeChecks,
        UpdateCheckKey key) : IDisposable
    {
        private int disposed;

        public void Dispose()
        {
            if (Interlocked.Exchange(ref disposed, 1) == 0)
            {
                activeChecks.TryRemove(key, out _);
            }
        }
    }
}
