using Domain.Entities.Identity;
using Infrastructure.EntityFramework.Seed;
using Microsoft.EntityFrameworkCore;
using Microsoft.EntityFrameworkCore.Metadata.Builders;

namespace Infrastructure.EntityFramework.Configurations;

internal sealed class PermissionConfiguration : IEntityTypeConfiguration<Permission>
{
    public void Configure(EntityTypeBuilder<Permission> builder)
    {
        builder.ToTable("Permissions");

        // Pk & Indexes
        builder.HasKey(p => p.Id);

        // Properties
        builder.Property(p => p.PermissionCode).HasColumnName("PermissionCode");

        builder.HasData(DbSeed.Permissions);
    }
}