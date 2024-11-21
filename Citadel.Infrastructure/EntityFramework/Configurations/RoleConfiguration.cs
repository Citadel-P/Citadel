using Infrastructure.Entities.Identity;
using Infrastructure.EntityFramework.Seed;
using Microsoft.EntityFrameworkCore;
using Microsoft.EntityFrameworkCore.Metadata.Builders;

namespace Infrastructure.EntityFramework.Configurations;

internal sealed class RoleConfiguration : IEntityTypeConfiguration<Role>
{
    public void Configure(EntityTypeBuilder<Role> builder)
    {
        builder.ToTable("Roles");

        // Pk & Indexes
        builder.HasKey(p => p.Id);

        // Each Role can have many associated permissions
        builder.HasMany(p => p.Permissions).WithOne().HasForeignKey("RoleId").IsRequired().OnDelete(DeleteBehavior.Cascade);

        // Properties
        builder.Property(p => p.Name).HasColumnName("Name").HasMaxLength(128).IsRequired();
        builder.Property(p => p.CreatedAt).HasColumnName("CreatedAt").HasDefaultValue(DateTime.UtcNow);
        builder.Property(p => p.UpdatedAt).HasColumnName("UpdatedAt").HasDefaultValue(DateTime.UtcNow);

        builder.HasData(DbSeed.Roles);
    }
}