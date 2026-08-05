CREATE TABLE IF NOT EXISTS test_data._footprint (
    key TEXT PRIMARY KEY DEFAULT 'global' CHECK (key = 'global'),
    run_id UUID NOT NULL UNIQUE,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    state TEXT NOT NULL DEFAULT 'running' CHECK (
        state IN ('running', 'stopped', 'completed', 'aborted')
    )
);
