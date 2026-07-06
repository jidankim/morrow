ALTER TABLE provider_route_outcomes
ADD COLUMN profile_id TEXT NOT NULL DEFAULT 'default';

ALTER TABLE provider_route_outcomes
ADD COLUMN profile_version TEXT NOT NULL DEFAULT 'pre-list-reminders';

ALTER TABLE provider_route_outcomes
ADD COLUMN profile_schema_version TEXT NOT NULL DEFAULT 'none';

ALTER TABLE provider_route_outcomes
ADD COLUMN profile_policy_version TEXT NOT NULL DEFAULT 'none';
