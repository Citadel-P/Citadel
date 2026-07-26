namespace Application.Services;

internal interface IStackStoragePathProvider
{
    string StacksRoot { get; }
}

internal sealed class StackStoragePathProvider : IStackStoragePathProvider
{
    public string StacksRoot => ApplicationStoragePaths.StacksRoot;
}
