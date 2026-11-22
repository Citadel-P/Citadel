SELECT EXISTS (
    SELECT 1
    FROM buildruns
    WHERE status IN ('Queued', 'Preparing', 'Running')
      AND CASE WHEN status = 'Queued' THEN
              nextattemptat IS NULL OR nextattemptat <= CURRENT_TIMESTAMP
              OR queuedat + INTERVAL '24 hours' <= CURRENT_TIMESTAMP
          ELSE
              startedat < $1
              AND startedat + make_interval(secs => timeoutseconds + 300) < CURRENT_TIMESTAMP
          END
)
