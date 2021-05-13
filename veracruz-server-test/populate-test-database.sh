# TODO_BEFORE_MERGE

hash_value=0000000000000000000000000000000000000000000000000000000000000000
echo $hash_value
rm -f proxy-attestation-server.db
diesel --config-file ../proxy-attestation-server/diesel.toml setup
echo "INSERT INTO firmware_versions VALUES(1, 'sgx', '0.3.0', '${hash_value}');" > tmp.sql
echo "INSERT INTO firmware_versions VALUES(2, 'psa', '0.3.0', '0000000000000000000000000000000000000000000000000000000000000000');" >> tmp.sql
pcr0=0000000000000000000000000000000000000000000000000000000000000000
echo "INSERT INTO firmware_versions VALUES(3, 'nitro', '0.1.0', '${pcr0}');" >> tmp.sql
sqlite3 proxy-attestation-server.db < tmp.sql

