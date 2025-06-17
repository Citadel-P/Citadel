using Domain.Contracts.Interfaces;
using Infrastructure.EntityFramework;
using Microsoft.EntityFrameworkCore;

namespace Infrastructure.Repositories;

/// <inheritdoc />
internal class Repository<TEntity>(ApplicationDbContext applicationDbContext) : IRepository<TEntity> where TEntity : class
{
    private readonly DbSet<TEntity> dbSet = applicationDbContext.Set<TEntity>();

    public void Add(TEntity entity) => dbSet.Add(entity);
    public void Remove(TEntity entity) => dbSet.Remove(entity);
    public void RemoveRange(IEnumerable<TEntity> entities) => dbSet.RemoveRange(entities);

    public IQueryable<TEntity> Query() => dbSet;
}