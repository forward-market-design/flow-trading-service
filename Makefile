# Install any CLI tools needed for database management
setup:
	cargo install sqlx-cli

# Create a database specified by `DATABASE_URL` (ENV or .env)
init:
	sqlx database create

# Run migrations against the database
migrate:
	sqlx migrate run --source ./fts-sqlite/schema

clean:
	sqlx database drop

reset: clean init migrate