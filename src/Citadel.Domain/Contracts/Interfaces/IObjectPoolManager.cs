using Microsoft.Extensions.ObjectPool;

namespace Domain.Contracts.Interfaces;

/// <summary>
/// Factory for managing object pools.
/// </summary>
public interface IObjectPoolManager
{
    ObjectPool<T> GetPool<T>() where T : class, new();
    T Get<T>() where T : class, new();
    void Return<T>(T obj) where T : class, new();
}
