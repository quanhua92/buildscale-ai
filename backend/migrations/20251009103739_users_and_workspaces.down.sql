-- Drop the junction table first as it has foreign keys to all other tables.
DROP TABLE IF EXISTS workspace_members;

-- Drop the roles table.
DROP TABLE IF EXISTS roles;

-- Drop the workspaces table, which depends on the users table.
DROP TABLE IF EXISTS workspaces;

-- Finally, drop the users table.
DROP TABLE IF EXISTS users;
