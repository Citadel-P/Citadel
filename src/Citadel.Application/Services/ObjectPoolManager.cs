using System.Collections.Concurrent;
using Domain.Contracts.Interfaces;
using Hosting.Common.Utils;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.ObjectPool;

namespace Application.Services;

internal class ObjectPoolManager(ObjectPoolProvider provider, IServiceProvider services) : IObjectPoolManager
{
    private readonly ConcurrentDictionary<Type, object> _pools = new();

    public ObjectPool<T> GetPool<T>() where T : class, new()
    {
        return (ObjectPool<T>)_pools.GetOrAdd(typeof(T), static (type, ctx) =>
        {
            var (provider, services) = ((ObjectPoolProvider, IServiceProvider))ctx;
            var policy = services.GetService<IPooledObjectPolicy<T>>() ?? CreateDefaultPolicy<T>();
            return provider.Create(policy);
        }, (provider, services));
    }

    public T Get<T>() where T : class, new()
    {
        return GetPool<T>().Get();
    }

    public void Return<T>(T obj) where T : class, new()
    {
        GetPool<T>().Return(obj);        
    }

    private static IPooledObjectPolicy<T> CreateDefaultPolicy<T>() where T : class, new()
    {
        if (typeof(T).IsGenericType &&
            typeof(T).GetGenericTypeDefinition() == typeof(List<>))
        {
            var elementType = typeof(T).GetGenericArguments()[0];
            var policyType = typeof(ListPoolPolicy<>).MakeGenericType(elementType);
            return (IPooledObjectPolicy<T>)Activator.CreateInstance(policyType)!;
        }

        return new ObjectPoolPolicy<T>();
    }
}
