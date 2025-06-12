using Domain.Entities.Identity;
using Infrastructure.EntityFramework.Seed;
using Microsoft.EntityFrameworkCore;
using Microsoft.EntityFrameworkCore.Metadata.Builders;

namespace Infrastructure.EntityFramework.Configurations;

internal sealed class TeamConfiguration : IEntityTypeConfiguration<Team>
{
    public void Configure(EntityTypeBuilder<Team> builder)
    {
        builder.ToTable("Teams");

        // Pk & Indexes
        builder.HasKey(p => p.Id);
        builder.Property(p => p.Name).HasMaxLength(128).IsRequired();

        // A team can have only one role, and each role can be attached to many teams
        builder.HasOne(p => p.Role).WithMany();
        builder.HasMany(p => p.Users).WithMany(p => p.Teams).UsingEntity<UserTeam>();

        builder.HasData(DbSeed.Teams);
    }
}