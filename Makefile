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

# Update SQLx offline query cache
prepare-sqlx:
	@echo "Updating SQLx offline query cache..."
	@export TMP_DB=sqlite:/tmp/$$(date +'%Y%m%d-%H%M%S').db && \
	cd fts-sqlite && \
	sqlx database create -D $${TMP_DB} && \
	sqlx migrate run --source ./schema -D $${TMP_DB} && \
	if cargo sqlx prepare --check -D $${TMP_DB} 2>/dev/null; then \
		echo "ℹ️  SQLx cache is already up to date"; \
	else \
		cargo sqlx prepare -D $${TMP_DB} && \
		echo "✅ SQLx cache updated successfully"; \
	fi && \
	sqlx database drop -D $${TMP_DB} -y

clean:
	sqlx database drop

reset: clean init migrate