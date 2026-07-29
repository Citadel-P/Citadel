namespace Application.Services.Identity;

public interface ISetupStateCache
{
    bool TryGetRequiresSetup(out bool requiresSetup);
    void SetRequiresSetup(bool requiresSetup);
    void Reset();
}

internal sealed class SetupStateCache : ISetupStateCache
{
    private const int Unknown = 0;
    private const int Pending = 1;
    private const int Complete = 2;

    private int state;

    public bool TryGetRequiresSetup(out bool requiresSetup)
    {
        var current = Volatile.Read(ref state);
        requiresSetup = current == Pending;
        return current != Unknown;
    }

    public void SetRequiresSetup(bool requiresSetup)
        => Volatile.Write(ref state, requiresSetup ? Pending : Complete);

    public void Reset() => Volatile.Write(ref state, Unknown);
}
