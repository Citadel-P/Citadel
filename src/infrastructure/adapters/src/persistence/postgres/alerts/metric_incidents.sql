WITH candidates AS MATERIALIZED (
    SELECT * FROM jsonb_to_recordset($1::jsonb) AS p(id uuid,ruleid uuid,resourceid uuid,
        name text,kind text,severity text,info json,key text,observed timestamptz)
), inserted AS (
    INSERT INTO alertevents(id,alertruleid,type,severity,info,resourceid,resourcename,resourcetype,deduplicationkey,openincidentkey)
    SELECT id,ruleid,kind,severity,info,resourceid,name,'Platform',key,key FROM candidates
    ON CONFLICT(openincidentkey) WHERE openincidentkey IS NOT NULL DO NOTHING
    RETURNING id,alertruleid
), triggered AS (
    UPDATE alertrulestates s SET consecutivematches=0,lasttriggeredat=p.observed
    FROM inserted i JOIN candidates p USING(id)
    WHERE s.alertruleid=p.ruleid AND s.resourceid=p.resourceid
), delivery AS (
    INSERT INTO alertdeliveryoutbox(id,alerteventid,alertchannelid)
    SELECT gen_random_uuid(),i.id,c.id FROM inserted i
    JOIN alertrulechannels r ON r.alertruleid=i.alertruleid
    JOIN alertchannels c ON c.id=r.alertchannelid AND c.isactive
    ON CONFLICT(alerteventid,alertchannelid) DO NOTHING RETURNING id
)
SELECT (SELECT count(*) FROM inserted),(SELECT count(*) FROM delivery)
