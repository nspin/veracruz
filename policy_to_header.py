#!/usr/bin/env python3

# TODO should these all have prefixes?

import json
import hashlib
import base64

# TODO fully populate this?
CIPHERSUITES = {
    'TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256': 'MBEDTLS_TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256',
}

def pem_to_der(pem):
    lines = pem.strip().split('\n')
    lines = ''.join(line.strip() for line in lines[1:-1])
    return base64.b64decode(lines)

def main(args):
    print('loading policy %s' % args.policy)
    with open(args.policy) as f:
        policy_raw = f.read()
        policy_hash = hashlib.sha256(policy_raw.encode('utf-8')).hexdigest()
        policy = json.loads(policy_raw)

    if args.identity:
        print('loading identity %s' % args.identity)
        with open(args.identity) as f:
            identity_pem = f.read()
            identity_der = pem_to_der(identity_pem)
            identity_hash = hashlib.sha256(identity_der).hexdigest()

        # sanity check that identity is in policy
        assert any(
            identity['certificate'].replace('\n', '')
                == identity_pem.replace('\n', '')
            for identity in policy['identities'])

    if args.key:
        print('loading key %s' % args.key)
        with open(args.key) as f:
            key_pem = f.read()
            key_der = pem_to_der(key_pem)

    if args.header:
        print('generating %s for policy %s' % (args.header, policy_hash))
        with open(args.header, 'w') as f:
            _write = f.write
            def write(s='', **args):
                _write(s % args)
            def writeln(s='', **args):
                _write(s % args)
                _write('\n')
            f.write = write
            f.writeln = writeln

            f.writeln('//// AUTOGENERATED ////')
            f.writeln('#ifndef VERACRUZ_POLICY_H')
            f.writeln('#define VERACRUZ_POLICY_H')
            f.writeln()
            f.writeln('#include <stdint.h>')
            f.writeln('#include <mbedtls/ssl_ciphersuites.h>')
            f.writeln()
            f.writeln('// general policy things')
            f.writeln('#define VERACRUZ_POLICY_HASH "%(hash)s"',
                hash=policy_hash)
    #        f.writeln('extern const uint8_t _VERACRUZ_POLICY_RAW[];')
    #        f.writeln('#define VERACRUZ_POLICY_RAW _VERACRUZ_POLICY_RAW')
            f.writeln()
            f.writeln('// various hashes')
            # TODO choose between platforms?
            f.writeln('extern const uint8_t _RUNTIME_MANAGER_HASH[%(size)d];',
                size=len(policy['mexico_city_hash_sgx'])/2)
            f.writeln('#define RUNTIME_MANAGER_HASH _RUNTIME_MANAGER_HASH')
            f.writeln()
            f.writeln('// server info')
            f.writeln('#define VERACRUZ_SERVER_HOST "%(host)s"',
                host=policy['sinaloa_url'].split(':')[0])
            f.writeln('#define VERACRUZ_SERVER_PORT %(port)s',
                port=policy['sinaloa_url'].split(':')[1])
            f.writeln('#define PROXY_ATTESTATION_SERVER_HOST "%(host)s"',
                host=policy['proxy_attestation_server_url'].split(':')[0])
            f.writeln('#define PROXY_ATTESTATION_SERVER_PORT %(port)s',
                port=policy['proxy_attestation_server_url'].split(':')[1])
            f.writeln()
            f.writeln('// ciphersuite requested by the policy, as both a constant')
            f.writeln('// and mbedtls-friendly null-terminated array')
            f.writeln('#define CIPHERSUITE %(ciphersuite)s',
                ciphersuite=CIPHERSUITES[policy['ciphersuite']])
            f.writeln('extern const int _CIPHERSUITES[2];')
            f.writeln('#define CIPHERSUITES _CIPHERSUITES')
            f.writeln()
            f.writeln('// client cert/key')
            if args.identity:
                f.writeln('extern const uint8_t _CLIENT_CERT_DER[%(len)d];',
                    len=len(identity_der))
                f.writeln('#define CLIENT_CERT_DER _CLIENT_CERT_DER')
                # TODO these should be raw bytes
                f.writeln('#define CLIENT_CERT_HASH "%(hash)s"',
                    hash=identity_hash)
            if args.key:
                f.writeln('extern const uint8_t _CLIENT_KEY_DER[%(len)d];',
                    len=len(key_der))
                f.writeln('#define CLIENT_KEY_DER _CLIENT_KEY_DER')
            f.writeln()
            f.writeln('#endif')

    if args.source:
        print('generating %s for policy %s' % (args.source, policy_hash))
        with open(args.source, 'w') as f:
            _write = f.write
            def write(s='', **args):
                _write(s % args)
            def writeln(s='', **args):
                _write(s % args)
                _write('\n')
            f.write = write
            f.writeln = writeln

            f.writeln('//// AUTOGENERATED ////')
            f.writeln()
            f.writeln('#include <stdint.h>')
            f.writeln('#include <mbedtls/ssl_ciphersuites.h>')
            f.writeln()
    #        f.writeln('const uint8_t _VERACRUZ_POLICY_RAW[] = {')
    #        for i in range(0, len(policy_raw), 8):
    #            f.writeln('    %(raw)s',
    #                raw=' '.join('0x%02x,' % ord(x)
    #                    for x in policy_raw[i : min(i+8, len(policy_raw))]))
    #        f.writeln('};')
    #        f.writeln()
            f.writeln('const uint8_t _RUNTIME_MANAGER_HASH[32] = {')
            runtime_manager_hash = policy['mexico_city_hash_sgx']
            for i in range(0, len(runtime_manager_hash)//2, 8):
                f.writeln('    %(hash)s',
                    hash=' '.join('0x%02x,' % int(runtime_manager_hash[2*j:2*j+2], 16)
                        for j in range(i, min(i+8, len(runtime_manager_hash)//2))))
            f.writeln('};')
            f.writeln()
            f.writeln('const int _CIPHERSUITES[2] = {')
            f.writeln('    %(ciphersuite)s,',
                ciphersuite=CIPHERSUITES[policy['ciphersuite']])
            f.writeln('    0,')
            f.writeln('};')
            f.writeln()
            if args.identity:
                f.writeln('const uint8_t _CLIENT_CERT_DER[%(len)d] = {',
                    len=len(identity_der))
                for i in range(0, len(identity_der), 8):
                    f.writeln('    %(der)s',
                        der=' '.join('0x%02x,' % identity_der[j]
                            for j in range(i, min(i+8, len(identity_der)))))
                f.writeln('};')
            if args.key:
                f.writeln('const uint8_t _CLIENT_KEY_DER[%(len)d] = {',
                    len=len(key_der))
                for i in range(0, len(key_der), 8):
                    f.writeln('    %(der)s',
                        der=' '.join('0x%02x,' % key_der[j]
                            for j in range(i, min(i+8, len(key_der)))))
                f.writeln('};')

if __name__ == "__main__":
    import sys
    import argparse
    parser = argparse.ArgumentParser(
        description='Generate header file from Veracruz policy')
    parser.add_argument('policy',
        help='Veracruz policy file (.json)')
    parser.add_argument('--header',
        help='Output header file (.h)')
    parser.add_argument('--source',
        help='Output source file (.c)')
    parser.add_argument('--identity',
        help='Identity of client (.pem)')
    parser.add_argument('--key',
        help='Private key of client (.pem)')
    args = parser.parse_args()
    main(args)
