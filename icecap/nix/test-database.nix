{ runCommand, sqlite, diesel-cli }:

runCommand "proxy-attestation-server.db" {
  nativeBuildInputs = [ sqlite diesel-cli ];
} ''
  diesel --config-file ${../../proxy-attestation-server/diesel.toml} setup
  mv proxy-attestation-server.db $out
''
