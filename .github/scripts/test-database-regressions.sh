#!/usr/bin/env bash
# CI-only: owns a disposable PostgreSQL container and preserves fixture isolation.
set -euo pipefail
export CITADEL_CI_LOG_DIRECTORY="${RUNNER_TEMP:?RUNNER_TEMP must be set}/ci-tests"
mkdir -p "$CITADEL_CI_LOG_DIRECTORY"
sequence=0
run_cargo() {
  sequence=$((sequence + 1))
  bash .github/scripts/run-ci-check.sh "database-${BASHPID}-${sequence}" 600 cargo "$@"
}

container="citadel-ci-postgres-${GITHUB_RUN_ID:-local}-${BASHPID}"
owned=false
cleanup() {
  status=$?
  trap - EXIT
  if [[ "$owned" != true ]]; then exit "$status"; fi
  if (( status != 0 )); then
    # Fixture connection state only; never dump SQL text, environment or records.
    timeout --kill-after=5s 15s docker exec "$container" psql -h 127.0.0.1 -U citadel_test -d citadel_authorization \
      -c "SELECT datname, pid, state, wait_event_type, wait_event, now() - query_start AS query_age, pg_blocking_pids(pid) AS blockers FROM pg_stat_activity WHERE backend_type = 'client backend'" \
      > "$CITADEL_CI_LOG_DIRECTORY/database-waits.log" 2>&1 || true
  fi
  if timeout --kill-after=5s 30s docker rm -f -v "$container" >/dev/null; then
    :
  else
    cleanup_status=$?
    echo "::error::Could not remove owned fixture $container (exit $cleanup_status)."
    if (( status == 0 )); then status=$cleanup_status; fi
  fi
  exit "$status"
}
trap cleanup EXIT
docker create --name "$container" \
  --publish 127.0.0.1::5432 \
  --env POSTGRES_USER=citadel_test \
  --env POSTGRES_PASSWORD=citadel_test \
  --env POSTGRES_DB=citadel_authorization \
  postgres@sha256:898c60cb390d1085bc97bdd6a1ae968757421df394b071848b52b1c09c89d6a5 \
  -c shared_preload_libraries=pg_stat_statements
owned=true
timeout --kill-after=5s 60s docker start "$container"
for _ in {1..60}; do
  if docker exec "$container" pg_isready -h 127.0.0.1 -U citadel_test -d citadel_authorization; then
    break
  fi
  sleep 1
done
docker exec "$container" psql -h 127.0.0.1 -U citadel_test -d citadel_authorization \
  -v ON_ERROR_STOP=1 -c 'CREATE EXTENSION pg_stat_statements'
port=$(docker port "$container" 5432/tcp | cut -d: -f2)
export CITADEL_WORKLOAD_DATABASE_URL="postgres://citadel_test:citadel_test@127.0.0.1:$port/citadel_authorization"
export CITADEL_IDENTITY_DATABASE_URL="$CITADEL_WORKLOAD_DATABASE_URL"
export CITADEL_PLATFORM_DATABASE_URL="$CITADEL_WORKLOAD_DATABASE_URL"
export CITADEL_EXECUTION_DATABASE_URL="$CITADEL_WORKLOAD_DATABASE_URL"
export CITADEL_REQUIRE_QUERY_COUNTS=1
# The migration checksum test intentionally corrupts its journal.
docker exec "$container" createdb -h 127.0.0.1 -U citadel_test citadel_migrations
export CITADEL_TEST_DATABASE_URL="postgres://citadel_test:citadel_test@127.0.0.1:$port/citadel_migrations"
: "${CITADEL_IDENTITY_DATABASE_URL:?}" "${CITADEL_EXECUTION_DATABASE_URL:?}" "${CITADEL_TEST_DATABASE_URL:?}"
# Keep workspace feature resolution identical to the unit-test build.
run_cargo test --locked --workspace --test deployments_http \
  deployment_endpoints_enforce_auth_and_persist_the_crud_lifecycle -- --ignored --exact --nocapture
stack_test=stack_endpoints_enforce_auth_and_persist_apply_release_and_delete
run_cargo test --locked --workspace --test stacks_http "$stack_test" -- --ignored --exact --list | grep -Fx "$stack_test: test"
run_cargo test --locked --workspace --test stacks_http "$stack_test" -- --ignored --exact --test-threads=1
run_cargo test --locked --workspace --test identity_authorization_differential -- --ignored --test-threads=1
run_cargo test --locked --workspace --test deployment_apply_persistence -- --ignored --test-threads=1
run_cargo test --locked --workspace --test stack_persistence --test platform_inventory_persistence -- --ignored --test-threads=1
run_cargo test --locked --workspace --test platforms_http -- --ignored --test-threads=1
run_cargo test --locked --workspace --test edge_transport -- --ignored --test-threads=1
run_cargo test --locked --workspace --test backup_source_discovery -- --ignored --test-threads=1
run_cargo test --locked --workspace --test mfa_http --test oidc_http --test users_http --test teams_http --test roles_http -- --ignored --test-threads=1
# Copy catalog reads include all authorized workloads. Keep them separate
# from earlier inventory fixtures that intentionally seed partial rows.
docker exec "$container" createdb -h 127.0.0.1 -U citadel_test citadel_resource_copies
(
  export CITADEL_EXECUTION_DATABASE_URL="postgres://citadel_test:citadel_test@127.0.0.1:$port/citadel_resource_copies"
  run_cargo test --locked --workspace --test execution_resources_http resource_copies -- --ignored --test-threads=1
)
# These ignored fixtures exercise real completion transactions and fault injection.
for selection in \
  'build_execution build_runs_claim_once_persist_results_cancel_and_recover' \
  'backup_execution backup_claims_are_repository_exclusive_and_late_results_do_not_overwrite_recovery' \
  'alert_persistence durable_observation_recovers_missed_wakes_and_does_not_evaluate_twice_after_ack_failure'; do
  read -r target test_name <<< "$selection"
  # Recovery and receipt consumption inspect shared queues; each fixture
  # gets a fresh execution database, separate from authorization tests.
  docker exec "$container" createdb -h 127.0.0.1 -U citadel_test "citadel_$target"
  (
    export CITADEL_EXECUTION_DATABASE_URL="postgres://citadel_test:citadel_test@127.0.0.1:$port/citadel_$target"
    run_cargo test --locked --workspace --test "$target" "$test_name" -- --ignored --exact --list | grep -Fx "$test_name: test"
    run_cargo test --locked --workspace --test "$target" "$test_name" -- --ignored --exact --test-threads=1
  )
done
docker exec "$container" createdb -h 127.0.0.1 -U citadel_test citadel_alert_updates
(
  export CITADEL_EXECUTION_DATABASE_URL="postgres://citadel_test:citadel_test@127.0.0.1:$port/citadel_alert_updates"
  for test_name in \
    update_resolution::supersession::unchanged_update_rechecks_fence_delayed_targets_across_restart_without_extending_cooldown \
    update_resolution::supersession::update_targets_supersede_atomically_after_cooldown_and_preserve_acknowledgments \
    update_resolution::supersession::update_recheck_repairs_existing_open_targets_without_replacing_current_or_other_incidents \
    update_resolution::supersession::update_supersession_rolls_back_with_delivery_failure_and_retries_durably; do
    run_cargo test --locked --workspace --test alert_persistence "$test_name" -- --ignored --exact --list | grep -Fx "$test_name: test"
  done
  run_cargo test --locked --workspace --test alert_persistence update_resolution:: -- --ignored --test-threads=1
)
docker exec "$container" createdb -h 127.0.0.1 -U citadel_test citadel_update_test_availability
(
  export CITADEL_EXECUTION_DATABASE_URL="postgres://citadel_test:citadel_test@127.0.0.1:$port/citadel_update_test_availability"
  test_name=instance_updates::instance_availability_uses_seeded_community_rules_and_existing_outbox
  run_cargo test --locked -p citadel-adapters --test alert_persistence "$test_name" -- --ignored --exact --list | grep -Fx "$test_name: test"
  run_cargo test --locked -p citadel-adapters --test alert_persistence "$test_name" -- --ignored --exact --test-threads=1
)
# Inventory changes other fixture connectors: isolate these library tests,
# and keep execution serial. This does not replace platforms_http above.
docker exec "$container" createdb -h 127.0.0.1 -U citadel_test citadel_runtime_platforms
(
  export CITADEL_PLATFORM_DATABASE_URL="postgres://citadel_test:citadel_test@127.0.0.1:$port/citadel_runtime_platforms"
  for test_name in \
    workers::platforms::state_delta_tests::lifecycle_events_use_no_docker_reads_and_unknown_identity_stays_container_scoped \
    workers::platforms::state_delta_tests::local_and_direct_bursts_commit_once_and_publish_six_ids \
    workers::platforms::inventory_tests::daemon_bursts_coalesce_and_events_during_refresh_schedule_one_follow_up; do
    run_cargo test --locked --workspace --lib "$test_name" -- --ignored --exact --list | grep -Fx "$test_name: test"
    run_cargo test --locked --workspace --lib "$test_name" -- --ignored --exact --test-threads=1
  done
)
cancellation_test=persistence::postgres::identity::authorization_cache::tests::cancelled_commits_preserve_acl_fencing_and_session_notifications
run_cargo test --locked -p citadel-adapters --lib "$cancellation_test" -- --ignored --exact --list | grep -F "$cancellation_test: test"
run_cargo test --locked -p citadel-adapters --lib "$cancellation_test" -- --ignored --exact --test-threads=1
run_cargo test --locked -p citadel-database --test migrations -- --ignored --test-threads=1

# Statistics retries and history run on their own database, separate from retention.
docker exec "$container" createdb -h 127.0.0.1 -U citadel_test citadel_stats_history
(
  export CITADEL_STATISTICS_DATABASE_URL="postgres://citadel_test:citadel_test@127.0.0.1:$port/citadel_stats_history"
  run_cargo test --locked -p citadel-adapters --test stats_batch_persistence -- --ignored --test-threads=1
)

# Retention plans execute DELETE only against dedicated disposable fixtures.
docker exec "$container" createdb -h 127.0.0.1 -U citadel_test citadel_stats_retention
(
  export CITADEL_STATISTICS_DATABASE_URL="postgres://citadel_test:citadel_test@127.0.0.1:$port/citadel_stats_retention"
  run_cargo test --locked -p citadel-adapters --lib persistence::postgres::maintenance::tests:: -- --ignored --test-threads=1
)
docker exec "$container" createdb -h 127.0.0.1 -U citadel_test citadel_copy_expiry
(
  export CITADEL_STATISTICS_DATABASE_URL="postgres://citadel_test:citadel_test@127.0.0.1:$port/citadel_copy_expiry"
  run_cargo test --locked -p citadel-adapters --test maintenance_retention -- --ignored --test-threads=1
)
