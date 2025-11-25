pg-env:
	printf "PGPASSWORD=" > pg-env
	openssl rand -hex 32 >> pg-env

clean:
	rm pg-env
