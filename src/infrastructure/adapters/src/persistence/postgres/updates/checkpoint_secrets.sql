-- Every AES-GCM envelope currently written by Core. Missing/malformed required
-- envelopes must reach the validator; never filter them out as if absent.
WITH envelopes AS (
    SELECT 1 AS kind, secretid AS id, encryptedvalue AS envelope FROM public.internalsecretvalues
    UNION ALL
    SELECT 2, id, COALESCE(configuration::jsonb->>'ProtectedToken', configuration::jsonb->>'protectedToken') FROM public.secretproviders
    UNION ALL
    SELECT 3, id, configuration->>'$protected' FROM public.gitaccounts
    UNION ALL
    SELECT 4, id, clientsecretciphertext FROM public.oidcproviders WHERE clientsecretciphertext IS NOT NULL
    UNION ALL
    SELECT 5, id, protectedtotpsecret FROM public.mfasetupsessions
    UNION ALL
    SELECT 6, userid, protectedtotpsecret FROM public.usermfasettings
)
SELECT COALESCE(json_agg(page), '[]'::json) FROM (
    SELECT kind, id, envelope FROM envelopes
    WHERE (kind, id) > (__KIND__, '__ID__'::uuid)
    ORDER BY kind, id LIMIT __PAGE_SIZE__
) page;
