using Domain.Contracts.Interfaces;
using Google.Api;
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
    public async Task<int> BulkInsertAsync(IEnumerable<TEntity> entities, CancellationToken cancellationToken)
    {
        var count = 0;
        const int batchSize = 1000;

        await using var transaction = await applicationDbContext.Database.BeginTransactionAsync(cancellationToken);

        try
        {
            applicationDbContext.ChangeTracker.AutoDetectChangesEnabled = false;

            foreach (var batch in entities.Chunk(batchSize))
            {
                await applicationDbContext.AddRangeAsync(batch, cancellationToken);
                count += await applicationDbContext.SaveChangesAsync(cancellationToken);
                applicationDbContext.ChangeTracker.Clear();
            }

            await transaction.CommitAsync(cancellationToken);
            return count;
        }
        finally
        {
            applicationDbContext.ChangeTracker.AutoDetectChangesEnabled = true;
        }
    }

    public IQueryable<TEntity> Query() => dbSet;

}