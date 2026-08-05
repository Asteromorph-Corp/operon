CREATE TABLE IF NOT EXISTS test_meta.runs (
    key TEXT PRIMARY KEY CHECK (key = 'global'),
    run_id UUID NOT NULL UNIQUE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    finished_at TIMESTAMP WITH TIME ZONE DEFAULT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    state TEXT NOT NULL DEFAULT 'running' CHECK (
        state IN ('running', 'stopped', 'completed', 'aborted')
    )
);

CREATE TABLE IF NOT EXISTS test_meta.run_executions (
    execution_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    run_id UUID REFERENCES test_meta.runs(run_id) ON DELETE CASCADE,
    started_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    ended_at TIMESTAMP WITH TIME ZONE,
    end_reason TEXT CHECK (
        end_reason IN ('stopped', 'completed', 'aborted')
    )
);
