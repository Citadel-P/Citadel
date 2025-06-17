namespace Domain.Contracts.Interfaces;

/// <summary>
/// repository interface for basic CRUD operations and query building
/// </summary>
public interface IRepository<TEntity> where TEntity : class
{
    void Add(TEntity entity);
    void Remove(TEntity entity);
    void RemoveRange(IEnumerable<TEntity> entities);

    IQueryable<TEntity> Query();
}


