#!/usr/bin/env bash 
set -x
set -eo pipefail

DB_USER=${POSTGRES_USER:=postgres}
DB_PASSWORD=${POSTGRES_USER:=fakepassword}
DB_NAME=${POSTGRES_USER:=podcasts}
DB_PORT=${POSTGRES_USER:=5432}

export PGPASSWORD="${DB_PASSWORD}"
until psql -h "localhost" -U "${DB_USER}" -d "postgres" -c '\q'; do
  >&2 echo "Postgres not available, retrying."
  sleep 2
done

>&2 echo "Postgres connected on port ${DB_PORT}"
export DATABASE_URL=postgres://${DB_USER}:${DB_PASSWORD}@127.0.0.1:${DB_PORT}/${DB_NAME}
sqlx migrate add create entretien_2000_table
