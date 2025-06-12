using Domain.Entities.Identity;
using Infrastructure.EntityFramework.Seed;
using Microsoft.EntityFrameworkCore;
using Microsoft.EntityFrameworkCore.Metadata.Builders;

namespace Infrastructure.EntityFramework.Configurations;

internal sealed class RoleConfiguration : IEntityTypeConfiguration<Role>
{
    private static readonly DateTime dateTime = new(2000, 1, 1, 0, 0, 0, DateTimeKind.Utc);
    public void Configure(EntityTypeBuilder<Role> builder)
    {
        builder.ToTable("Roles");

        // Pk & Indexes
        builder.HasKey(p => p.Id);

        // Each Role can have many associated permissions
        builder.HasMany(p => p.Permissions).WithOne().HasForeignKey("RoleId").IsRequired().OnDelete(DeleteBehavior.Cascade);

        // Properties
        builder.Property(p => p.Name).HasColumnName("Name").HasMaxLength(128).IsRequired();
        builder.Property(p => p.CreatedAt).HasColumnName("CreatedAt").HasDefaultValue(dateTime);
        builder.Property(p => p.UpdatedAt).HasColumnName("UpdatedAt").HasDefaultValue(dateTime);

        builder.HasData(DbSeed.Roles);
    }
}