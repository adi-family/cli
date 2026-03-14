-- Remove is_active column (credentials are either present or deleted)

-- Drop the partial index first
DROP INDEX IF EXISTS idx_credentials_active;

-- Remove the column
ALTER TABLE credentials DROP COLUMN IF EXISTS is_active;
