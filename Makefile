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
	export TMP_DB=sqlite:/tmp/$$(date +'%Y%m%d-%H%M%S').db && \
	cd fts-sqlite && \
	sqlx database create -D $${TMP_DB} && \
	sqlx migrate run --source ./schema -D $${TMP_DB} && \
	cargo sqlx prepare -D $${TMP_DB} && \
	sqlx database drop -D $${TMP_DB} -y

clean:
	sqlx database drop

reset: clean init migrate