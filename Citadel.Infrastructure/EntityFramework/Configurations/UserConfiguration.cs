using Infrastructure.Entities.Identity;
using Infrastructure.EntityFramework.Seed;
using Microsoft.EntityFrameworkCore;
using Microsoft.EntityFrameworkCore.Metadata.Builders;

namespace Infrastructure.EntityFramework.Configurations;

internal sealed class UserConfiguration : IEntityTypeConfiguration<User>
{
    private static readonly DateTime dateTime = new(2000, 1, 1, 0, 0, 0, DateTimeKind.Utc);
    public void Configure(EntityTypeBuilder<User> builder)
    {
        builder.ToTable("Users");

        // Pk & Indexes
        builder.HasKey(p => p.Id);
        builder.HasIndex(p => p.Email).HasDatabaseName("EmailIndex").IsUnique();

        // A user can join many teams, and each team can host many users
        builder.HasMany(p => p.Teams).WithMany(p => p.Users).UsingEntity<UserTeam>();

        // Properties
        builder.Property(p => p.Id).HasColumnName("Id");
        builder.Property(p => p.Name).HasColumnName("Name").HasMaxLength(128);
        builder.Property(p => p.Email).HasColumnName("Email").HasMaxLength(128).IsRequired();
        builder.Property(p => p.Password).HasColumnName("Password").HasMaxLength(128).IsRequired();
        builder.Property(p => p.CreatedAt).HasColumnName("CreatedAt").HasDefaultValue(dateTime).IsRequired();
        builder.Property(p => p.UpdatedAt).HasColumnName("UpdatedAt").HasDefaultValue(dateTime).IsRequired();

        builder.HasData(DbSeed.Users);
    }
}