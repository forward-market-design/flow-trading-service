# Install any CLI tools needed for database management
setup:
	cargo install sqlx-cli

# Create a database specified by `DATABASE_URL` (ENV or .env)
init:
	if [ ! -f .env ]; then cp .env.example .env; fi
	echo "\n=== NOTE: Local compilation will now depend on the database being created. Check the .env file for more details. ===\n"
	sqlx database create

# Run migrations against the database
migrate:
	sqlx migrate run --source ./fts-sqlite/schema

clean:
	sqlx database drop

reset: clean init migrate